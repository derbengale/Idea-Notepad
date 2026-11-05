use crate::error::AppError;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::{debug, info, warn};
use url::Url;

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";
const USERINFO_ENDPOINT: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const SERVICE: &str = "better-notepad";
const SCOPES: &str = "openid email profile https://www.googleapis.com/auth/drive.appdata";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProfile {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

/// Extracts the machine-readable `error` and `error_description` from a failed
/// token response without ever touching token values.
fn describe_token_error(status: reqwest::StatusCode, body: &str) -> (String, String) {
    match serde_json::from_str::<TokenError>(body) {
        Ok(parsed) => (parsed.error, parsed.error_description.unwrap_or_default()),
        Err(_) => (format!("http_{}", status.as_u16()), String::new()),
    }
}

fn token_form(client_id: &str, client_secret: Option<&str>) -> Vec<(&'static str, String)> {
    let mut form = vec![("client_id", client_id.to_string())];
    if let Some(secret) = client_secret
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        form.push(("client_secret", secret.to_string()));
    }
    form
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

pub struct SignInResult {
    pub profile: AuthProfile,
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
}

fn random_bytes(count: usize) -> Vec<u8> {
    let mut buffer = vec![0u8; count];
    rand::thread_rng().fill_bytes(&mut buffer);
    buffer
}

fn base64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

pub fn sign_in(client_id: &str, client_secret: Option<&str>) -> Result<SignInResult, AppError> {
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|error| AppError::auth_failed(format!("Cannot start loopback server: {error}")))?;
    let address = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| AppError::auth_failed("Loopback server has no IP address"))?;
    let redirect_uri = format!("http://127.0.0.1:{}/oauth2callback", address.port());
    info!(
        port = address.port(),
        "starting Google sign-in with loopback redirect"
    );

    let state = base64url(&random_bytes(32));
    let verifier = base64url(&random_bytes(48));
    let challenge = base64url(&Sha256::digest(verifier.as_bytes()));

    let mut url = Url::parse(AUTH_ENDPOINT)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", SCOPES)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");

    open::that(url.as_str())
        .map_err(|error| AppError::auth_failed(format!("Cannot open the browser: {error}")))?;

    let mut code: Option<String> = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(180);
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        match server.recv_timeout(remaining) {
            Ok(Some(request)) => {
                let request_url = request.url().to_string();
                if request_url.starts_with("/favicon") {
                    let _ = request.respond(tiny_http::Response::empty(404));
                    continue;
                }
                let parsed = Url::parse(&format!("http://127.0.0.1{request_url}"))?;
                if parsed.path() != "/oauth2callback" {
                    let _ = request.respond(tiny_http::Response::empty(404));
                    continue;
                }
                let mut returned_state = String::new();
                let mut returned_code = String::new();
                let mut error = String::new();
                for (key, value) in parsed.query_pairs() {
                    match key.as_ref() {
                        "state" => returned_state = value.to_string(),
                        "code" => returned_code = value.to_string(),
                        "error" => error = value.to_string(),
                        _ => {}
                    }
                }
                let body = if !error.is_empty() {
                    "<html><body><p>Sign-in was cancelled. You can close this window.</p></body></html>"
                } else {
                    "<html><body><p>Better Notepad received the sign-in response. You can close this window and return to the app.</p></body></html>"
                };
                let response = tiny_http::Response::from_string(body).with_header(
                    tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8")
                        .map_err(|_| AppError::auth_failed("Cannot build HTTP response"))?,
                );
                let _ = request.respond(response);
                if !error.is_empty() {
                    drop(server);
                    return Err(AppError::auth_failed("Sign-in was cancelled")
                        .with_details(serde_json::json!({ "reason": "cancelled" })));
                }
                if !constant_time_eq(&returned_state, &state) {
                    warn!("OAuth callback rejected: state mismatch");
                    drop(server);
                    return Err(AppError::auth_failed("OAuth state mismatch")
                        .with_details(serde_json::json!({ "reason": "state" })));
                }
                debug!("OAuth callback received and state validated");
                code = Some(returned_code);
                break;
            }
            Ok(None) => continue,
            Err(_) => break,
        }
    }
    drop(server);

    let code = code.ok_or_else(|| {
        AppError::auth_failed("Sign-in timed out")
            .with_details(serde_json::json!({ "reason": "state" }))
    })?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let mut form = token_form(client_id, client_secret);
    form.push(("code", code));
    form.push(("code_verifier", verifier));
    form.push(("grant_type", "authorization_code".to_string()));
    form.push(("redirect_uri", redirect_uri));
    let response = client.post(TOKEN_ENDPOINT).form(&form).send()?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        let (google_error, description) = describe_token_error(status, &body);
        warn!(
            status = status.as_u16(),
            google_error = %google_error,
            "Google rejected the authorization code"
        );
        let mut message = format!("Google rejected the authorization code: {google_error}");
        if !description.is_empty() {
            message.push_str(&format!(" ({description})"));
        }
        if description.contains("client_secret") || google_error == "invalid_client" {
            message.push_str(". Add the OAuth client secret in Settings → Sync.");
        }
        return Err(
            AppError::auth_failed(message).with_details(serde_json::json!({
                "reason": "token",
                "googleError": google_error,
                "status": status.as_u16(),
            })),
        );
    }
    let token: TokenResponse = response
        .json()
        .map_err(|_| AppError::auth_failed("Google returned an unexpected token response"))?;
    let refresh_token = token.refresh_token.ok_or_else(|| {
        warn!("Google returned no refresh token");
        AppError::auth_failed("No refresh token was returned. Remove the app from your Google account and sign in again.")
            .with_details(serde_json::json!({ "reason": "refresh_token" }))
    })?;
    info!(has_refresh_token = true, "Google token exchange completed");

    let response = client
        .get(USERINFO_ENDPOINT)
        .bearer_auth(&token.access_token)
        .send()?;
    let status = response.status();
    if !status.is_success() {
        warn!(status = status.as_u16(), "Google userinfo request failed");
        return Err(
            AppError::auth_failed("Google returned an unexpected user profile").with_details(
                serde_json::json!({ "reason": "userinfo", "status": status.as_u16() }),
            ),
        );
    }
    let userinfo: UserInfo = response
        .json()
        .map_err(|_| AppError::auth_failed("Google returned an unexpected user profile"))?;

    Ok(SignInResult {
        profile: AuthProfile {
            sub: userinfo.sub,
            email: userinfo.email,
            name: userinfo.name,
            picture: userinfo.picture,
        },
        access_token: token.access_token,
        expires_in: token.expires_in,
        refresh_token,
    })
}

