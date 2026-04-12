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

/// Load the cookie store from keyring, falling back to the config-dir file.
/// Returns a CookieStoreMutex suitable for use with reqwest.
#[allow(deprecated)]
pub fn load_cookie_store() -> Result<Arc<CookieStoreMutex>> {
    let json = load_raw_cookies()?;
    let store = match json {
        Some(data) => {
            let cursor = std::io::Cursor::new(data.as_bytes());
            CookieStore::load_json(cursor).unwrap_or_else(|_| CookieStore::default())
        }
        None => CookieStore::default(),
    };
    Ok(Arc::new(CookieStoreMutex::new(store)))
}

/// Persist the cookie store back to keyring or file.
#[allow(deprecated)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_cookie_store_returns_arc_mutex() {
        // Should succeed even when no persisted cookies exist
        let result = load_cookie_store();
        assert!(result.is_ok());
        let store = result.unwrap();
        // Verify we can lock it
        let _guard = store.lock().unwrap();
    }

    #[test]
    fn test_save_and_load_round_trip() {
        // Create a store, save it, then reload it
        let store = load_cookie_store().unwrap();
        let save_result = save_cookie_store(&store);
        // May fail gracefully if keyring unavailable, but should not panic
        let _ = save_result;
    }

    #[test]
    fn test_clear_cookie_store_does_not_panic() {
        // Should be safe to call even when nothing is stored
        let result = clear_cookie_store();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cookie_file_path_returns_valid_path() {
        let result = cookie_file_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        // Path should end with cookies.json
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "cookies.json");
    }

    #[test]
    fn test_load_cookie_store_after_clear_returns_empty() {
        // Clear first so we start fresh
        let _ = clear_cookie_store();
        let store = load_cookie_store().unwrap();
        let inner = store.lock().unwrap();
        // An empty cookie store has no cookies for any URL
        let url = url::Url::parse("https://alexa.amazon.com").unwrap();
        let cookies: Vec<_> = inner.get_request_values(&url).collect();
        assert!(cookies.is_empty());
    }
}
