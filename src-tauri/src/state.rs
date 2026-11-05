use crate::db::Db;
use crate::dto::Settings;
use crate::error::AppError;
use crate::schema::SchemaCache;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct Paths {
    pub data_dir: PathBuf,
    pub mirror_dir: PathBuf,
    pub log_dir: PathBuf,
    pub log_path: PathBuf,
    pub db_path: PathBuf,
    pub settings_path: PathBuf,
}

pub fn resolve_paths() -> Paths {
    let base = if let Ok(override_dir) = std::env::var("BETTER_NOTEPAD_DATA_DIR") {
        PathBuf::from(override_dir)
    } else if let Ok(appdata) = std::env::var("APPDATA") {
        Path::new(&appdata).join("Better Notepad")
    } else {
        PathBuf::from("Better Notepad")
    };
    let mirror_dir = base.join("mirror");
    let log_dir = base.join("logs");
    Paths {
        db_path: base.join("better-notepad.db"),
        settings_path: base.join("settings.json"),
        log_path: log_dir.join("app.log"),
        data_dir: base,
        mirror_dir,
        log_dir,
    }
}

/// Resolves the Google OAuth client credentials from the runtime environment
/// or, for release builds, from the values embedded by `build.rs`.
fn resolve_google_credentials() -> (Option<String>, Option<String>) {
    let client_id = std::env::var("BN_GOOGLE_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            option_env!("BN_GOOGLE_CLIENT_ID")
                .map(str::to_string)
                .filter(|value| !value.trim().is_empty())
        });
    let client_secret = std::env::var("BN_GOOGLE_CLIENT_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            option_env!("BN_GOOGLE_CLIENT_SECRET")
                .map(str::to_string)
                .filter(|value| !value.trim().is_empty())
        });
    (client_id, client_secret)
}

#[derive(Clone)]
pub struct AuthManager {
    pub client_id: Arc<RwLock<Option<String>>>,
    pub client_secret: Arc<RwLock<Option<String>>>,
    pub access_token: Arc<RwLock<Option<AccessToken>>>,
    pub signing_in: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: i64,
}

impl AuthManager {
    pub fn new(client_id: Option<String>, client_secret: Option<String>) -> Self {
        Self {
            client_id: Arc::new(RwLock::new(client_id)),
            client_secret: Arc::new(RwLock::new(client_secret)),
            access_token: Arc::new(RwLock::new(None)),
            signing_in: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn client_id(&self) -> Option<String> {
        self.client_id.read().clone()
    }

    pub fn client_secret(&self) -> Option<String> {
        self.client_secret.read().clone()
    }

    pub fn cached_token(&self) -> Option<AccessToken> {
        self.access_token.read().clone()
    }

    pub fn store_token(&self, token: String, expires_at: i64) {
        *self.access_token.write() = Some(AccessToken { token, expires_at });
    }

    pub fn clear(&self) {
        *self.access_token.write() = None;
    }
}

pub struct SyncEngine {
    pub status: RwLock<crate::dto::SyncStatus>,
    pub paused: std::sync::atomic::AtomicBool,
    pub cycle: parking_lot::Mutex<()>,
}

impl SyncEngine {
    pub fn new() -> Self {
        Self {
            status: RwLock::new(crate::dto::SyncStatus::default()),
            paused: std::sync::atomic::AtomicBool::new(false),
            cycle: parking_lot::Mutex::new(()),
        }
    }
}

pub struct StateInner {
    pub paths: Paths,
    pub db: Db,
    pub schemas: SchemaCache,
    pub settings: RwLock<Settings>,
    pub auth: AuthManager,
    pub sync: SyncEngine,
    pub app: RwLock<Option<AppHandle>>,
}

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<StateInner>,
}

impl AppState {
    pub fn new(paths: Paths) -> Result<Self, AppError> {
        let db = Db::open(&paths.db_path)?;
        let settings = Settings::load(&paths.settings_path);
        crate::util::ensure_dir(&paths.mirror_dir)?;
        crate::util::ensure_dir(&paths.log_dir)?;

        let (client_id, client_secret) = resolve_google_credentials();

        let schemas = SchemaCache::new();
        db.with_write(|conn| crate::schema::ensure_seeded(conn, &settings.language))?;
        schemas.reload(&db)?;

        Ok(Self {
            inner: Arc::new(StateInner {
                paths,
                db,
                schemas,
                settings: RwLock::new(settings),
                auth: AuthManager::new(client_id, client_secret),
                sync: SyncEngine::new(),
                app: RwLock::new(None),
            }),
        })
    }

    pub fn attach_app(&self, app: AppHandle) {
        *self.inner.app.write() = Some(app);
    }

    pub fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) {
        if let Some(app) = self.inner.app.read().as_ref() {
            if let Err(error) = app.emit(event, payload) {
                tracing::warn!(error = %error, event, "failed to emit event");
            }
        }
    }

    pub fn settings(&self) -> Settings {
        self.inner.settings.read().clone()
    }

    pub fn save_settings(&self, settings: Settings) -> Result<Settings, AppError> {
        settings.validate()?;
        settings.save(&self.inner.paths.settings_path)?;
        *self.inner.settings.write() = settings.clone();
        Ok(settings)
    }

    pub fn mirror_dir(&self) -> &Path {
        &self.inner.paths.mirror_dir
    }

    pub fn refresh_schemas(&self) -> Result<(), AppError> {
        self.inner.schemas.reload(&self.inner.db)
    }

    pub fn emit_workspace_changed(&self, reason: &str) {
        self.emit(
            "workspace://changed",
            serde_json::json!({ "reason": reason }),
        );
    }
}
