//! Device Authorization Grant / Amazon "Code-Based Linking" (CBL) login flow
//! (RFC 8628), the same flow used by Echo and Fire TV devices to pair with an
//! Amazon account via https://amazon.com/code.
//!
//! Flow:
//!   1. POST to Amazon's codepair endpoint to obtain a `user_code` and
//!      `device_code`.
//!   2. Show the user the `user_code` and the verification URL
//!      (https://amazon.com/code), optionally opening it in a browser.
//!   3. Poll the token endpoint with the `device_code` until the user has
//!      entered the code and approved the request (or the code expires).
//!   4. POST the resulting access_token to Amazon's token-exchange endpoint to
//!      obtain amazon.com session cookies, then follow through to
//!      alexa.amazon.com to finalize the Alexa session.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use reqwest_cookie_store::CookieStoreMutex;

use super::cookie_store::save_cookie_store;
use super::login::build_client;
use crate::config::Settings;

// ── Amazon OAuth endpoints ────────────────────────────────────────────────────

const AMAZON_CODEPAIR_URL: &str = "https://api.amazon.com/auth/O2/create/codepair";
const AMAZON_TOKEN_URL: &str = "https://api.amazon.com/auth/o2/token";
/// Exchanges a web access_token for amazon.com session cookies.
const AMAZON_EXCHANGE_URL: &str = "https://www.amazon.com/ap/exchangetoken/sidebar";
/// User-facing page where the device code is entered.
const VERIFICATION_URI: &str = "https://amazon.com/code";

/// Scopes needed to access alexa.amazon.com internal APIs.
const OAUTH_SCOPES: &str = "alexa:all";

// ── Device code request ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CodePairResponse {
    user_code: String,
    device_code: String,
    #[serde(default)]
    verification_uri: Option<String>,
    #[serde(default)]
    interval: Option<u64>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// Request a `user_code` / `device_code` pair from Amazon, scoped to this
/// "device" (identified by `product_id` + `device_serial_number`, which an
/// AVS Product registration assigns).
async fn request_code_pair(
    client_id: &str,
    product_id: &str,
    device_serial_number: &str,
) -> Result<CodePairResponse> {
    let http = reqwest::Client::new();

    let scope_data = serde_json::json!({
        "alexa:all": {
            "productID": product_id,
            "productInstanceAttributes": {
                "deviceSerialNumber": device_serial_number
            }
        }
    })
    .to_string();

    let resp = http
        .post(AMAZON_CODEPAIR_URL)
        .form(&[
            ("response_type", "device_code"),
            ("client_id", client_id),
            ("scope", OAUTH_SCOPES),
            ("scope_data", &scope_data),
        ])
        .send()
        .await
        .context("Failed to POST to Amazon device-code endpoint")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "Failed to request a device code (HTTP {}): {}",
            status.as_u16(),
            body
        );
    }

    serde_json::from_str(&body).context("Failed to parse device-code response from Amazon")
}

// ── Token polling ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
}

/// Outcome of a single poll of the token endpoint.
enum PollOutcome {
    Success(TokenResponse),
    Pending,
    SlowDown,
    Denied,
    Expired,
}

/// Classify a token-endpoint response by HTTP status and body.
fn classify_poll_response(status_success: bool, body: &str) -> Result<PollOutcome> {
    if status_success {
        let tokens: TokenResponse =
            serde_json::from_str(body).context("Failed to parse token response from Amazon")?;
        return Ok(PollOutcome::Success(tokens));
    }

    let err: TokenErrorResponse = serde_json::from_str(body)
        .with_context(|| format!("Unexpected error response from Amazon: {}", body))?;

    match err.error.as_str() {
        "authorization_pending" => Ok(PollOutcome::Pending),
        "slow_down" => Ok(PollOutcome::SlowDown),
        "access_denied" => Ok(PollOutcome::Denied),
        "expired_token" => Ok(PollOutcome::Expired),
        other => bail!("Amazon returned an unexpected error: {}", other),
    }
}

