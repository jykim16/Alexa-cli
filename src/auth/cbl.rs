//! Device Authorization Grant / Amazon "Code-Based Linking" (CBL) login flow.
//! The user visits amazon.com/code, enters a short code, and the CLI gets an access token.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::time::Duration;

use crate::auth::cookie_store::cookie_file_path;
use crate::config::Settings;

const CODEPAIR_URL: &str = "https://api.amazon.com/auth/O2/create/codepair";
const TOKEN_URL: &str = "https://api.amazon.com/auth/o2/token";
const SCOPES: &str = "alexa:all";

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

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
}

pub async fn device_code_login(settings: &mut Settings) -> Result<()> {
    let client_id = settings.lwa_client_id.clone()
        .context("lwa_client_id not set in config. Register at https://developer.amazon.com/loginwithamazon/console/site/lwa/overview.html and add it to your config.toml")?;

    let product_id = settings.avs_product_id.clone().unwrap_or_else(|| "alexa_cli".to_string());
    let serial = settings.ensure_device_serial_number()?;

    // Step 1: Request code pair
    let scope_data = serde_json::json!({
        "alexa:all": {
            "productID": product_id,
            "productInstanceAttributes": {
                "deviceSerialNumber": serial
            }
        }
    }).to_string();

    let http = reqwest::Client::new();
    let resp = http
        .post(CODEPAIR_URL)
        .form(&[
            ("response_type", "device_code"),
            ("client_id", &client_id),
            ("scope", SCOPES),
            ("scope_data", &scope_data),
        ])
        .send()
        .await
        .context("Failed to request device code")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("Device code request failed ({}): {}", status, body);
    }

    let pair: CodePairResponse = serde_json::from_str(&body)
        .context("Failed to parse device code response")?;

    let verification_uri = pair.verification_uri.as_deref().unwrap_or("https://amazon.com/code");

    // Step 2: Show code to user
    eprintln!();
    eprintln!("Go to: {}", verification_uri);
    eprintln!("Enter code: {}", pair.user_code);
    eprintln!();
    if let Some(secs) = pair.expires_in {
        eprintln!("(expires in {} minutes)", secs / 60);
    }
    eprintln!("Waiting for approval...");

    // Step 3: Poll for token
    let interval = Duration::from_secs(pair.interval.unwrap_or(5).max(1));
    let tokens = loop {
        tokio::time::sleep(interval).await;

        let resp = http
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "device_code"),
                ("device_code", &pair.device_code),
                ("user_code", &pair.user_code),
                ("client_id", &client_id),
            ])
            .send()
            .await
            .context("Failed to poll token endpoint")?;

        let ok = resp.status().is_success();
        let body = resp.text().await.unwrap_or_default();

        if ok {
            let tokens: TokenResponse = serde_json::from_str(&body)
                .context("Failed to parse token response")?;
            break tokens;
        }

        let err: TokenErrorResponse = serde_json::from_str(&body)
            .unwrap_or(TokenErrorResponse { error: "unknown".to_string() });

        match err.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            "access_denied" => bail!("Login was denied."),
            "expired_token" => bail!("Code expired. Please try again."),
            other => bail!("Unexpected error: {}", other),
        }
    };

    // Step 4: Store refresh token
    if let Some(ref rt) = tokens.refresh_token {
        if let Ok(entry) = keyring::Entry::new("alexa-cli", "lwa-refresh-token") {
            let _ = entry.set_password(rt);
        }
        // Also save to settings for fallback
        settings.refresh_token = Some(rt.clone());
    }

    // Step 5: Write access token as at-main cookie + get csrf
    // The access_token IS the at-main value
    write_token_cookies(&tokens.access_token, settings)?;

    settings.mark_authenticated();
    settings.save()?;

    eprintln!("Login successful.");
    Ok(())
}

/// Write the access token as at-main cookie in the cookie file format.
fn write_token_cookies(access_token: &str, settings: &Settings) -> Result<()> {
    let cookies = vec![
        format!("at-main={}", access_token),
        "ubid-main=000-0000000-0000000".to_string(),
        "csrf=0".to_string(),
        "session-id=000-0000000-0000000".to_string(),
        format!("lc-main={}", settings.locale.replace('-', "_")),
    ];

    let lines: Vec<String> = cookies.iter().map(|c| {
        format!(
            "{{\"raw_cookie\":\"{c}; Secure; Path=/; Domain=.amazon.com\",\"path\":[\"/\",true],\"domain\":{{\"Suffix\":\"amazon.com\"}},\"expires\":{{\"AtUtc\":\"2036-01-01T08:00:01Z\"}}}}"
        )
    }).collect();

    let path = cookie_file_path()?;
    std::fs::write(&path, lines.join("\n"))
        .context("Failed to write cookie file")?;
    Ok(())
}

/// Refresh the access token using a stored refresh token.
pub async fn refresh_login(settings: &mut Settings) -> Result<()> {
    let client_id = settings.lwa_client_id.as_deref()
        .context("lwa_client_id not set")?;

    let refresh_token = settings.refresh_token.as_deref()
        .or_else(|| {
            keyring::Entry::new("alexa-cli", "lwa-refresh-token")
                .ok()
                .and_then(|e| e.get_password().ok())
                .as_deref()
                .map(|_| "") // placeholder - need to handle this differently
        })
        .context("No refresh token stored. Run `auth login` first.")?;

    // Try keyring first
    let rt = if refresh_token.is_empty() {
        keyring::Entry::new("alexa-cli", "lwa-refresh-token")
            .ok()
            .and_then(|e| e.get_password().ok())
            .context("No refresh token in keyring")?
    } else {
        refresh_token.to_string()
    };

    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt),
            ("client_id", &client_id),
        ])
        .send()
        .await
        .context("Failed to refresh token")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("Token refresh failed ({}): {}", status, body);
    }

    let tokens: TokenResponse = serde_json::from_str(&body)
        .context("Failed to parse refresh response")?;

    write_token_cookies(&tokens.access_token, settings)?;
    settings.mark_authenticated();
    settings.save()?;

    Ok(())
}
