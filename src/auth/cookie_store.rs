use anyhow::Result;
use std::fs;
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

/// Load an empty cookie store for reqwest client building.
pub fn load_cookie_store() -> Result<Arc<CookieStoreMutex>> {
    Ok(Arc::new(CookieStoreMutex::new(CookieStore::default())))
}

/// Clear all stored cookies.
pub fn clear_cookie_store() -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_COOKIE_KEY) {
        let _ = entry.delete_credential();
    }
    let path = cookie_file_path()?;
    if path.exists() {
        fs::remove_file(&path).ok();
    }
    Ok(())
}
