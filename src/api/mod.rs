pub mod alarms;
pub mod automations;
pub mod behaviors;
pub mod devices;
pub mod dnd;
pub mod errors;
pub mod history;
pub mod lists;
pub mod media;
pub mod reminders;
pub mod smart_home;
pub mod timers;

use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

use reqwest_cookie_store::CookieStoreMutex;

use crate::auth::{build_client, fetch_csrf, load_cookie_store, save_cookie_store};
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

        // Build raw cookie string from cookie file for direct header injection
        let raw_cookies = crate::auth::cookie_store::load_raw_cookie_string()?;

        // Extract csrf from the raw cookies
        let csrf = raw_cookies
            .split(';')
            .find_map(|pair| {
                let pair = pair.trim();
                if pair.starts_with("csrf=") {
                    Some(pair[5..].to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();

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
            (
                "Referer",
                format!("{}/spa/index.html", self.base_url),
            ),
            ("Origin", self.base_url.clone()),
        ]
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.get(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        self.handle_response(resp).await
    }

    pub async fn get_text(&self, path: &str) -> Result<String, AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.get(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await.map_err(AlexaError::Network)?;
        if !status.is_success() {
            return Err(AlexaError::from_status(status, &body));
        }
        Ok(body)
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
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

    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.post(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        self.handle_response(resp).await
    }

    pub async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.put(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.json(body).send().await?;
        self.handle_response(resp).await
    }

    pub async fn put_no_body(&self, path: &str) -> Result<(), AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.put(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AlexaError::from_status(status, &body));
        }
        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<(), AlexaError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.delete(&url);
        for (k, v) in self.alexa_headers() {
            req = req.header(k, v);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AlexaError::from_status(status, &body));
        }
        Ok(())
    }

    async fn handle_response<T: DeserializeOwned>(
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

    /// Save cookies after operations that may set new session cookies.
    pub fn persist_cookies(&self) -> Result<()> {
        save_cookie_store(&self.cookie_store)
    }
}
