use crate::db::{repo_entry, repo_schema, repo_sync};
use crate::dto::{DeletedCount, FieldDefDto, SchemaDto, SchemaInput};
use crate::error::AppError;
use crate::schema::{self, formula};
use crate::state::AppState;
use crate::util::{new_id, slugify};
use serde_json::{json, Map, Value};
use tauri::State;

#[tauri::command]
pub fn schema_list(state: State<'_, AppState>) -> Result<Vec<SchemaDto>, AppError> {
    Ok(state.inner.schemas.list())
}

#[tauri::command]
pub fn schema_create(
    state: State<'_, AppState>,
    input: SchemaInput,
) -> Result<SchemaDto, AppError> {
    if let Some(id) = &input.id {
        if state.inner.schemas.get(id).is_some() {
            return Err(AppError::conflict("A schema with this id already exists"));
        }
    }
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::field("name", "Schema name must not be empty"));
    }
    if name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Schema name must not exceed 60 characters",
        ));
    }
    let mut key = match input.key {
        Some(key) if !key.trim().is_empty() => key.trim().to_string(),
        _ => slugify(&name).replace('-', "_"),
    };
    if key.is_empty() {
        key = "schema".to_string();
    }
    let base = key.clone();
    let mut counter = 2;
    while state.inner.schemas.get_by_key(&key).is_some() {
        key = format!("{base}_{counter}");
        counter += 1;
    }
    let order = state
        .inner
        .schemas
        .list()
        .iter()
        .map(|schema| schema.order)
        .max()
        .unwrap_or(-1)
        + 1;
    let fields = input
        .fields
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, field)| normalize_field(field, index as i64))
        .collect();
    let schema = SchemaDto {
        id: input.id.unwrap_or_else(new_id),
        key,
        name,
        icon: input.icon.unwrap_or_else(|| "file-text".to_string()),
        color: input.color.unwrap_or_else(|| "#64748b".to_string()),
        order: input.order.unwrap_or(order),
        title_field_id: input.title_field_id,
        subtitle_field_id: input.subtitle_field_id,
        badge_field_id: input.badge_field_id,
        format_version: 1,
        fields,
    };
    schema::validate::validate_schema(&schema)?;
    state
        .inner
        .db
        .with_write(|conn| repo_schema::insert(conn, &schema))?;
    state.refresh_schemas()?;
    enqueue_schema(&state, &schema.id)?;
    Ok(schema)
}

#[tauri::command]
pub fn schema_update(state: State<'_, AppState>, schema: SchemaDto) -> Result<SchemaDto, AppError> {
    let existing = state.inner.schemas.require(&schema.id)?;
    if existing.key != schema.key {
        return Err(AppError::field(
            "key",
            "The schema key cannot be changed after creation",
        ));
    }
    let mut next = schema;
    next.fields = next
        .fields
        .into_iter()
        .enumerate()
        .map(|(index, field)| normalize_field(field, index as i64))
        .collect();
    schema::validate::validate_schema(&next)?;
    let cleared: Vec<String> = next
        .fields
        .iter()
        .filter_map(|field| {
            let previous = existing.fields.iter().find(|old| old.id == field.id)?;
            if previous.field_type == field.field_type {
                return None;
            }
            let compatible = matches!(
                (previous.field_type.as_str(), field.field_type.as_str()),
                ("text", "longtext")
                    | ("longtext", "text")
                    | ("number", "scale")
                    | ("scale", "number")
            );
            if compatible {
                None
            } else {
                Some(field.id.clone())
            }
        })
        .collect();
    let affected = state
        .inner
        .db
        .with_read(|conn| repo_entry::ids_for_schema(conn, &existing.id))?;
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_schema::update(&tx, &next)?;
        for field_id in &cleared {
            repo_schema::clear_field_values(&tx, field_id)?;
        }
        for entry_id in &affected {
            repo_sync::enqueue(&tx, "note.write", entry_id, json!({ "entryId": entry_id }))?;
        }
        tx.commit()?;
        Ok(())
    })?;
    state.refresh_schemas()?;
    enqueue_schema(&state, &next.id)?;
    state
        .inner
        .db
        .with_read(|conn| repo_schema::get(conn, &next.id))
}

#[tauri::command]
pub fn schema_delete(
    state: State<'_, AppState>,
    id: String,
    reassign_to: String,
) -> Result<DeletedCount, AppError> {
    let affected = state
        .inner
        .db
        .with_read(|conn| repo_entry::ids_for_schema(conn, &id))?;
    let moved = state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let moved = repo_schema::delete(&tx, &id, &reassign_to)?;
        for entry_id in affected {
            repo_sync::enqueue(&tx, "note.write", &entry_id, json!({ "entryId": entry_id }))?;
        }
        tx.commit()?;
        Ok(moved)
    })?;
    state.refresh_schemas()?;
    enqueue_schema(&state, &reassign_to)?;
    Ok(DeletedCount {
        moved_entries: moved,
    })
}

