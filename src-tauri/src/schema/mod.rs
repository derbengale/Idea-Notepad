pub mod formula;
pub mod validate;

use crate::db::Db;
use crate::dto::{FieldDefDto, SchemaDto};
use crate::error::AppError;
use crate::util::new_id;
use parking_lot::RwLock;
use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct SchemaCache {
    inner: RwLock<HashMap<String, Arc<SchemaDto>>>,
}

impl SchemaCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reload(&self, db: &Db) -> Result<(), AppError> {
        let schemas = db.with_read(crate::db::repo_schema::list)?;
        let mut map = HashMap::with_capacity(schemas.len());
        for schema in schemas {
            map.insert(schema.id.clone(), Arc::new(schema));
        }
        *self.inner.write() = map;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<Arc<SchemaDto>> {
        self.inner.read().get(id).cloned()
    }

    pub fn get_by_key(&self, key: &str) -> Option<Arc<SchemaDto>> {
        self.inner
            .read()
            .values()
            .find(|schema| schema.key == key)
            .cloned()
    }

    pub fn list(&self) -> Vec<SchemaDto> {
        let mut schemas: Vec<SchemaDto> = self
            .inner
            .read()
            .values()
            .map(|schema| (**schema).clone())
            .collect();
        schemas.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.name.cmp(&b.name)));
        schemas
    }

    pub fn require(&self, id: &str) -> Result<Arc<SchemaDto>, AppError> {
        self.get(id)
            .ok_or_else(|| AppError::not_found("Schema not found"))
    }
}

pub fn ensure_seeded(conn: &Connection, language: &str) -> Result<(), AppError> {
    let workspace_id = crate::db::repo_meta::get(conn, "workspace.id")?;
    if workspace_id.is_none() {
        crate::db::repo_meta::set(conn, "workspace.id", &new_id())?;
        crate::db::repo_meta::set(conn, "workspace.name", "Better Notepad")?;
    }
    let seeded = crate::db::repo_meta::get(conn, "workspace.seeded")?;
    if seeded.is_some() {
        return Ok(());
    }
    let german = language == "de";
    for schema in seed_schemas(german) {
        crate::db::repo_schema::insert(conn, &schema)?;
    }
    crate::db::repo_meta::set(conn, "workspace.seeded", "1")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn field(
    key: &str,
    label: &str,
    field_type: &str,
    required: bool,
    order: i64,
    config: Value,
    expression: Option<&str>,
    show_in_list: bool,
) -> FieldDefDto {
    FieldDefDto {
        id: new_id(),
        key: key.to_string(),
        label: label.to_string(),
        field_type: field_type.to_string(),
        required,
        order,
        config,
        expression: expression.map(|value| value.to_string()),
        show_in_list,
    }
}

fn status_field(
    key: &str,
    label: &str,
    order: i64,
    options: &[(&str, &str, &str)],
    default: &str,
    show_in_list: bool,
) -> FieldDefDto {
    let options_json: Vec<Value> = options
        .iter()
        .map(|(option_key, option_label, color)| {
            json!({ "key": option_key, "label": option_label, "color": color })
        })
        .collect();
    field(
        key,
        label,
        "status",
        true,
        order,
        json!({ "options": options_json, "default": default }),
        None,
        show_in_list,
    )
}

fn schema(
    key: &str,
    name: &str,
    icon: &str,
    color: &str,
    order: i64,
    fields: Vec<FieldDefDto>,
    badge_key: Option<&str>,
) -> SchemaDto {
    let badge_field_id = badge_key.and_then(|wanted| {
        fields
            .iter()
            .find(|field| field.key == wanted)
            .map(|field| field.id.clone())
    });
    SchemaDto {
        id: new_id(),
        key: key.to_string(),
        name: name.to_string(),
        icon: icon.to_string(),
        color: color.to_string(),
        order,
        title_field_id: None,
        subtitle_field_id: None,
        badge_field_id,
        format_version: 1,
        fields,
    }
}

fn seed_schemas(german: bool) -> Vec<SchemaDto> {
    fn tr(german: bool, en: &'static str, de: &'static str) -> &'static str {
        if german {
            de
        } else {
            en
        }
    }
    let t = |en: &'static str, de: &'static str| tr(german, en, de);
    let mut schemas = Vec::new();

    schemas.push(schema(
        "note",
        t("Note", "Notiz"),
        "sticky-note",
        "#64748b",
        0,
        vec![
            field(
                "summary",
                t("Summary", "Zusammenfassung"),
                "text",
                false,
                0,
                json!({ "maxLength": 300 }),
                None,
                true,
            ),
            status_field(
                "status",
                t("Status", "Status"),
                1,
                &[
                    ("draft", t("Draft", "Entwurf"), "#64748b"),
                    ("active", t("Active", "Aktiv"), "#0ea5e9"),
                    ("archived", t("Archived", "Archiviert"), "#94a3b8"),
                ],
                "active",
                true,
            ),
        ],
        Some("status"),
    ));

    schemas.push(schema(
        "todo",
        t("To-do", "Aufgabe"),
        "square-check-big",
        "#0ea5e9",
        1,
        vec![
            status_field(
                "status",
                t("Status", "Status"),
                0,
                &[
                    ("open", t("Open", "Offen"), "#64748b"),
                    ("in_progress", t("In progress", "In Arbeit"), "#0ea5e9"),
                    ("blocked", t("Blocked", "Blockiert"), "#ef4444"),
                    ("done", t("Done", "Erledigt"), "#22c55e"),
                ],
                "open",
                true,
            ),
            field(
                "due",
                t("Due", "Fällig"),
                "date",
                false,
                1,
                json!({ "withTime": false }),
                None,
                true,
            ),
            field(
                "priority",
                t("Priority", "Priorität"),
                "scale",
                false,
                2,
                json!({ "min": 1, "max": 6, "default": 3 }),
                None,
                true,
            ),
            field(
                "effort",
                t("Effort", "Aufwand"),
                "number",
                false,
                3,
                json!({ "min": 0, "step": 0.25, "suffix": "h" }),
                None,
                true,
            ),
        ],
        Some("status"),
    ));

    schemas.push(schema(
        "hypothesis",
        t("Hypothesis", "Hypothese"),
        "lightbulb",
        "#f59e0b",
        2,
        vec![
            status_field(
                "status",
                t("Status", "Status"),
                0,
                &[
                    ("untested", t("Untested", "Ungeprüft"), "#64748b"),
                    ("testing", t("Testing", "In Prüfung"), "#0ea5e9"),
                    ("supported", t("Supported", "Bestätigt"), "#22c55e"),
                    ("refuted", t("Refuted", "Widerlegt"), "#ef4444"),
                ],
                "untested",
                true,
            ),
            field(
                "confidence",
                t("Confidence", "Zuversicht"),
                "scale",
                false,
                1,
                json!({ "min": 1, "max": 6 }),
                None,
                true,
            ),
            field(
                "assumptions",
                t("Assumptions", "Annahmen"),
                "longtext",
                false,
                2,
                json!({ "rows": 4 }),
                None,
                false,
            ),
            field(
                "prediction",
                t("Prediction", "Vorhersage"),
                "longtext",
                false,
                3,
                json!({ "rows": 4 }),
                None,
                false,
            ),
        ],
        Some("status"),
    ));

    schemas.push(schema(
        "experiment",
        t("Experiment", "Experiment"),
        "flask-conical",
        "#8b5cf6",
        3,
        vec![
            status_field(
                "status",
                t("Status", "Status"),
                0,
                &[
                    ("planned", t("Planned", "Geplant"), "#64748b"),
                    ("running", t("Running", "Läuft"), "#0ea5e9"),
                    ("done", t("Done", "Fertig"), "#22c55e"),
                    ("aborted", t("Aborted", "Abgebrochen"), "#ef4444"),
                ],
                "planned",
                true,
            ),
            field(
                "importance",
                t("Importance", "Wichtigkeit"),
                "scale",
                false,
                1,
                json!({ "min": 1, "max": 6, "default": 3 }),
                None,
                true,
            ),
            field(
                "complexity",
                t("Complexity", "Komplexität"),
                "scale",
                false,
                2,
                json!({ "min": 1, "max": 6, "default": 3 }),
                None,
                true,
            ),
            field(
                "time",
                t("Time consumption", "Zeitaufwand"),
                "scale",
                false,
                3,
                json!({ "min": 1, "max": 6, "default": 3 }),
                None,
                true,
            ),
            field(
                "productivity",
                t("Productivity", "Produktivität"),
                "formula",
                false,
                4,
                json!({ "precision": 2, "suffix": "" }),
                Some("round((importance ^ 2) / (complexity * time), 2)"),
                true,
            ),
            field(
                "result",
                t("Result", "Ergebnis"),
                "longtext",
                false,
                5,
                json!({ "rows": 4 }),
                None,
                false,
            ),
        ],
        Some("status"),
    ));

    schemas
}

