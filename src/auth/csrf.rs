use anyhow::{bail, Context, Result};

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

    let body = resp.text().await.context("Failed to read bootstrap body")?;

    // Try to get csrfToken from response JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(token) = v.get("csrfToken").and_then(|t| t.as_str()) {
            if !token.is_empty() {
                return Ok(token.to_string());
            }
        }
    }

    bail!("No CSRF token in bootstrap response")
}
