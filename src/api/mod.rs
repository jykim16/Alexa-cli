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
}

impl ApiClient {
    /// Load cookies from storage, fetch CSRF token, and return a ready-to-use client.
    pub async fn new(settings: Arc<Settings>) -> Result<Self> {
        let cookie_store = load_cookie_store()?;
        let http = build_client(Arc::clone(&cookie_store))?;
        let base_url = settings.base_url.clone();

        let csrf = fetch_csrf(&http, &base_url).await?;

        Ok(Self {
            http,
            csrf,
            base_url,
            cookie_store,
            settings,
        })
    }

    fn alexa_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Accept", "application/json".to_string()),
            ("Content-Type", "application/json; charset=UTF-8".to_string()),
            ("X-Csrf-Token", self.csrf.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
    use std::sync::Arc;
    use crate::config::Settings;

    fn make_client(server: &mockito::Server) -> ApiClient {
        let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        let http = reqwest::Client::builder()
            .cookie_provider(Arc::clone(&cookie_store))
            .build()
            .unwrap();
        ApiClient {
            http,
            csrf: "test-csrf".to_string(),
            base_url: server.url(),
            cookie_store,
            settings: Arc::new(Settings::default()),
        }
    }

    #[test]
    fn test_alexa_headers_contains_required_keys() {
        // Create a dummy server just to get a URL
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = rt.block_on(Server::new_async());
        let client = make_client(&server);
        let headers = client.alexa_headers();
        let keys: Vec<&str> = headers.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"X-Csrf-Token"));
        assert!(keys.contains(&"Accept"));
        assert!(keys.contains(&"Content-Type"));
        // Verify CSRF value is correct
        let csrf_header = headers.iter().find(|(k, _)| *k == "X-Csrf-Token");
        assert!(csrf_header.is_some());
        assert_eq!(csrf_header.unwrap().1, "test-csrf");
    }

    #[tokio::test]
    async fn test_get_success_deserializes_json() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test-path")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"key":"value"}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result: Result<serde_json::Value, AlexaError> = client.get("/test-path").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["key"], "value");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_401_returns_session_expired() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test-path")
            .with_status(401)
            .with_body("unauthorized")
            .create_async()
            .await;

        let client = make_client(&server);
        let result: Result<serde_json::Value, AlexaError> = client.get("/test-path").await;
        assert!(matches!(result.unwrap_err(), AlexaError::SessionExpired));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_429_returns_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test-path")
            .with_status(429)
            .with_body("too many requests")
            .create_async()
            .await;

        let client = make_client(&server);
        let result: Result<serde_json::Value, AlexaError> = client.get("/test-path").await;
        assert!(matches!(result.unwrap_err(), AlexaError::RateLimited));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_success_deserializes_json() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test-post")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result":"ok"}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let body = serde_json::json!({"data": "test"});
        let result: Result<serde_json::Value, AlexaError> = client.post("/test-post", &body).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["result"], "ok");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_put_success_deserializes_json() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/test-put")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"updated":true}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let body = serde_json::json!({"status": "ON"});
        let result: Result<serde_json::Value, AlexaError> = client.put("/test-put", &body).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["updated"], true);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_put_no_body_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/test-put-nobody")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = client.put_no_body("/test-put-nobody").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delete_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/test-delete")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = client.delete("/test-delete").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delete_404_returns_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/test-delete-missing")
            .with_status(404)
            .with_body("not found")
            .create_async()
            .await;

        let client = make_client(&server);
        let result = client.delete("/test-delete-missing").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AlexaError::ApiError { status: 404, .. }));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_handle_response_malformed_json_returns_other_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/bad-json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("this is not json at all !!!")
            .create_async()
            .await;

        let client = make_client(&server);
        let result: Result<serde_json::Value, AlexaError> = client.get("/bad-json").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AlexaError::Other(_)));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_text_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/text-endpoint")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("hello world")
            .create_async()
            .await;

        let client = make_client(&server);
        let result = client.get_text("/text-endpoint").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
        mock.assert_async().await;
    }
}
