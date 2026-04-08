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
