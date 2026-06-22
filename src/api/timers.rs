use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Timer {
    pub id: Option<String>,
    pub timer_label: Option<String>,
    pub device_serial_number: Option<String>,
    pub device_type: Option<String>,
    pub original_duration_seconds: Option<u64>,
    pub remaining_time: Option<u64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimersResponse {
    timers: Vec<Timer>,
}

pub async fn list_timers(client: &ApiClient) -> Result<Vec<Timer>, AlexaError> {
    let resp: TimersResponse = client.get("/api/timers/running?cached=false").await?;
    Ok(resp.timers)
}

/// Parse a duration string like "1h30m", "90m", "90s", "1h" into total seconds.
pub fn parse_duration(s: &str) -> u64 {
    let mut total = 0u64;
    let mut current = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            current.push(c);
        } else {
            let n: u64 = current.parse().unwrap_or(0);
            current.clear();
            match c {
                'h' | 'H' => total += n * 3600,
                'm' | 'M' => total += n * 60,
                's' | 'S' => total += n,
                _ => {}
            }
        }
    }
    // Handle bare number (treat as seconds)
    if !current.is_empty() {
        total += current.parse::<u64>().unwrap_or(0);
    }
    total
}

pub async fn create_timer(
    client: &ApiClient,
    serial_number: &str,
    device_type: &str,
    duration_secs: u64,
    label: Option<&str>,
) -> Result<serde_json::Value, AlexaError> {
    let body = json!({
        "deviceSerialNumber": serial_number,
        "deviceType": device_type,
        "duration": format!("PT{}S", duration_secs),
        "label": label.unwrap_or(""),
        "status": "ON",
        "timerLabel": label.unwrap_or(""),
        "originalDuration": format!("PT{}S", duration_secs)
    });
    client.post("/api/timers", &body).await
}

pub async fn cancel_timer(client: &ApiClient, timer_id: &str) -> Result<(), AlexaError> {
    client.delete(&format!("/api/timers/{}", timer_id)).await
}

pub async fn pause_timer(client: &ApiClient, timer_id: &str) -> Result<(), AlexaError> {
    client
        .put_no_body(&format!("/api/timers/{}/pause", timer_id))
        .await
}

pub async fn resume_timer(client: &ApiClient, timer_id: &str) -> Result<(), AlexaError> {
    client
        .put_no_body(&format!("/api/timers/{}/resume", timer_id))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use mockito::Server;
    use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
    use std::sync::Arc;

    fn make_client(server: &mockito::Server) -> crate::api::ApiClient {
        let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        let http = reqwest::Client::builder()
            .cookie_provider(Arc::clone(&cookie_store))
            .build()
            .unwrap();
        crate::api::ApiClient {
            http,
            csrf: "test-csrf".to_string(),
            base_url: server.url(),
            cookie_store,
            settings: Arc::new(Settings::default()),
        }
    }

    #[test]
    fn test_parse_duration_1h30m() {
        assert_eq!(parse_duration("1h30m"), 5400);
    }

    #[test]
    fn test_parse_duration_90m() {
        assert_eq!(parse_duration("90m"), 5400);
    }

    #[test]
    fn test_parse_duration_90s() {
        assert_eq!(parse_duration("90s"), 90);
    }

    #[test]
    fn test_parse_duration_1h() {
        assert_eq!(parse_duration("1h"), 3600);
    }

    #[test]
    fn test_parse_duration_bare_number_as_seconds() {
        assert_eq!(parse_duration("30"), 30);
    }

    #[test]
    fn test_parse_duration_2h30m15s() {
        assert_eq!(parse_duration("2h30m15s"), 9015);
    }

    #[test]
    fn test_parse_duration_zero() {
        assert_eq!(parse_duration("0"), 0);
    }

    #[test]
    fn test_parse_duration_empty() {
        assert_eq!(parse_duration(""), 0);
    }

    #[tokio::test]
    async fn test_list_timers_returns_timers() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/timers/running?cached=false")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"timers":[{"id":"t-1","status":"ON"},{"id":"t-2","status":"PAUSED"}]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_timers(&client).await;
        assert!(result.is_ok());
        let timers = result.unwrap();
        assert_eq!(timers.len(), 2);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_create_timer_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/timers")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = create_timer(&client, "SN1", "T1", 3600, Some("egg timer")).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_cancel_timer_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/api/timers/t-1")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = cancel_timer(&client, "t-1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_pause_timer_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/timers/t-1/pause")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = pause_timer(&client, "t-1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_resume_timer_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/timers/t-1/resume")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = resume_timer(&client, "t-1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
