use crate::dto::{FieldDefDto, SchemaDto};
use crate::error::AppError;
use crate::schema::formula;
use serde_json::{Map, Value};
use std::collections::HashSet;

pub const FIELD_TYPES: &[&str] = &[
    "text", "longtext", "number", "scale", "status", "date", "checkbox", "tags", "formula",
];

pub fn valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    if key.chars().count() > 40 {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

pub fn validate_schema(schema: &SchemaDto) -> Result<(), AppError> {
    if schema.format_version != 1 {
        return Err(AppError::unsupported("Unsupported schema format version"));
    }
    let name = schema.name.trim();
    if name.is_empty() {
        return Err(AppError::field("name", "Schema name must not be empty"));
    }
    if name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Schema name must not exceed 60 characters",
        ));
    }
    if !valid_key(&schema.key) {
        return Err(AppError::field(
            "key",
            "Schema key must match ^[a-z][a-z0-9_]{0,39}$",
        ));
    }

    let mut keys = HashSet::new();
    let mut tags_fields = 0usize;
    let mut formulas: Vec<(String, String)> = Vec::new();
    let mut list_columns = 0usize;

    for field in &schema.fields {
        if !valid_key(&field.key) {
            return Err(AppError::field(
                &field.key,
                "Field key must match ^[a-z][a-z0-9_]{0,39}$",
            ));
        }
        if !keys.insert(field.key.clone()) {
            return Err(AppError::field(&field.key, "Duplicate field key"));
        }
        if field.label.trim().is_empty() {
            return Err(AppError::field(&field.key, "Field label must not be empty"));
        }
        if field.label.chars().count() > 80 {
            return Err(AppError::field(
                &field.key,
                "Field label must not exceed 80 characters",
            ));
        }
        if !FIELD_TYPES.contains(&field.field_type.as_str()) {
            return Err(AppError::field(&field.key, "Unknown field type"));
        }
        if field.field_type == "tags" {
            tags_fields += 1;
            if field.key != "tags" {
                return Err(AppError::field(
                    &field.key,
                    "The tags field key must be 'tags'",
                ));
            }
        }
        if field.field_type == "formula" {
            if field.required {
                return Err(AppError::field(
                    &field.key,
                    "Formula fields cannot be required",
                ));
            }
            let expression = field.expression.as_deref().unwrap_or("").trim();
            if expression.is_empty() {
                return Err(AppError::field(&field.key, "Formula expression is empty"));
            }
            formula::parse(expression)
                .map_err(|error| AppError::field(&field.key, error.message))?;
            formulas.push((field.key.clone(), expression.to_string()));
        }
        if field.field_type == "status" {
            let options = field
                .config
                .get("options")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if !(2..=12).contains(&options.len()) {
                return Err(AppError::field(
                    &field.key,
                    "A status field needs 2 to 12 options",
                ));
            }
            for option in &options {
                let key = option.get("key").and_then(Value::as_str).unwrap_or("");
                if !valid_key(key) || key.chars().count() > 32 {
                    return Err(AppError::field(
                        &field.key,
                        "Status option keys must match ^[a-z][a-z0-9_]{0,31}$",
                    ));
                }
            }
        }
        if field.show_in_list {
            list_columns += 1;
        }
    }

    if tags_fields > 1 {
        return Err(AppError::validation(
            "Only one tags field is allowed per schema",
        ));
    }
    if list_columns > 6 {
        return Err(AppError::validation(
            "At most six fields may be shown in the list",
        ));
    }

    let known: HashSet<String> = schema
        .fields
        .iter()
        .map(|field| field.key.clone())
        .collect();
    for (key, expression) in &formulas {
        for identifier in formula::identifiers(&formula::parse(expression)?) {
            if !known.contains(&identifier) {
                return Err(AppError::field(
                    key,
                    format!("Unknown field '{identifier}'"),
                ));
            }
        }
    }
    formula::find_cycle(&formulas)?;

    for role in [
        ("title", schema.title_field_id.as_deref(), &["text"][..]),
        (
            "subtitle",
            schema.subtitle_field_id.as_deref(),
            &["text", "status", "date", "number", "scale"][..],
        ),
        (
            "badge",
            schema.badge_field_id.as_deref(),
            &["status", "formula", "checkbox"][..],
        ),
    ] {
        if let Some(field_id) = role.1 {
            match schema.fields.iter().find(|field| field.id == field_id) {
                Some(field) if role.2.contains(&field.field_type.as_str()) => {}
                Some(_) => {
                    return Err(AppError::validation(format!(
                        "The {} role is not allowed for this field type",
                        role.0
                    )))
                }
                None => {
                    return Err(AppError::validation(format!(
                        "The {} role references an unknown field",
                        role.0
                    )))
                }
            }
        }
    }

    Ok(())
}