async fn poll_for_tokens(
    client_id: &str,
    device_code: &str,
    user_code: &str,
    interval: u64,
) -> Result<TokenResponse> {
    let http = reqwest::Client::new();
    let mut interval = Duration::from_secs(interval.max(1));

    loop {
        tokio::time::sleep(interval).await;

        let resp = http
            .post(AMAZON_TOKEN_URL)
            .form(&[
                ("grant_type", "device_code"),
                ("device_code", device_code),
                ("user_code", user_code),
                ("client_id", client_id),
            ])
            .send()
            .await
            .context("Failed to POST to Amazon token endpoint")?;

        let status_success = resp.status().is_success();
        let body = resp.text().await.unwrap_or_default();

        match classify_poll_response(status_success, &body)? {
            PollOutcome::Success(tokens) => return Ok(tokens),
            PollOutcome::Pending => continue,
            PollOutcome::SlowDown => {
                interval += Duration::from_secs(5);
                continue;
            }
            PollOutcome::Denied => bail!("Login was denied. Please try again."),
            PollOutcome::Expired => {
                bail!("The device code expired before login was completed. Please try again.")
            }
        }
    }
}

// ── Session establishment ─────────────────────────────────────────────────────

/// POST the access_token to Amazon's cookie-exchange endpoint to obtain
/// amazon.com session cookies, then follow through to alexa.amazon.com.
async fn establish_session_from_token(
    access_token: &str,
    cookie_store: Arc<CookieStoreMutex>,
    settings: &Settings,
) -> Result<()> {
    let client = build_client(Arc::clone(&cookie_store))?;

    eprintln!("Exchanging token for session cookies...");
    let resp = client
        .post(AMAZON_EXCHANGE_URL)
        .form(&[("access_token", access_token), ("profile", "alexa")])
        .send()
        .await
        .context("Failed to POST to Amazon token-exchange endpoint")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!(
            "Session cookie exchange failed (HTTP {}): {}",
            status.as_u16(),
            body
        );
    }

    // Follow through to alexa.amazon.com so it can set its own session cookies.
    eprintln!("Establishing Alexa session...");
    client
        .get(format!("{}/", settings.base_url))
        .send()
        .await
        .context("Failed to reach alexa.amazon.com after token exchange")?;

    Ok(())
}

// ── Store / retrieve refresh token ───────────────────────────────────────────

const KEYRING_SERVICE: &str = "alexa-cli";
const KEYRING_REFRESH_KEY: &str = "lwa-refresh-token";

pub fn store_refresh_token(token: &str) -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH_KEY) {
        entry
            .set_password(token)
            .context("Failed to store LWA refresh token in keyring")?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn load_refresh_token() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH_KEY)
        .ok()
        .and_then(|e| e.get_password().ok())
        .filter(|s| !s.is_empty())
}

