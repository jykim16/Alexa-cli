use anyhow::{bail, Result};

use crate::auth::fetch_csrf;
use crate::auth::login::build_client;
use crate::auth::{
    browser_login, clear_cookie_store, clear_refresh_token, load_cookie_store, login,
};
use crate::cli::OutputFormat;
use crate::config::Settings;

/// `alexa-cli auth login [--email <email>]`
///
/// When `lwa_client_id` is configured → browser-based OAuth PKCE (no password in CLI).
/// Otherwise → form-based login (email + password prompted in terminal).
pub async fn cmd_login(email: Option<&str>, output: OutputFormat) -> Result<()> {
    let mut settings = Settings::load()?;
    let cookie_store = load_cookie_store()?;

    let logged_in_email = if let Some(client_id) = settings.lwa_client_id.clone() {
        // ── Browser-based OAuth PKCE ─────────────────────────────────────
        let client_secret = settings.lwa_client_secret.clone();
        browser_login(
            &client_id,
            client_secret.as_deref(),
            cookie_store,
            &mut settings,
        )
        .await?;
        settings.email.clone()
    } else {
        // ── Form-based fallback ──────────────────────────────────────────
        let email = email.unwrap_or("").to_string();
        if email.is_empty() {
            bail!(
                "Email is required for form-based login.\n\
                 Usage: alexa-cli auth login --email you@example.com\n\n\
                 For a more secure browser-based login (no password in terminal), \
                 set `lwa_client_id` in your config file (~/.config/alexa-cli/config.toml).\n\
                 Register a free Security Profile at:\n  \
                 https://developer.amazon.com/loginwithamazon/console/site/lwa/overview.html"
            );
        }
        let password = rpassword::prompt_password("Amazon password: ")?;
        login(&email, &password, cookie_store, &mut settings).await?;
        email
    };

    match output {
        OutputFormat::Json => {
            println!(
                "{{\"status\":\"authenticated\",\"email\":\"{}\"}}",
                logged_in_email
            );
        }
        _ => println!("Logged in as {}", logged_in_email),
    }
    Ok(())
}

pub async fn cmd_logout(output: OutputFormat) -> Result<()> {
    clear_cookie_store()?;
    clear_refresh_token();

    let mut settings = Settings::load()?;
    settings.cookie_expires_at = None;
    settings.save()?;

    match output {
        OutputFormat::Json => println!("{{\"status\":\"logged_out\"}}"),
        _ => println!("Logged out. Cookies and tokens cleared."),
    }
    Ok(())
}

pub async fn cmd_status(output: OutputFormat) -> Result<()> {
    let settings = Settings::load()?;

    let email = if settings.email.is_empty() {
        "not set".to_string()
    } else {
        settings.email.clone()
    };

    let cookie_status = if settings.cookie_expires_at.is_none() {
        "not authenticated"
    } else if settings.is_cookie_expired() {
        "expired"
    } else {
        "active"
    };

    let expires = settings
        .cookie_expires_at
        .map(|ts| {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string())
        })
        .unwrap_or_else(|| "n/a".to_string());

    let auth_method = if settings.lwa_client_id.is_some() {
        "oauth-pkce"
    } else {
        "form"
    };

    // Try to verify session live
    let session_valid = if cookie_status == "active" {
        let cookie_store = load_cookie_store()?;
        if let Ok(client) = build_client(cookie_store) {
            fetch_csrf(&client, &settings.base_url).await.is_ok()
        } else {
            false
        }
    } else {
        false
    };

    match output {
        OutputFormat::Json => {
            println!(
                "{{\"email\":\"{}\",\"cookieStatus\":\"{}\",\"expires\":\"{}\",\
                 \"sessionValid\":{},\"authMethod\":\"{}\"}}",
                email, cookie_status, expires, session_valid, auth_method
            );
        }
        _ => {
            crate::cli::output::print_pairs(&[
                ("Email", email),
                ("Auth method", auth_method.to_string()),
                ("Cookie status", cookie_status.to_string()),
                ("Expires", expires),
                ("Session valid", session_valid.to_string()),
            ]);
        }
    }
    Ok(())
}