pub fn validate_values(schema: &SchemaDto, values: &Map<String, Value>) -> Result<(), AppError> {
    let known: HashSet<&str> = schema
        .fields
        .iter()
        .filter(|field| field.field_type != "tags" && field.field_type != "formula")
        .map(|field| field.key.as_str())
        .collect();
    for key in values.keys() {
        if !known.contains(key.as_str()) {
            return Err(AppError::field(key, "Unknown field key"));
        }
    }
    for field in &schema.fields {
        if field.field_type == "tags" || field.field_type == "formula" {
            continue;
        }
        validate_field(field, values.get(&field.key))?;
    }
    Ok(())
}

pub fn validate_field(field: &FieldDefDto, value: Option<&Value>) -> Result<(), AppError> {
    let empty = match value {
        None | Some(Value::Null) => true,
        Some(Value::String(text)) => text.trim().is_empty(),
        Some(Value::Array(items)) => items.is_empty(),
        _ => false,
    };
    if field.required && empty {
        return Err(AppError::field(&field.key, "This field is required"));
    }
    if empty {
        return Ok(());
    }
    let value = match value {
        Some(value) => value,
        None => return Ok(()),
    };

    match field.field_type.as_str() {
        "text" => {
            let text = value
                .as_str()
                .ok_or_else(|| AppError::field(&field.key, "Expected text"))?
                .trim();
            let max = field
                .config
                .get("maxLength")
                .and_then(Value::as_u64)
                .unwrap_or(500) as usize;
            if text.chars().count() > max {
                return Err(AppError::field(
                    &field.key,
                    format!("Text must not exceed {max} characters"),
                ));
            }
        }
        "longtext" => {
            let text = value
                .as_str()
                .ok_or_else(|| AppError::field(&field.key, "Expected text"))?;
            let max = field
                .config
                .get("maxLength")
                .and_then(Value::as_u64)
                .unwrap_or(20_000) as usize;
            if text.chars().count() > max {
                return Err(AppError::field(
                    &field.key,
                    format!("Text must not exceed {max} characters"),
                ));
            }
        }
        "number" => {
            let number = value
                .as_f64()
                .ok_or_else(|| AppError::field(&field.key, "Expected a number"))?;
            if !number.is_finite() {
                return Err(AppError::field(&field.key, "Number must be finite"));
            }
            if let Some(min) = field.config.get("min").and_then(Value::as_f64) {
                if number < min {
                    return Err(AppError::field(
                        &field.key,
                        format!("Value must be at least {min}"),
                    ));
                }
            }
            if let Some(max) = field.config.get("max").and_then(Value::as_f64) {
                if number > max {
                    return Err(AppError::field(
                        &field.key,
                        format!("Value must be at most {max}"),
                    ));
                }
            }
            if let Some(step) = field.config.get("step").and_then(Value::as_f64) {
                if step > 0.0 {
                    let base = field
                        .config
                        .get("min")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let steps = (number - base) / step;
                    if (steps - steps.round()).abs() > 1e-9 {
                        return Err(AppError::field(
                            &field.key,
                            format!("Value must align to a step of {step}"),
                        ));
                    }
                }
            }
        }
        "scale" => {
            let number = value
                .as_f64()
                .ok_or_else(|| AppError::field(&field.key, "Expected a number"))?;
            if (number - number.round()).abs() > f64::EPSILON {
                return Err(AppError::field(&field.key, "Scale must be an integer"));
            }
            let min = field
                .config
                .get("min")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let max = field
                .config
                .get("max")
                .and_then(Value::as_f64)
                .unwrap_or(6.0);
            if number < min || number > max {
                return Err(AppError::field(
                    &field.key,
                    format!("Scale must be between {min} and {max}"),
                ));
            }
        }
        "status" => {
            let key = value
                .as_str()
                .ok_or_else(|| AppError::field(&field.key, "Expected a status key"))?;
            let options = field
                .config
                .get("options")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let found = options
                .iter()
                .any(|option| option.get("key").and_then(Value::as_str) == Some(key));
            if !found {
                return Err(AppError::field(&field.key, "Unknown status option"));
            }
        }
        "date" => {
            let text = value
                .as_str()
                .ok_or_else(|| AppError::field(&field.key, "Expected a date string"))?;
            let with_time = field
                .config
                .get("withTime")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let valid = if with_time {
                chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M").is_ok()
                    || chrono::DateTime::parse_from_rfc3339(text).is_ok()
            } else {
                chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
            };
            if !valid {
                return Err(AppError::field(
                    &field.key,
                    if with_time {
                        "Date must use YYYY-MM-DDTHH:mm"
                    } else {
                        "Date must use YYYY-MM-DD"
                    },
                ));
            }
        }
        "checkbox" if value.as_bool().is_none() => {
            return Err(AppError::field(&field.key, "Expected true or false"));
        }
        "checkbox" => {}
        _ => {}
    }
    Ok(())
}

