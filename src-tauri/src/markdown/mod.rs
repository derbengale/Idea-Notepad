use crate::error::AppError;
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct NoteFile {
    pub id: String,
    pub schema: String,
    pub title: String,
    pub project: Option<String>,
    pub project_name: Option<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub created: String,
    pub updated: String,
    pub fields: Map<String, Value>,
    pub extra: Map<String, Value>,
    pub body: String,
}

pub fn parse_note(text: &str) -> Result<NoteFile, AppError> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.split_inclusive('\n');
    match lines.next() {
        Some(first) if first.trim_end() == "---" => {}
        _ => return Err(AppError::validation("Note has no front matter")),
    }
    let mut front_matter = String::new();
    let mut body = String::new();
    let mut in_front_matter = true;
    let mut closed = false;
    for line in lines {
        if in_front_matter {
            if line.trim_end() == "---" {
                in_front_matter = false;
                closed = true;
                continue;
            }
            front_matter.push_str(line);
        } else {
            body.push_str(line);
        }
    }
    if !closed {
        return Err(AppError::validation("Front matter is not terminated"));
    }
    if body.starts_with('\n') {
        body.remove(0);
    }

    let yaml: serde_yaml::Value = serde_yaml::from_str(&front_matter)?;
    let mapping = yaml
        .as_mapping()
        .ok_or_else(|| AppError::validation("Front matter must be a mapping"))?;

    let mut extra = Map::new();
    let mut fields = Map::new();
    let mut project = None;
    let mut project_name = None;
    let mut tags = Vec::new();
    let mut links = Vec::new();
    let mut id = String::new();
    let mut schema = String::new();
    let mut title = String::new();
    let mut created = String::new();
    let mut updated = String::new();
    let mut version = 0i64;

    for (key, value) in mapping {
        let key = key.as_str().unwrap_or_default().to_string();
        match key.as_str() {
            "bn" => version = value.as_i64().unwrap_or(0),
            "id" => id = string_value(value).unwrap_or_default(),
            "schema" => schema = string_value(value).unwrap_or_default(),
            "title" => title = string_value(value).unwrap_or_default(),
            "project" => project = string_value(value),
            "project_name" => project_name = string_value(value),
            "tags" => tags = string_list(value),
            "links" => links = string_list(value),
            "created" => created = string_value(value).unwrap_or_default(),
            "updated" => updated = string_value(value).unwrap_or_default(),
            "fields" => {
                if let Some(map) = value.as_mapping() {
                    for (field_key, field_value) in map {
                        if let Some(field_key) = field_key.as_str() {
                            fields.insert(field_key.to_string(), yaml_to_json(field_value));
                        }
                    }
                }
            }
            "extra" => {
                if let Some(map) = value.as_mapping() {
                    for (extra_key, extra_value) in map {
                        if let Some(extra_key) = extra_key.as_str() {
                            extra.insert(extra_key.to_string(), yaml_to_json(extra_value));
                        }
                    }
                }
            }
            other => {
                extra.insert(other.to_string(), yaml_to_json(value));
            }
        }
    }

    if version != 1 {
        return Err(AppError::unsupported(format!(
            "Unsupported note format version {version}"
        )));
    }
    if !crate::util::is_uuid(&id) {
        return Err(AppError::validation("Note has an invalid id"));
    }
    if schema.trim().is_empty() {
        return Err(AppError::validation("Note has no schema"));
    }
    if title.trim().is_empty() {
        return Err(AppError::validation("Note has no title"));
    }
    if created.is_empty() || updated.is_empty() {
        return Err(AppError::validation("Note is missing timestamps"));
    }

    Ok(NoteFile {
        id,
        schema,
        title,
        project,
        project_name,
        tags,
        links,
        created,
        updated,
        fields,
        extra,
        body: crate::util::normalize_body(&body),
    })
}

