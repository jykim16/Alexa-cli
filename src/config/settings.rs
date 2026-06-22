use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

    /// LWA client_id for device code login
    #[serde(default)]
    pub lwa_client_id: Option<String>,

    /// AVS product ID
    #[serde(default)]
    pub avs_product_id: Option<String>,

    /// Device serial number (auto-generated)
    #[serde(default)]
    pub device_serial_number: Option<String>,

    /// Stored refresh token
    #[serde(default)]
    pub refresh_token: Option<String>,
}

fn default_base_url() -> String {
    "https://alexa.amazon.com".to_string()
}

fn default_locale() -> String {
    "en-US".to_string()
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
            avs_product_id: None,
            device_serial_number: None,
            refresh_token: None,
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
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let settings: Settings =
            toml::from_str(&content).context("Failed to parse config.toml")?;
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    pub fn set_email(&mut self, email: &str) {
        self.email = email.to_string();
    }

    pub fn set_default_device(&mut self, device: &str) {
        self.default_device = Some(device.to_string());
    }

    /// Mark cookies as valid for the next 14 days
    pub fn mark_authenticated(&mut self) {
        let expires = chrono::Utc::now() + chrono::Duration::days(14);
        self.cookie_expires_at = Some(expires.timestamp());
    }

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
