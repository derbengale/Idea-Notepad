use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub default_schema_id: Option<String>,
    pub last_schema_id: Option<String>,
    pub editor_mode: String,
    pub editor_font_size: u32,
    pub spell_check: bool,
    pub load_remote_images: bool,
    pub allow_raw_html: bool,
    pub density: String,
    pub sidebar_width: f64,
    pub list_width: f64,
    pub detail_width: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            language: "system".to_string(),
            default_schema_id: None,
            last_schema_id: None,
            editor_mode: "split".to_string(),
            editor_font_size: 14,
            spell_check: true,
            load_remote_images: false,
            allow_raw_html: false,
            density: "comfortable".to_string(),
            sidebar_width: 260.0,
            list_width: 400.0,
            detail_width: 0.0,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        let json = serde_json::to_vec_pretty(self)?;
        crate::util::atomic_write(path, &json)
    }

    pub fn validate(&self) -> Result<(), AppError> {
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            return Err(AppError::validation("Unknown theme"));
        }
        if !matches!(self.language.as_str(), "system" | "en" | "de") {
            return Err(AppError::validation("Unknown language"));
        }
        if !matches!(self.editor_mode.as_str(), "source" | "preview" | "split") {
            return Err(AppError::validation("Unknown editor mode"));
        }
        if !(10..=24).contains(&self.editor_font_size) {
            return Err(AppError::validation(
                "Editor font size must be between 10 and 24",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub data_dir: String,
    pub db_path: String,
    pub log_path: String,
    pub mirror_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    pub note_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDefDto {
    pub id: String,
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    pub order: i64,
    pub config: Value,
    pub expression: Option<String>,
    pub show_in_list: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDto {
    pub id: String,
    pub key: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub order: i64,
    pub title_field_id: Option<String>,
    pub subtitle_field_id: Option<String>,
    pub badge_field_id: Option<String>,
    pub format_version: i64,
    pub fields: Vec<FieldDefDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaInput {
    pub id: Option<String>,
    pub key: Option<String>,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub fields: Option<Vec<FieldDefDto>>,
    pub order: Option<i64>,
    pub title_field_id: Option<String>,
    pub subtitle_field_id: Option<String>,
    pub badge_field_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryListItem {
    pub id: String,
    pub title: String,
    pub schema_id: String,
    pub schema_key: String,
    pub project_id: Option<String>,
    pub tags: Vec<TagDto>,
    pub subtitle: Option<String>,
    pub badge: Option<String>,
    pub badge_color: Option<String>,
    pub computed: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub sync_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkItem {
    pub id: Option<String>,
    pub entry_id: String,
    pub title: String,
    pub schema_key: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkBundle {
    pub links: Vec<LinkItem>,
    pub backlinks: Vec<LinkItem>,
    pub markdown_links: Vec<LinkItem>,
    pub markdown_backlinks: Vec<LinkItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDto {
    pub id: String,
    pub entry_id: String,
    pub kind: String,
    pub file_name: String,
    pub mime_type: String,
    pub byte_size: i64,
    pub sha256: String,
    pub local_path: String,
    pub preview_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDetail {
    pub id: String,
    pub schema_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub body_md: String,
    pub values: Map<String, Value>,
    pub computed: Map<String, Value>,
    pub tags: Vec<TagDto>,
    pub links: LinkBundle,
    pub attachments: Vec<AttachmentDto>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub sync_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryInput {
    pub id: Option<String>,
    pub schema_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub body_md: String,
    #[serde(default)]
    pub values: Map<String, Value>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    pub expected_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortSpec {
    pub key: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EntryQuery {
    pub search: Option<String>,
    pub schema_ids: Vec<String>,
    pub project_id: Option<String>,
    pub tag_ids: Vec<String>,
    pub status_value: Option<String>,
    pub sort: SortSpec,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub include_deleted: bool,
}

impl Default for EntryQuery {
    fn default() -> Self {
        Self {
            search: None,
            schema_ids: Vec::new(),
            project_id: None,
            tag_ids: Vec::new(),
            status_value: None,
            sort: SortSpec {
                key: "updated".to_string(),
                direction: "desc".to_string(),
            },
            cursor: None,
            limit: None,
            include_deleted: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPage {
    pub items: Vec<EntryListItem>,
    pub total: i64,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub entry_id: String,
    pub title: String,
    pub schema_key: String,
    pub snippet: String,
    pub rank: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub items: Vec<SearchHit>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStats {
    pub entries: i64,
    pub projects: i64,
    pub schemas: i64,
    pub attachments: i64,
    pub trash: i64,
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildReport {
    pub notes: i64,
    pub attachments: i64,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub signed_in: bool,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub has_client_id: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub state: String,
    pub last_sync_at: Option<String>,
    pub pending_operations: i64,
    pub conflict_count: i64,
    pub last_error: Option<AppError>,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            state: "idle".to_string(),
            last_sync_at: None,
            pending_operations: 0,
            conflict_count: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub notes: i64,
    pub attachments: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleCounts {
    pub notes: i64,
    pub schemas: i64,
    pub attachments: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleScope {
    #[serde(rename = "type")]
    pub scope_type: String,
    pub project_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleNoteEntry {
    pub id: String,
    pub file: String,
    pub title: String,
    pub updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleSchemaEntry {
    pub key: String,
    pub id: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleAttachmentEntry {
    pub id: String,
    pub entry_id: String,
    pub file: String,
    pub mime_type: String,
    pub sha256: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub format: String,
    pub version: i64,
    pub exported_at: String,
    pub app_version: String,
    pub scope: BundleScope,
    pub counts: BundleCounts,
    pub schemas: Vec<BundleSchemaEntry>,
    pub notes: Vec<BundleNoteEntry>,
    pub attachments: Vec<BundleAttachmentEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCounts {
    pub total: i64,
    pub new: i64,
    pub updated: i64,
    pub unchanged: i64,
    pub conflicts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePreview {
    pub format_version: i64,
    pub exported_at: String,
    pub app_version: String,
    pub scope: BundleScope,
    pub notes: PreviewCounts,
    pub schemas: Vec<String>,
    pub schema_conflicts: Vec<String>,
    pub projects: Vec<String>,
    pub attachments_total: i64,
    pub attachments_bytes: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub created: i64,
    pub updated: i64,
    pub skipped: i64,
    pub conflicts: i64,
    pub attachments_copied: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedCount {
    pub moved_entries: i64,
}