pub fn apply_defaults(schema: &SchemaDto, values: &mut Map<String, Value>) {
    for field in &schema.fields {
        if field.field_type == "formula" || field.field_type == "tags" {
            continue;
        }
        if values.contains_key(&field.key) {
            continue;
        }
        if let Some(default) = field.config.get("default") {
            if !default.is_null() {
                values.insert(field.key.clone(), default.clone());
            }
        }
    }
}

pub struct StorageValue {
    pub text: Option<String>,
    pub number: Option<f64>,
    pub date: Option<String>,
    pub boolean: Option<i64>,
}

pub fn storage_value(field: &FieldDefDto, value: &Value) -> StorageValue {
    let empty = StorageValue {
        text: None,
        number: None,
        date: None,
        boolean: None,
    };
    if value.is_null() {
        return empty;
    }
    match field.field_type.as_str() {
        "text" | "longtext" => match value.as_str() {
            Some(text) if !text.trim().is_empty() => StorageValue {
                text: Some(text.trim().to_string()),
                number: None,
                date: None,
                boolean: None,
            },
            _ => empty,
        },
        "status" => match value.as_str() {
            Some(text) if !text.is_empty() => StorageValue {
                text: Some(text.to_string()),
                number: None,
                date: None,
                boolean: None,
            },
            _ => empty,
        },
        "number" | "scale" => match value.as_f64() {
            Some(number) => StorageValue {
                text: None,
                number: Some(number),
                date: None,
                boolean: None,
            },
            None => empty,
        },
        "date" => match value.as_str() {
            Some(text) if !text.is_empty() => StorageValue {
                text: None,
                number: None,
                date: Some(text.to_string()),
                boolean: None,
            },
            _ => empty,
        },
        "checkbox" => match value.as_bool() {
            Some(flag) => StorageValue {
                text: None,
                number: None,
                date: None,
                boolean: Some(if flag { 1 } else { 0 }),
            },
            None => empty,
        },
        _ => empty,
    }
}

pub fn value_from_storage(
    field: &FieldDefDto,
    text: Option<String>,
    number: Option<f64>,
    date: Option<String>,
    boolean: Option<i64>,
) -> Value {
    match field.field_type.as_str() {
        "text" | "longtext" | "status" => text.map(Value::String).unwrap_or(Value::Null),
        "number" => number
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        "scale" => number
            .map(|value| Value::from(value as i64))
            .unwrap_or(Value::Null),
        "date" => date.map(Value::String).unwrap_or(Value::Null),
        "checkbox" => boolean
            .map(|flag| Value::Bool(flag != 0))
            .unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
