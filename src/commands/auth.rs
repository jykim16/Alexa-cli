use anyhow::{Context, Result};

use crate::auth::{clear_cookie_store, load_cookie_store, login, save_cookie_store};
use crate::auth::login::build_client;
use crate::auth::fetch_csrf;
use crate::cli::OutputFormat;
use crate::config::Settings;

pub async fn cmd_login(email: &str, output: OutputFormat) -> Result<()> {
    let mut settings = Settings::load()?;

    // Try refresh token if available
    if settings.refresh_token.is_some() {
        settings.set_email(email);
        match crate::auth::cbl::refresh_login(&mut settings).await {
            Ok(()) => {
                match output {
                    OutputFormat::Json => println!("{{\"status\":\"authenticated\",\"email\":\"{}\"}}", email),
                    _ => println!("Logged in as {} (refreshed token)", email),
                }
                return Ok(());
            }
            Err(e) => eprintln!("Token refresh failed ({}), prompting for password...", e),
        }
    }

    // Primary: Browser-based login

    crate::auth::device_login::login(email, "", &mut settings).await?;

    match output {
        OutputFormat::Json => println!("{{\"status\":\"authenticated\",\"email\":\"{}\"}}", email),
        _ => println!("Logged in as {}", email),
    }
    Ok(())
}

pub async fn cmd_import_cookies(output: OutputFormat) -> Result<()> {
    use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
    use std::sync::Arc;

    eprintln!("Open https://alexa.amazon.com in your browser and log in.");
    eprintln!("Then open DevTools (F12) → Console, and run:");
    eprintln!("  document.cookie");
    eprintln!("Paste the output below (the full cookie string):");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).context("Failed to read input")?;
    let cookie_str = input.trim();

    if cookie_str.is_empty() {
        anyhow::bail!("No cookies provided");
    }

    // Parse cookie string into the cookie store
    let store = CookieStore::default();
    let store = Arc::new(CookieStoreMutex::new(store));
    {
        let mut s = store.lock().unwrap();
        let alexa_url = url::Url::parse("https://alexa.amazon.com/").unwrap();
        let amazon_url = url::Url::parse("https://www.amazon.com/").unwrap();
        for pair in cookie_str.split(';') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            // Set on alexa.amazon.com
            let set_cookie = format!("{}; Domain=.amazon.com; Path=/; Secure", pair);
            let _ = s.parse(&set_cookie, &alexa_url);
            // Also set directly for alexa subdomain
            let set_cookie2 = format!("{}; Domain=alexa.amazon.com; Path=/; Secure", pair);
            let _ = s.parse(&set_cookie2, &alexa_url);
            let _ = s.parse(&set_cookie, &amazon_url);
        }
    }

    save_cookie_store(&store)?;

    // Verify it works
    let http = build_client(Arc::clone(&store))?;
    let settings = Settings::load()?;
    match fetch_csrf(&http, &settings.base_url).await {
        Ok(_) => {
            let mut settings = settings;
            settings.mark_authenticated();
            settings.save()?;
            match output {
                OutputFormat::Json => println!("{{\"status\":\"authenticated\"}}"),
                _ => println!("Cookies imported and verified. Session is valid."),
            }
        }
        Err(_) => {
            // Still save — behaviors API may work even without CSRF
            let mut settings = settings;
            settings.mark_authenticated();
            settings.save()?;
            match output {
                OutputFormat::Json => println!("{{\"status\":\"partial\"}}"),
                _ => {
                    println!("Cookies imported. CSRF validation failed but some commands may still work.");
                    println!("Try: alexa-cli speak say \"test\" --device \"Big Show\"");
                }
            }
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
