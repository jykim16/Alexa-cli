use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct BootstrapResponse {
    #[serde(rename = "csrfToken")]
    csrf_token: Option<String>,
}

/// Fetch the CSRF token from alexa.amazon.com/api/bootstrap.
/// This also validates that the session is still authenticated.
pub async fn fetch_csrf(client: &reqwest::Client, base_url: &str) -> Result<String> {
    let url = format!("{}/api/bootstrap?version=0", base_url);

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to GET /api/bootstrap")?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        bail!("Session expired. Please run `alexa-cli auth login` to re-authenticate.");
    }
    if !status.is_success() {
        bail!("Bootstrap request failed with status: {}", status);
    }

    let bootstrap: BootstrapResponse = resp
        .json()
        .await
        .context("Failed to parse bootstrap response")?;

    bootstrap
        .csrf_token
        .filter(|s| !s.is_empty())
        .context("No CSRF token in bootstrap response — session may not be authenticated")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent("test-agent")
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_fetch_csrf_returns_token_on_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/bootstrap?version=0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"csrfToken":"csrf-token-abc123"}"#)
            .create_async()
            .await;

        let client = make_http_client();
        let result = fetch_csrf(&client, &server.url()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "csrf-token-abc123");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_csrf_returns_error_on_401() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/bootstrap?version=0")
            .with_status(401)
            .with_body("unauthorized")
            .create_async()
            .await;

        let client = make_http_client();
        let result = fetch_csrf(&client, &server.url()).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Session expired") || msg.contains("auth login"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_csrf_returns_error_on_403() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/bootstrap?version=0")
            .with_status(403)
            .with_body("forbidden")
            .create_async()
            .await;

        let client = make_http_client();
        let result = fetch_csrf(&client, &server.url()).await;
        assert!(result.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_csrf_returns_error_when_token_missing() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/bootstrap?version=0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"someOtherKey":"value"}"#)
            .create_async()
            .await;

        let client = make_http_client();
        let result = fetch_csrf(&client, &server.url()).await;
        assert!(result.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_csrf_returns_error_when_token_empty_string() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/bootstrap?version=0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"csrfToken":""}"#)
            .create_async()
            .await;

        let client = make_http_client();
        let result = fetch_csrf(&client, &server.url()).await;
        assert!(result.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_csrf_returns_error_on_server_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/bootstrap?version=0")
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let client = make_http_client();
        let result = fetch_csrf(&client, &server.url()).await;
        assert!(result.is_err());
        mock.assert_async().await;
    }
}
