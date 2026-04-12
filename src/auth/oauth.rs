//! Browser-based OAuth 2.0 Authorization Code + PKCE login flow (RFC 7636 / RFC 8252).
//!
//! Flow:
//!   1. Generate a random PKCE code_verifier and derive the code_challenge.
//!   2. Bind a local TCP listener on 127.0.0.1 (random port).
//!   3. Open the system browser to Amazon's authorization URL.
//!   4. Wait for Amazon to redirect back to http://127.0.0.1:<port>/callback?code=...
//!   5. Exchange the authorization code + code_verifier for tokens.
//!   6. POST the access_token to Amazon's token-exchange endpoint to obtain
//!      amazon.com session cookies, then follow the redirect to alexa.amazon.com
//!      to finalize the Alexa session.

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use reqwest_cookie_store::CookieStoreMutex;

use super::cookie_store::save_cookie_store;
use super::login::build_client;
use crate::config::Settings;

// ── Amazon OAuth endpoints ────────────────────────────────────────────────────

const AMAZON_AUTH_URL: &str = "https://www.amazon.com/ap/oa";
const AMAZON_TOKEN_URL: &str = "https://api.amazon.com/auth/o2/token";
/// Exchanges a web access_token for amazon.com session cookies.
const AMAZON_EXCHANGE_URL: &str = "https://www.amazon.com/ap/exchangetoken/sidebar";

/// Scopes needed to access alexa.amazon.com internal APIs.
/// `profile` establishes the identity; `alexa:all` covers device/behavior APIs.
const OAUTH_SCOPES: &str = "profile%20alexa%3Aall";

// ── PKCE helpers ─────────────────────────────────────────────────────────────

/// Generate a cryptographically random PKCE code_verifier (43–128 chars, base64url, no padding).
fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Derive the code_challenge from a code_verifier: BASE64URL(SHA256(verifier)).
fn derive_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

// ── Callback server ───────────────────────────────────────────────────────────

/// Bind on 127.0.0.1:0, open the browser, then accept the first request and
/// extract the `code` query parameter from the redirect URL.
async fn wait_for_callback(listener: TcpListener, auth_url: &str) -> Result<String> {
    // Open the browser after the listener is ready so we don't miss a fast redirect.
    eprintln!("Opening browser for Amazon login...");
    eprintln!("If the browser does not open automatically, visit:");
    eprintln!("  {}", auth_url);

    if let Err(e) = webbrowser::open(auth_url) {
        eprintln!("Warning: could not open browser automatically ({})", e);
    }

    // Accept exactly one connection — the OAuth callback redirect.
    let (stream, _) = listener
        .accept()
        .await
        .context("Failed to accept OAuth callback connection")?;

    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .context("Failed to read callback request")?;

    // Request line looks like: GET /callback?code=XXXX&state=YYY HTTP/1.1
    let code = request_line
        .split_whitespace()
        .nth(1) // path + query
        .and_then(|path| {
            url::Url::parse(&format!("http://localhost{}", path))
                .ok()
                .and_then(|u| {
                    u.query_pairs()
                        .find(|(k, _)| k == "code")
                        .map(|(_, v)| v.into_owned())
                })
        })
        .context(
            "Amazon did not return an authorization code. \
             Check that your LWA redirect URI is configured correctly.",
        )?;

    // Respond to the browser so the tab shows a success message.
    let body = "\
        <!doctype html><html><body>\
        <h2>Login successful</h2>\
        <p>You can close this tab and return to the terminal.</p>\
        </body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    // Best-effort write; ignore errors (tab might already be closed).
    let _ = reader.get_mut().write_all(response.as_bytes()).await;

    Ok(code)
}

// ── Token exchange ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

async fn exchange_code_for_tokens(
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<TokenResponse> {
    let http = reqwest::Client::new();

    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("code_verifier", code_verifier),
    ];
    let secret_owned;
    if let Some(s) = client_secret {
        secret_owned = s.to_string();
        params.push(("client_secret", &secret_owned));
    }

    let resp = http
        .post(AMAZON_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .context("Failed to POST to Amazon token endpoint")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("Token exchange failed (HTTP {}): {}", status.as_u16(), body);
    }

    serde_json::from_str(&body).context("Failed to parse token response from Amazon")
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
            "Session cookie exchange failed (HTTP {}): {}\n\
             Ensure your LWA Security Profile has 'alexa:all' in its allowed scopes \
             and that the redirect URI is set to http://127.0.0.1",
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

/// Full browser-based PKCE login flow.
///
/// Opens the system browser, lets the user authenticate through Amazon's
/// official login UI (the CLI never sees the password), then exchanges the
/// resulting tokens for alexa.amazon.com session cookies.
pub async fn browser_login(
    client_id: &str,
    client_secret: Option<&str>,
    cookie_store: Arc<CookieStoreMutex>,
    settings: &mut Settings,
) -> Result<()> {
    // 1. PKCE
    let code_verifier = generate_code_verifier();
    let code_challenge = derive_code_challenge(&code_verifier);

    // 2. Bind local listener
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("Failed to bind local OAuth callback server")?;
    let port = listener
        .local_addr()
        .context("Failed to get local address")?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    // 3. Build authorization URL
    let state = URL_SAFE_NO_PAD.encode({
        let mut b = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut b);
        b
    });
    let auth_url = format!(
        "{}?client_id={}&scope={}&response_type=code\
         &redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        AMAZON_AUTH_URL,
        urlencoding::encode(client_id),
        OAUTH_SCOPES,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&code_challenge),
        urlencoding::encode(&state),
    );

    // 4. Wait for callback (opens browser inside)
    let code = wait_for_callback(listener, &auth_url).await?;

    // 5. Exchange code → tokens
    eprintln!("Exchanging authorization code for tokens...");
    let tokens = exchange_code_for_tokens(
        client_id,
        client_secret,
        &code,
        &redirect_uri,
        &code_verifier,
    )
    .await?;

    // 6. Persist refresh token
    if let Some(ref rt) = tokens.refresh_token {
        store_refresh_token(rt)?;
    }

    // 7. Exchange access_token for alexa.amazon.com session cookies
    establish_session_from_token(&tokens.access_token, Arc::clone(&cookie_store), settings).await?;

    // 8. Persist cookies + settings
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
    fn test_code_verifier_is_43_plus_chars() {
        let v = generate_code_verifier();
        // RFC 7636: code_verifier must be 43-128 characters
        assert!(v.len() >= 43, "verifier too short: {}", v.len());
        assert!(v.len() <= 128, "verifier too long: {}", v.len());
    }

    #[test]
    fn test_code_verifier_is_base64url_chars_only() {
        let v = generate_code_verifier();
        assert!(
            v.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "verifier contains invalid chars: {}",
            v
        );
    }

    #[test]
    fn test_code_challenge_is_base64url_chars_only() {
        let v = generate_code_verifier();
        let c = derive_code_challenge(&v);
        assert!(
            c.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "challenge contains invalid chars: {}",
            c
        );
    }

    #[test]
    fn test_code_challenge_differs_from_verifier() {
        let v = generate_code_verifier();
        let c = derive_code_challenge(&v);
        assert_ne!(v, c);
    }

    #[test]
    fn test_code_challenge_is_deterministic() {
        let v = generate_code_verifier();
        assert_eq!(derive_code_challenge(&v), derive_code_challenge(&v));
    }

    #[test]
    fn test_known_pkce_vector() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(derive_code_challenge(verifier), expected_challenge);
    }
}
