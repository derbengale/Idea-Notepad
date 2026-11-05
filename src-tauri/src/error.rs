use rusqlite::Error as SqlError;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Validation,
    NotFound,
    Conflict,
    Db,
    Io,
    AuthRequired,
    AuthFailed,
    Network,
    Quota,
    Unsupported,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Validation => "VALIDATION",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::Db => "DB",
            ErrorCode::Io => "IO",
            ErrorCode::AuthRequired => "AUTH_REQUIRED",
            ErrorCode::AuthFailed => "AUTH_FAILED",
            ErrorCode::Network => "NETWORK",
            ErrorCode::Quota => "QUOTA",
            ErrorCode::Unsupported => "UNSUPPORTED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let retryable = matches!(
            code,
            ErrorCode::Network | ErrorCode::Quota | ErrorCode::Db | ErrorCode::Io
        );
        Self {
            code,
            message: message.into(),
            details: None,
            retryable,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Validation, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    pub fn db(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Db, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Io, message)
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Network, message)
    }

    pub fn auth_required(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthRequired, message)
    }

    pub fn auth_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthFailed, message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unsupported, message)
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn field(field_key: &str, message: impl Into<String>) -> Self {
        Self::validation(message).with_details(serde_json::json!({ "fieldKey": field_key }))
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 4)?;
        state.serialize_field("code", self.code.as_str())?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("details", &self.details)?;
        state.serialize_field("retryable", &self.retryable)?;
        state.end()
    }
}

impl From<SqlError> for AppError {
    fn from(value: SqlError) -> Self {
        match value {
            SqlError::QueryReturnedNoRows => AppError::not_found("Row not found"),
            other => AppError::db(other.to_string()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::io(value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::validation(format!("Invalid JSON: {value}"))
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(value: serde_yaml::Error) -> Self {
        AppError::validation(format!("Invalid YAML: {value}"))
    }
}

impl From<uuid::Error> for AppError {
    fn from(value: uuid::Error) -> Self {
        AppError::validation(format!("Invalid identifier: {value}"))
    }
}

impl From<url::ParseError> for AppError {
    fn from(value: url::ParseError) -> Self {
        AppError::validation(format!("Invalid URL: {value}"))
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(value: zip::result::ZipError) -> Self {
        AppError::io(format!("Archive error: {value}"))
    }
}

impl From<keyring::Error> for AppError {
    fn from(value: keyring::Error) -> Self {
        AppError::io(format!("Credential store error: {value}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        let status = value.status().map(|s| s.as_u16());
        if let Some(code) = status {
            if code == 401 {
                return AppError::auth_required("Access token rejected");
            }
            if code == 403 || code == 429 {
                return AppError::new(
                    ErrorCode::Quota,
                    format!("Drive quota or rate limit ({code})"),
                );
            }
        }
        AppError::network(value.to_string())
    }
}