pub fn clear_refresh_token() {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH_KEY) {
        let _ = entry.delete_credential();
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Full device-pairing (Code-Based Linking) login flow.
///
/// Displays a short code for the user to enter at https://amazon.com/code,
/// then polls Amazon until login completes. The CLI never sees the user's
/// password.
pub async fn device_code_login(
    client_id: &str,
    product_id: &str,
    device_serial_number: &str,
    cookie_store: Arc<CookieStoreMutex>,
    settings: &mut Settings,
) -> Result<()> {
    // 1. Request a code pair.
    let pair = request_code_pair(client_id, product_id, device_serial_number).await?;
    let verification_uri = pair.verification_uri.as_deref().unwrap_or(VERIFICATION_URI);

    // 2. Show the user the code.
    eprintln!();
    eprintln!("To sign in, visit:");
    eprintln!("  {}", verification_uri);
    eprintln!("and enter this code:");
    eprintln!();
    eprintln!("  {}", pair.user_code);
    eprintln!();
    if let Some(secs) = pair.expires_in {
        eprintln!("(this code expires in {} minutes)", secs / 60);
    }
    if let Err(e) = webbrowser::open(verification_uri) {
        eprintln!("(Could not open browser automatically: {})", e);
    }
    eprintln!("Waiting for you to approve the login...");

    // 3. Poll until the user approves (or the code expires).
    let interval = pair.interval.unwrap_or(5);
    let tokens = poll_for_tokens(client_id, &pair.device_code, &pair.user_code, interval).await?;

    // 4. Persist refresh token.
    if let Some(ref rt) = tokens.refresh_token {
        store_refresh_token(rt)?;
    }

    // 5. Exchange access_token for alexa.amazon.com session cookies.
    establish_session_from_token(&tokens.access_token, Arc::clone(&cookie_store), settings).await?;

    // 6. Persist cookies + settings.
    save_cookie_store(&cookie_store)?;

    // Derive email from token if not already set (best-effort; not critical).
    if settings.email.is_empty() {
        if let Ok(profile) = fetch_amazon_profile(&tokens.access_token).await {
            settings.set_email(&profile);
        }
    }

    // Record expiry from token lifetime (default 14 days if not provided).
    let days = tokens.expires_in.map(|s| s / 86400).unwrap_or(14).max(1);
    let expires = chrono::Utc::now() + chrono::Duration::days(days as i64);
    settings.cookie_expires_at = Some(expires.timestamp());
    settings.save()?;

    eprintln!("Login successful.");
    Ok(())
}

/// Best-effort fetch of the Amazon account email from the LWA profile endpoint.
async fn fetch_amazon_profile(access_token: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Profile {
        email: Option<String>,
    }
    let http = reqwest::Client::new();
    let resp = http
        .get("https://api.amazon.com/user/profile")
        .bearer_auth(access_token)
        .send()
        .await?;
    let profile: Profile = resp.json().await?;
    profile.email.context("Profile has no email field")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_poll_response_success() {
        let body = r#"{"access_token":"atza_x","refresh_token":"Atzr_y","expires_in":3600}"#;
        let outcome = classify_poll_response(true, body).expect("should parse");
        match outcome {
            PollOutcome::Success(tokens) => {
                assert_eq!(tokens.access_token, "atza_x");
                assert_eq!(tokens.refresh_token.as_deref(), Some("Atzr_y"));
                assert_eq!(tokens.expires_in, Some(3600));
            }
            _ => panic!("expected Success outcome"),
        }
    }

    #[test]
    fn test_classify_poll_response_pending() {
        let body = r#"{"error":"authorization_pending"}"#;
        let outcome = classify_poll_response(false, body).expect("should parse");
        assert!(matches!(outcome, PollOutcome::Pending));
    }

    #[test]
    fn test_classify_poll_response_slow_down() {
        let body = r#"{"error":"slow_down"}"#;
        let outcome = classify_poll_response(false, body).expect("should parse");
        assert!(matches!(outcome, PollOutcome::SlowDown));
    }

    #[test]
    fn test_classify_poll_response_denied() {
        let body = r#"{"error":"access_denied"}"#;
        let outcome = classify_poll_response(false, body).expect("should parse");
        assert!(matches!(outcome, PollOutcome::Denied));
    }

    #[test]
    fn test_classify_poll_response_expired() {
        let body = r#"{"error":"expired_token"}"#;
        let outcome = classify_poll_response(false, body).expect("should parse");
        assert!(matches!(outcome, PollOutcome::Expired));
    }

    #[test]
    fn test_classify_poll_response_unexpected_error_bails() {
        let body = r#"{"error":"invalid_client"}"#;
        let result = classify_poll_response(false, body);
        assert!(result.is_err());
    }

    #[test]
    fn test_classify_poll_response_malformed_body_bails() {
        let result = classify_poll_response(false, "not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_codepair_response_parses_minimal_fields() {
        let body = r#"{"user_code":"ABCD1234","device_code":"dev_xyz"}"#;
        let pair: CodePairResponse = serde_json::from_str(body).expect("should parse");
        assert_eq!(pair.user_code, "ABCD1234");
        assert_eq!(pair.device_code, "dev_xyz");
        assert!(pair.verification_uri.is_none());
        assert!(pair.interval.is_none());
        assert!(pair.expires_in.is_none());
    }

    #[test]
    fn test_codepair_response_parses_all_fields() {
        let body = r#"{
            "user_code":"ABCD1234",
            "device_code":"dev_xyz",
            "verification_uri":"https://amazon.com/code",
            "interval":5,
            "expires_in":600
        }"#;
        let pair: CodePairResponse = serde_json::from_str(body).expect("should parse");
        assert_eq!(
            pair.verification_uri.as_deref(),
            Some("https://amazon.com/code")
        );
        assert_eq!(pair.interval, Some(5));
        assert_eq!(pair.expires_in, Some(600));
    }
}
