//! Build script: exposes the local Google OAuth credentials from the
//! repository `.env` file to `option_env!` in the application crate.
//!
//! Credential values are never logged; only the keys are handled.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Keys that may be baked into the binary from the repository `.env` file.
const DOTENV_KEYS: [&str; 2] = ["BN_GOOGLE_CLIENT_ID", "BN_GOOGLE_CLIENT_SECRET"];

fn main() {
    bake_google_credentials();
    tauri_build::build();
}

/// Reads `<repo>/.env` and forwards the OAuth keys to `option_env!`.
///
/// A value already present in the process environment wins over the file.
fn bake_google_credentials() {
    let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let env_path = PathBuf::from(manifest_dir).join("..").join(".env");
    println!("cargo:rerun-if-changed={}", env_path.display());
    for key in DOTENV_KEYS {
        println!("cargo:rerun-if-env-changed={key}");
    }

    let Ok(contents) = fs::read_to_string(&env_path) else {
        return;
    };

    let values = parse_dotenv(&contents);
    for key in DOTENV_KEYS {
        if std::env::var_os(key).is_some() {
            continue;
        }
        if let Some(value) = values.get(key) {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}

/// Minimal `.env` parser: blank lines, `#` comments, an optional `export`
/// prefix, `KEY=VALUE` entries and matching single or double quotes.
fn parse_dotenv(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if !DOTENV_KEYS.contains(&key) {
            continue;
        }
        values.insert(key.to_string(), unquote(raw_value.trim()).to_string());
    }
    values
}

/// Strips one pair of matching single or double quotes from a value.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}