#[tauri::command]
pub fn schema_reorder(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<SchemaDto>, AppError> {
    state
        .inner
        .db
        .with_write(|conn| repo_schema::reorder(conn, &ids))?;
    state.refresh_schemas()?;
    Ok(state.inner.schemas.list())
}

#[tauri::command]
pub fn schema_duplicate(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<SchemaDto, AppError> {
    let source = state.inner.schemas.require(&id)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::field("name", "Schema name must not be empty"));
    }
    let mut key = slugify(&name).replace('-', "_");
    if key.is_empty() {
        key = "schema".to_string();
    }
    let base = key.clone();
    let mut counter = 2;
    while state.inner.schemas.get_by_key(&key).is_some() {
        key = format!("{base}_{counter}");
        counter += 1;
    }
    let mut id_map: Map<String, Value> = Map::new();
    let mut fields = Vec::new();
    for field in &source.fields {
        let new_field_id = new_id();
        id_map.insert(field.id.clone(), Value::String(new_field_id.clone()));
        fields.push(FieldDefDto {
            id: new_field_id,
            key: field.key.clone(),
            label: field.label.clone(),
            field_type: field.field_type.clone(),
            required: field.required,
            order: field.order,
            config: field.config.clone(),
            expression: field.expression.clone(),
            show_in_list: field.show_in_list,
        });
    }
    let remap = |field_id: &Option<String>| -> Option<String> {
        let field_id = field_id.as_ref()?;
        id_map
            .get(field_id)
            .and_then(Value::as_str)
            .map(|value| value.to_string())
    };
    let order = state
        .inner
        .schemas
        .list()
        .iter()
        .map(|schema| schema.order)
        .max()
        .unwrap_or(-1)
        + 1;
    let schema = SchemaDto {
        id: new_id(),
        key,
        name,
        icon: source.icon.clone(),
        color: source.color.clone(),
        order,
        title_field_id: remap(&source.title_field_id),
        subtitle_field_id: remap(&source.subtitle_field_id),
        badge_field_id: remap(&source.badge_field_id),
        format_version: 1,
        fields,
    };
    schema::validate::validate_schema(&schema)?;
    state
        .inner
        .db
        .with_write(|conn| repo_schema::insert(conn, &schema))?;
    state.refresh_schemas()?;
    enqueue_schema(&state, &schema.id)?;
    Ok(schema)
}

#[tauri::command]
pub fn formula_validate(
    state: State<'_, AppState>,
    expression: String,
    schema_id: Option<String>,
) -> Result<Value, AppError> {
    let parsed = match formula::parse(&expression) {
        Ok(parsed) => parsed,
        Err(error) => return Ok(json!({ "ok": false, "error": error.message, "references": [] })),
    };
    let references = formula::identifiers(&parsed);
    if let Some(schema_id) = schema_id {
        if let Some(schema) = state.inner.schemas.get(&schema_id) {
            let known: Vec<String> = schema
                .fields
                .iter()
                .map(|field| field.key.clone())
                .collect();
            if let Some(unknown) = references
                .iter()
                .find(|reference| !known.contains(reference))
            {
                return Ok(json!({
                    "ok": false,
                    "error": format!("Unknown field '{unknown}'"),
                    "references": references
                }));
            }
        }
    }
    Ok(json!({ "ok": true, "error": null, "references": references }))
}

#[tauri::command]
pub fn formula_eval(
    state: State<'_, AppState>,
    schema_id: String,
    values: Map<String, Value>,
) -> Result<Value, AppError> {
    let schema = state.inner.schemas.require(&schema_id)?;
    let computed = schema::compute_formulas(&schema, &values);
    Ok(json!({ "values": computed }))
}

fn normalize_field(mut field: FieldDefDto, index: i64) -> FieldDefDto {
    if field.id.trim().is_empty() {
        field.id = new_id();
    }
    if field.label.trim().is_empty() {
        field.label = field.key.clone();
    }
    if field.order < 0 {
        field.order = index;
    }
    field
}

fn enqueue_schema(state: &State<'_, AppState>, schema_id: &str) -> Result<(), AppError> {
    state.inner.db.with_write(|conn| {
        repo_sync::enqueue(
            conn,
            "schema.write",
            schema_id,
            json!({ "schemaId": schema_id }),
        )
    })
}
