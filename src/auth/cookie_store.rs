use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use reqwest_cookie_store::{CookieStore, CookieStoreMutex};

use crate::config::Settings;

const KEYRING_SERVICE: &str = "alexa-cli";
const KEYRING_COOKIE_KEY: &str = "cookies-v1";

/// Returns the fallback cookie file path (chmod 0600)
pub fn cookie_file_path() -> Result<PathBuf> {
    let dir = Settings::config_dir()?;
    Ok(dir.join("cookies.json"))
}

/// Load cookies as a raw "name=value; name=value" string for direct header injection.
pub fn load_raw_cookie_string() -> Result<String> {
    let path = cookie_file_path()?;
    if !path.exists() {
        return Ok(String::new());
    }
    let data = fs::read_to_string(&path).unwrap_or_default();
    let mut pairs = Vec::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Extract "name=value" from raw_cookie field
        if let Some(start) = line.find("\"raw_cookie\":\"") {
            let rest = &line[start + 14..];
            if let Some(end) = rest.find(';') {
                pairs.push(rest[..end].to_string());
            } else if let Some(end) = rest.find('"') {
                pairs.push(rest[..end].to_string());
            }
        }
    }
    Ok(pairs.join("; "))
}

/// Load the cookie store from keyring, falling back to the config-dir file.
/// Returns a CookieStoreMutex suitable for use with reqwest.
pub fn load_cookie_store() -> Result<Arc<CookieStoreMutex>> {
    let json = load_raw_cookies()?;
    let store = match json {
        Some(data) => {
            let cursor = std::io::Cursor::new(data.as_bytes());
            CookieStore::load_json(cursor)
                .unwrap_or_else(|_| CookieStore::default())
        }
        None => CookieStore::default(),
    };
    Ok(Arc::new(CookieStoreMutex::new(store)))
}

/// Persist the cookie store back to keyring or file.
pub fn save_cookie_store(store: &Arc<CookieStoreMutex>) -> Result<()> {
    let mut buf = Vec::new();
    {
        let store = store.lock().unwrap();
        store
            .save_json(&mut buf)
            .map_err(|e| anyhow::anyhow!("Failed to serialize cookies: {}", e))?;
    }
    let json = String::from_utf8(buf).context("Cookie JSON is not valid UTF-8")?;
    save_raw_cookies(&json)
}

/// Clear all stored cookies.
pub fn clear_cookie_store() -> Result<()> {
    // Try keyring first
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_COOKIE_KEY);
    if let Ok(e) = entry {
        let _ = e.delete_credential();
    }
    // Also remove file fallback
    let path = cookie_file_path()?;
    if path.exists() {
        fs::remove_file(&path).ok();
    }
    Ok(())
}

fn load_raw_cookies() -> Result<Option<String>> {
    // Try keyring first
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_COOKIE_KEY) {
        match entry.get_password() {
            Ok(data) if !data.is_empty() => return Ok(Some(data)),
            _ => {}
        }
    }
    // Fall back to file
    let path = cookie_file_path()?;
    if path.exists() {
        let data = fs::read_to_string(&path).context("Failed to read cookie file")?;
        if !data.is_empty() {
            return Ok(Some(data));
        }
    }
    Ok(None)
}

fn save_raw_cookies(json: &str) -> Result<()> {
    // Try keyring first
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_COOKIE_KEY) {
        if entry.set_password(json).is_ok() {
            return Ok(());
        }
    }
    // Fall back to file with restricted permissions
    let path = cookie_file_path()?;
    fs::write(&path, json).context("Failed to write cookie file")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .context("Failed to set cookie file permissions")?;
    eprintln!(
        "Warning: keyring unavailable. Cookies stored in {} (mode 0600)",
        path.display()
    );
    Ok(())
}
