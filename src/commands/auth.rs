use anyhow::Result;

use crate::auth::fetch_csrf;
use crate::auth::login::build_client;
use crate::auth::{clear_cookie_store, load_cookie_store, login};
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_login(email: &str, output: OutputFormat) -> Result<()> {
    let password = rpassword::prompt_password("Amazon password: ")?;

    let mut settings = Settings::load()?;
    let cookie_store = load_cookie_store()?;

    login(email, &password, cookie_store, &mut settings).await?;

    match output {
        OutputFormat::Json => {
            println!("{{\"status\":\"authenticated\",\"email\":\"{}\"}}", email);
        }
        _ => {
            println!("Logged in as {}", email);
        }
    }
    Ok(())
}

pub async fn cmd_logout(output: OutputFormat) -> Result<()> {
    clear_cookie_store()?;

    let mut settings = Settings::load()?;
    settings.cookie_expires_at = None;
    settings.save()?;

    match output {
        OutputFormat::Json => println!("{{\"status\":\"logged_out\"}}"),
        _ => println!("Logged out. Cookies cleared."),
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
                "{{\"email\":\"{}\",\"cookieStatus\":\"{}\",\"expires\":\"{}\",\"sessionValid\":{}}}",
                email, cookie_status, expires, session_valid
            );
        }
        _ => {
            crate::cli::output::print_pairs(&[
                ("Email", email),
                ("Cookie status", cookie_status.to_string()),
                ("Expires", expires),
                ("Session valid", session_valid.to_string()),
            ]);
        }
    }
    Ok(())
}
