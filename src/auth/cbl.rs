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
const DEFAULT_CLIENT_ID: &str = "amzn1.application-oa2-client.75ecd91677d949f8b473891703fe167b";

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
        .or_else(|| Some(DEFAULT_CLIENT_ID.to_string()))
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());

    let product_id = settings.avs_product_id.clone().unwrap_or_else(|| "Echo".to_string());
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
            ("client_id", client_id.as_str()),
            ("scope", SCOPES),
            ("scope_data", scope_data.as_str()),
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

    // Step 5: Exchange access token for Alexa website cookies
    establish_alexa_session(&tokens.access_token, settings).await?;

    settings.mark_authenticated();
    settings.save()?;

    eprintln!("Login successful.");
    Ok(())
}

/// Exchange the LWA access token for actual Alexa website session cookies.
async fn establish_alexa_session(access_token: &str, settings: &Settings) -> Result<()> {
    use crate::auth::cookie_store::cookie_file_path;
    use crate::auth::login::build_client;
    use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
    use std::sync::Arc;

    let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
    let client = build_client(Arc::clone(&cookie_store))?;

    // Try exchanging token for cookies via Amazon's token exchange
    let mut form_params = vec![
        ("access_token".to_string(), access_token.to_string()),
        ("requested_token_type".to_string(), "auth_cookies".to_string()),
        ("domain".to_string(), ".amazon.com".to_string()),
        ("source_token_type".to_string(), "access_token".to_string()),
        ("source_token".to_string(), access_token.to_string()),
    ];

    if let Some(ref secret) = settings.lwa_client_secret {
        form_params.push(("client_id".to_string(), settings.lwa_client_id.clone().unwrap_or_default()));
        form_params.push(("client_secret".to_string(), secret.clone()));
    }

    let resp = client
        .post("https://www.amazon.com/ap/exchangetoken/cookies")
        .form(&form_params)
        .send()
        .await
        .context("Failed to exchange token for cookies")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        eprintln!("[debug] Cookie exchange returned {} - {}", status, &body[..body.len().min(200)]);
        // Fall back: use access token directly as at-main
        write_token_cookies(access_token, settings)?;
        return Ok(());
    }

    // Parse the exchange response - it returns cookies in JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(cookies) = v.pointer("/response/tokens/cookies") {
            // Extract cookies from all domains
            let mut cookie_lines = Vec::new();
            if let Some(obj) = cookies.as_object() {
                for (_domain, domain_cookies) in obj {
                    if let Some(arr) = domain_cookies.as_array() {
                        for c in arr {
                            let name = c.get("Name").and_then(|v| v.as_str()).unwrap_or("");
                            let value = c.get("Value").and_then(|v| v.as_str()).unwrap_or("");
                            if !name.is_empty() {
                                cookie_lines.push(format!(
                                    "{{\"raw_cookie\":\"{name}={value}; Secure; Path=/; Domain=.amazon.com\",\"path\":[\"/\",true],\"domain\":{{\"Suffix\":\"amazon.com\"}},\"expires\":{{\"AtUtc\":\"2036-01-01T08:00:01Z\"}}}}"
                                ));
                            }
                        }
                    }
                }
            }
            if !cookie_lines.is_empty() {
                let path = cookie_file_path()?;
                std::fs::write(&path, cookie_lines.join("\n"))
                    .context("Failed to write cookie file")?;
                return Ok(());
            }
        }
    }

    // Fallback
    write_token_cookies(access_token, settings)?;
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
    let client_id = settings.lwa_client_id.clone()
        .or_else(|| Some(DEFAULT_CLIENT_ID.to_string()))
        .context("lwa_client_id not set")?;
    let refresh_token = settings.refresh_token.clone()
        .context("No refresh token stored. Run device code login first.")?;

    let http = reqwest::Client::new();

    // Device clients don't use client_secret for refresh
    let mut params = vec![
        ("grant_type".to_string(), "refresh_token".to_string()),
        ("refresh_token".to_string(), refresh_token),
        ("client_id".to_string(), client_id),
    ];
    if let Some(ref secret) = settings.lwa_client_secret {
        params.push(("client_secret".to_string(), secret.clone()));
    }

    let resp = http
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .context("Failed to refresh token")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("Token refresh failed ({}): {}", status, &body[..body.len().min(200)]);
    }

    let tokens: TokenResponse = serde_json::from_str(&body)
        .context("Failed to parse refresh response")?;

    if let Some(ref rt) = tokens.refresh_token {
        settings.refresh_token = Some(rt.clone());
    }

    establish_alexa_session(&tokens.access_token, settings).await?;
    settings.mark_authenticated();
    settings.save()?;

    Ok(())
}
