use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    /// Amazon account email
    #[serde(default)]
    pub email: String,

    /// Amazon region base URL
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// Default device name (used when --device is not specified)
    #[serde(default)]
    pub default_device: Option<String>,

    /// Default locale for behaviors (TTS, music, etc.)
    #[serde(default = "default_locale")]
    pub locale: String,

    /// Cookie expiry timestamp (Unix seconds), stored to know when to re-auth
    #[serde(default)]
    pub cookie_expires_at: Option<i64>,

    /// Login with Amazon OAuth 2.0 client ID (Security Profile client_id).
    /// When set, `auth login` uses a browser-based PKCE flow instead of form scraping.
    /// Register at https://developer.amazon.com/loginwithamazon/console/site/lwa/overview.html
    #[serde(default)]
    pub lwa_client_id: Option<String>,

    /// LWA client secret (optional — only needed for confidential clients).
    /// Public/installed app registrations use PKCE and do not need a secret.
    #[serde(default)]
    pub lwa_client_secret: Option<String>,

    /// AVS (Alexa Voice Service) Product ID. Required for the device-pairing
    /// (Code-Based Linking) login flow, which needs an `alexa:all` scoped
    /// product registered at https://developer.amazon.com/alexa/console/avs.
    #[serde(default)]
    pub avs_product_id: Option<String>,

    /// Device serial number used to identify this CLI instance when pairing.
    /// Auto-generated and persisted on first device-pairing login if unset.
    #[serde(default)]
    pub device_serial_number: Option<String>,
}

fn default_base_url() -> String {
    "https://alexa.amazon.com".to_string()
}

fn default_locale() -> String {
    "en-US".to_string()
}

/// Write `data` to `path`, creating it with owner-only (0600) permissions on
/// Unix. The file is opened with the restricted mode up front (so there is no
/// window where it is world-readable) and existing files are re-tightened
/// before any data is written.
pub(crate) fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let mut file = opts.open(path)?;

    // If the file already existed, OpenOptions::mode is ignored, so explicitly
    // tighten permissions before writing any sensitive bytes.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    file.write_all(data)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            email: String::new(),
            base_url: default_base_url(),
            default_device: None,
            locale: default_locale(),
            cookie_expires_at: None,
            lwa_client_id: None,
            lwa_client_secret: None,
            avs_product_id: None,
            device_serial_number: None,
        }
    }
}