pub fn compute_formulas(schema: &SchemaDto, values: &Map<String, Value>) -> Map<String, Value> {
    let mut context: HashMap<String, Option<f64>> = HashMap::new();
    for field in &schema.fields {
        if field.field_type == "formula" {
            continue;
        }
        if let Some(value) = values.get(&field.key) {
            context.insert(field.key.clone(), numeric_value(field, value));
        }
    }
    let mut computed = Map::new();
    for field in &schema.fields {
        if field.field_type != "formula" {
            continue;
        }
        let Some(expression) = &field.expression else {
            computed.insert(field.key.clone(), Value::Null);
            continue;
        };
        let result = formula::parse(expression)
            .ok()
            .and_then(|expr| formula::evaluate(&expr, &context));
        computed.insert(
            field.key.clone(),
            match result {
                Some(value) if value.is_finite() => {
                    let precision = field
                        .config
                        .get("precision")
                        .and_then(Value::as_u64)
                        .unwrap_or(2)
                        .min(12) as i32;
                    let factor = 10f64.powi(precision);
                    json!((value * factor).round() / factor)
                }
                _ => Value::Null,
            },
        );
    }
    computed
}

fn numeric_value(field: &FieldDefDto, value: &Value) -> Option<f64> {
    match field.field_type.as_str() {
        "number" | "scale" => value.as_f64(),
        "checkbox" => value.as_bool().map(|flag| if flag { 1.0 } else { 0.0 }),
        "text" | "longtext" | "status" | "date" => value
            .as_str()
            .and_then(|text| text.trim().parse::<f64>().ok()),
        _ => None,
    }
}

pub fn field_by_key<'a>(schema: &'a SchemaDto, key: &str) -> Option<&'a FieldDefDto> {
    schema.fields.iter().find(|field| field.key == key)
}

pub fn field_by_id<'a>(schema: &'a SchemaDto, id: &str) -> Option<&'a FieldDefDto> {
    schema.fields.iter().find(|field| field.id == id)
}