pub fn serialize_note(note: &NoteFile) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("bn: 1\n");
    out.push_str(&format!("id: {}\n", yaml_string(&note.id)));
    out.push_str(&format!("schema: {}\n", yaml_string(&note.schema)));
    out.push_str(&format!("title: {}\n", yaml_string(&note.title)));
    match &note.project {
        Some(project) => out.push_str(&format!("project: {}\n", yaml_string(project))),
        None => out.push_str("project: null\n"),
    }
    match &note.project_name {
        Some(name) => out.push_str(&format!("project_name: {}\n", yaml_string(name))),
        None => out.push_str("project_name: null\n"),
    }
    out.push_str(&format!("tags: {}\n", inline_string_list(&note.tags)));
    if note.links.is_empty() {
        out.push_str("links: []\n");
    } else {
        out.push_str("links:\n");
        for link in &note.links {
            out.push_str(&format!("  - {}\n", yaml_string(link)));
        }
    }
    out.push_str(&format!("created: {}\n", yaml_string(&note.created)));
    out.push_str(&format!("updated: {}\n", yaml_string(&note.updated)));
    if note.fields.is_empty() {
        out.push_str("fields: {}\n");
    } else {
        out.push_str("fields:\n");
        for (key, value) in &note.fields {
            out.push_str(&format!("  {}: {}\n", yaml_key(key), yaml_inline(value)));
        }
    }
    if !note.extra.is_empty() {
        out.push_str("extra:\n");
        for (key, value) in &note.extra {
            match serde_yaml::to_string(value) {
                Ok(yaml) => {
                    out.push_str(&format!("  {}:\n", yaml_key(key)));
                    for line in yaml.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        out.push_str(&format!("    {line}\n"));
                    }
                }
                Err(_) => {
                    out.push_str(&format!("  {}: null\n", yaml_key(key)));
                }
            }
        }
    }
    out.push_str("---\n\n");
    out.push_str(&crate::util::normalize_body(&note.body));
    out
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn yaml_key(key: &str) -> String {
    if is_plain_key(key) {
        key.to_string()
    } else {
        yaml_string(key)
    }
}

fn is_plain_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        && key
            .chars()
            .next()
            .map(|ch| !ch.is_ascii_digit())
            .unwrap_or(false)
}

fn inline_string_list(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let rendered: Vec<String> = items.iter().map(|item| yaml_string(item)).collect();
    format!("[{}]", rendered.join(", "))
}

fn yaml_inline(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => yaml_string(text),
        Value::Array(items) => {
            let rendered: Vec<String> = items.iter().map(yaml_inline).collect();
            format!("[{}]", rendered.join(", "))
        }
        Value::Object(_) => serde_json::to_string(value)
            .map(|json| yaml_string(&json))
            .unwrap_or_else(|_| "null".to_string()),
    }
}

fn string_value(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(text) => Some(text.clone()),
        serde_yaml::Value::Null => None,
        serde_yaml::Value::Number(number) => Some(number.to_string()),
        serde_yaml::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn string_list(value: &serde_yaml::Value) -> Vec<String> {
    match value {
        serde_yaml::Value::Sequence(items) => items.iter().filter_map(string_value).collect(),
        serde_yaml::Value::String(text) => vec![text.clone()],
        _ => Vec::new(),
    }
}

fn yaml_to_json(value: &serde_yaml::Value) -> Value {
    match value {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(flag) => Value::Bool(*flag),
        serde_yaml::Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                Value::from(int)
            } else if let Some(float) = number.as_f64() {
                serde_json::Number::from_f64(float)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(text) => Value::String(text.clone()),
        serde_yaml::Value::Sequence(items) => {
            Value::Array(items.iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut object = Map::new();
            for (key, value) in map {
                if let Some(key) = key.as_str() {
                    object.insert(key.to_string(), yaml_to_json(value));
                }
            }
            Value::Object(object)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

pub fn resolve_wiki_links<F>(body: &str, resolver: F) -> String
where
    F: Fn(&str) -> Vec<(String, String)>,
{
    let mut output = String::with_capacity(body.len());
    let chars: Vec<char> = body.chars().collect();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '[' && chars.get(index + 1) == Some(&'[') {
            if let Some(end) = find_closing_brackets(&chars, index + 2) {
                let label: String = chars[index + 2..end].iter().collect();
                let label_trimmed = label.trim();
                let matches = resolver(label_trimmed);
                if matches.len() == 1 {
                    let (id, title) = &matches[0];
                    output.push_str(&format!("[{title}](note:{id})"));
                } else {
                    output.push_str(&format!("[[{label}]]"));
                }
                index = end + 2;
                continue;
            }
        }
        output.push(chars[index]);
        index += 1;
    }
    output
}

fn find_closing_brackets(chars: &[char], start: usize) -> Option<usize> {
    let mut index = start;
    while index + 1 < chars.len() {
        if chars[index] == ']' && chars[index + 1] == ']' {
            return Some(index);
        }
        index += 1;
    }
    None
}

pub fn extract_note_links(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(position) = rest.find("](note:") {
        let after = &rest[position + 7..];
        if let Some(end) = after.find(')') {
            let candidate = &after[..end];
            if crate::util::is_uuid(candidate) {
                out.push(candidate.to_string());
            }
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn rewrite_note_links<F>(body: &str, mut mapper: F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let mut output = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(position) = rest.find("](note:") {
        let before_and_marker = &rest[..position + 7];
        output.push_str(before_and_marker);
        let after = &rest[position + 7..];
        if let Some(end) = after.find(')') {
            let id = &after[..end];
            match mapper(id) {
                Some(replacement) => output.push_str(&replacement),
                None => output.push_str(id),
            }
            output.push(')');
            rest = &after[end + 1..];
        } else {
            output.push_str(after);
            return output;
        }
    }
    output.push_str(rest);
    output
}