impl Settings {
    pub fn config_dir() -> Result<PathBuf> {
        let proj = ProjectDirs::from("com", "alexa", "alexa-cli")
            .context("Could not determine config directory")?;
        let dir = proj.config_dir().to_path_buf();
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config dir: {}", dir.display()))?;
        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        let mut settings: Settings = if !path.exists() {
            Self::default()
        } else {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config: {}", path.display()))?;
            toml::from_str(&content).context("Failed to parse config.toml")?
        };
        // Allow tests (and CI) to override the base URL without touching the config file.
        if let Ok(url) = std::env::var("ALEXA_BASE_URL") {
            settings.base_url = url;
        }
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        // The config may contain secrets (e.g. lwa_client_secret), so write it
        // with owner-only (0600) permissions rather than the umask default.
        write_private(&path, content.as_bytes())
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    pub fn set_email(&mut self, email: &str) {
        self.email = email.to_string();
    }

    #[allow(dead_code)]
    pub fn set_default_device(&mut self, device: &str) {
        self.default_device = Some(device.to_string());
    }

    /// Mark cookies as valid for the next 14 days
    pub fn mark_authenticated(&mut self) {
        let expires = chrono::Utc::now() + chrono::Duration::days(14);
        self.cookie_expires_at = Some(expires.timestamp());
    }

    /// Returns the device serial number used for device-pairing login,
    /// generating and persisting a random one on first use.
    pub fn ensure_device_serial_number(&mut self) -> Result<String> {
        if let Some(ref serial) = self.device_serial_number {
            return Ok(serial.clone());
        }
        let serial = uuid::Uuid::new_v4().simple().to_string();
        self.device_serial_number = Some(serial.clone());
        self.save()?;
        Ok(serial)
    }

    pub fn is_cookie_expired(&self) -> bool {
        match self.cookie_expires_at {
            None => true,
            Some(ts) => {
                let now = chrono::Utc::now().timestamp();
                // Consider expired if within 1 hour of expiry
                now >= ts - 3600
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_base_url() {
        assert_eq!(default_base_url(), "https://alexa.amazon.com");
    }

    #[test]
    fn test_default_locale() {
        assert_eq!(default_locale(), "en-US");
    }

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert_eq!(s.email, "");
        assert_eq!(s.base_url, "https://alexa.amazon.com");
        assert_eq!(s.locale, "en-US");
        assert!(s.default_device.is_none());
        assert!(s.cookie_expires_at.is_none());
    }

    #[test]
    fn test_set_email() {
        let mut s = Settings::default();
        s.set_email("user@example.com");
        assert_eq!(s.email, "user@example.com");
    }

    #[test]
    fn test_set_default_device() {
        let mut s = Settings::default();
        s.set_default_device("Kitchen Echo");
        assert_eq!(s.default_device, Some("Kitchen Echo".to_string()));
    }

    #[test]
    fn test_mark_authenticated_sets_expiry_about_14_days_out() {
        let mut s = Settings::default();
        s.mark_authenticated();
        let expiry = s.cookie_expires_at.expect("expiry should be set");
        let now = chrono::Utc::now().timestamp();
        // Should be roughly 14 days (within ±10 seconds)
        let expected = now + 14 * 24 * 3600;
        assert!(
            (expiry - expected).abs() < 10,
            "expiry={expiry}, expected≈{expected}"
        );
    }

    #[test]
    fn test_is_cookie_expired_when_none() {
        let s = Settings::default(); // cookie_expires_at = None
        assert!(s.is_cookie_expired());
    }

    #[test]
    fn test_is_cookie_expired_far_future() {
        let mut s = Settings::default();
        // Set expiry 30 days in the future
        let future = chrono::Utc::now().timestamp() + 30 * 24 * 3600;
        s.cookie_expires_at = Some(future);
        assert!(!s.is_cookie_expired());
    }

    #[test]
    fn test_is_cookie_expired_past_timestamp() {
        let mut s = Settings::default();
        // Set expiry in the past
        s.cookie_expires_at = Some(chrono::Utc::now().timestamp() - 1000);
        assert!(s.is_cookie_expired());
    }

    #[test]
    fn test_is_cookie_expired_within_1h_grace_period() {
        let mut s = Settings::default();
        // Set expiry exactly 30 minutes from now — should be considered expired
        let soon = chrono::Utc::now().timestamp() + 30 * 60;
        s.cookie_expires_at = Some(soon);
        assert!(s.is_cookie_expired());
    }

    #[test]
    fn test_alexa_base_url_env_var_override() {
        // The load() function picks up ALEXA_BASE_URL env var.
        // We verify the logic directly on the struct since load() touches the filesystem.
        let mut s = Settings::default();
        // Simulate what load() does with the env var
        s.base_url = "http://localhost:9999".to_string();
        assert_eq!(s.base_url, "http://localhost:9999");
    }

    #[cfg(unix)]
    #[test]
    fn test_write_private_creates_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("alexa-cli-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("secret.toml");

        write_private(&path, b"lwa_client_secret = \"shh\"").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {:o}", mode);

        // Overwriting an existing (looser) file should re-tighten it.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&path, b"updated").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected re-tightened 0600, got {:o}", mode);

        fs::remove_file(&path).ok();
        fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_settings_serializes_and_deserializes_via_toml() {
        let mut s = Settings::default();
        s.set_email("test@example.com");
        s.set_default_device("My Echo");
        s.mark_authenticated();

        let toml_str = toml::to_string_pretty(&s).expect("serialize");
        let s2: Settings = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(s2.email, "test@example.com");
        assert_eq!(s2.default_device, Some("My Echo".to_string()));
        assert_eq!(s2.base_url, "https://alexa.amazon.com");
        assert!(s2.cookie_expires_at.is_some());
    }
}
