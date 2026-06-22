pub mod behaviors;
pub mod devices;
pub mod errors;

use anyhow::Result;
use std::sync::Arc;

use reqwest_cookie_store::CookieStoreMutex;

use crate::auth::{build_client, load_cookie_store, save_cookie_store};
use crate::config::Settings;
use errors::AlexaError;

/// Central HTTP client that attaches cookies + CSRF headers to every request.
pub struct ApiClient {
    pub http: reqwest::Client,
    pub csrf: String,
    pub base_url: String,
    pub cookie_store: Arc<CookieStoreMutex>,
    pub settings: Arc<Settings>,
    pub raw_cookies: String,
}

impl ApiClient {
    /// Load cookies from storage, fetch CSRF token, and return a ready-to-use client.
    pub async fn new(settings: Arc<Settings>) -> Result<Self> {
        let cookie_store = load_cookie_store()?;
        let http = build_client(Arc::clone(&cookie_store))?;
        let base_url = settings.base_url.clone();

        let raw_cookies = crate::auth::cookie_store::load_raw_cookie_string()?;

        let csrf = {
            let bootstrap_url = format!("{}/api/bootstrap?version=0", base_url);
            let resp = http
                .get(&bootstrap_url)
                .header("Cookie", &raw_cookies)
                .send()
                .await
                .ok();

            let mut csrf_val = String::new();
            if let Some(r) = resp {
                for cookie_str in r.headers().get_all("set-cookie") {
                    if let Ok(s) = cookie_str.to_str() {
                        if s.starts_with("csrf=") {
                            if let Some(end) = s.find(';') {
                                csrf_val = s[5..end].to_string();
                            }
                        }
                    }
                }
            }
            if csrf_val.is_empty() {
                csrf_val = raw_cookies
                    .split(';')
                    .find_map(|pair| {
                        let pair = pair.trim();
                        pair.strip_prefix("csrf=").map(|v| v.to_string())
                    })
                    .unwrap_or_default();
            }
            csrf_val
        };

        let raw_cookies = if !csrf.is_empty() && !raw_cookies.contains("csrf=") {
            format!("{}; csrf={}", raw_cookies, csrf)
        } else {
            raw_cookies
        };

        Ok(Self {
            http,
            csrf,
            base_url,
            cookie_store,
            settings,
            raw_cookies,
        })
    }

    pub(crate) fn alexa_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Cookie", self.raw_cookies.clone()),
            ("Accept", "application/json".to_string()),
            ("Content-Type", "application/json; charset=UTF-8".to_string()),
            ("csrf", self.csrf.clone()),
            ("Referer", format!("{}/spa/index.html", self.base_url)),
            ("Origin", self.base_url.clone()),
        ]
    }

    pub async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.get(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        self.handle_response(resp).await
    }

    pub async fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.post(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.json(body).send().await?;
        self.handle_response(resp).await
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, AlexaError> {
        let status = resp.status();
        if status.is_success() {
            let bytes = resp.bytes().await?;
            serde_json::from_slice(&bytes).map_err(|e| {
                AlexaError::Other(format!(
                    "JSON parse error: {e}\nBody: {}",
                    String::from_utf8_lossy(&bytes).chars().take(500).collect::<String>()
                ))
            })
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(AlexaError::from_status(status, &body))
        }
    }

    pub fn persist_cookies(&self) -> Result<()> {
        save_cookie_store(&self.cookie_store)
    }
}