pub fn refresh(
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> Result<(String, i64), AppError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let mut form = token_form(client_id, client_secret);
    form.push(("refresh_token", refresh_token.to_string()));
    form.push(("grant_type", "refresh_token".to_string()));
    let response = client.post(TOKEN_ENDPOINT).form(&form).send()?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        let (google_error, description) = describe_token_error(status, &body);
        warn!(
            status = status.as_u16(),
            google_error = %google_error,
            "Google token refresh failed"
        );
        if google_error == "invalid_grant" {
            return Err(
                AppError::auth_required("The Google session expired. Sign in again.")
                    .with_details(serde_json::json!({ "reason": "invalid_grant" })),
            );
        }
        let mut message = format!("Google rejected the token refresh: {google_error}");
        if !description.is_empty() {
            message.push_str(&format!(" ({description})"));
        }
        if description.contains("client_secret") || google_error == "invalid_client" {
            message.push_str(". Check the OAuth client ID and client secret in Settings → Sync.");
        }
        return Err(
            AppError::auth_failed(message).with_details(serde_json::json!({
                "reason": "client",
                "googleError": google_error,
                "status": status.as_u16(),
            })),
        );
    }
    let token: TokenResponse = response
        .json()
        .map_err(|_| AppError::auth_required("Google returned an unexpected refresh response"))?;
    Ok((token.access_token, token.expires_in))
}

pub fn revoke(token: &str) {
    if let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
    {
        let _ = client
            .post(REVOKE_ENDPOINT)
            .form(&[("token", token)])
            .send();
    }
}

pub fn store_refresh_token(sub: &str, token: &str) -> Result<(), AppError> {
    let entry = keyring::Entry::new(SERVICE, sub)?;
    entry.set_password(token)?;
    Ok(())
}

pub fn load_refresh_token(sub: &str) -> Result<Option<String>, AppError> {
    let entry = keyring::Entry::new(SERVICE, sub)?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn delete_refresh_token(sub: &str) -> Result<(), AppError> {
    let entry = keyring::Entry::new(SERVICE, sub)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
