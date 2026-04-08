use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Alarm {
    pub alarm_index: Option<String>,
    pub id: Option<String>,
    pub device_serial_number: String,
    pub device_type: String,
    pub alarm_time: Option<u64>,
    pub date_time: Option<String>,
    pub status: Option<String>,
    pub alarm_label: Option<String>,
    #[serde(rename = "type")]
    pub alarm_type: Option<String>,
    pub recurring: Option<bool>,
    pub recurrence_rule: Option<String>,
    pub sound: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlarmsResponse {
    alarms: Vec<Alarm>,
}

pub async fn list_alarms(client: &ApiClient) -> Result<Vec<Alarm>, AlexaError> {
    let resp: AlarmsResponse = client
        .get("/api/alerts/running?cached=false")
        .await?;
    Ok(resp.alarms)
}

pub async fn create_alarm(
    client: &ApiClient,
    serial_number: &str,
    device_type: &str,
    time_str: &str, // "HH:MM" 24-hour
    label: Option<&str>,
) -> Result<Alarm, AlexaError> {
    let body = json!({
        "alarmTime": 0,
        "deviceSerialNumber": serial_number,
        "deviceType": device_type,
        "dateTime": format_alarm_time(time_str),
        "status": "ON",
        "type": "Alarm",
        "sound": {
            "displayName": "Simple Alarm",
            "folder": null,
            "id": "system_alerts_melodic_01",
            "providerId": "ECHO",
            "sampleUrl": ""
        },
        "alarmLabel": label.unwrap_or("")
    });
    client.post("/api/alarms", &body).await
}

/// Format "HH:MM" into the datetime string Amazon expects: "YYYY-MM-DDTHH:MM:SS.000"
pub(crate) fn format_alarm_time(time_str: &str) -> String {
    let now = chrono::Local::now();
    let parts: Vec<&str> = time_str.split(':').collect();
    let (h, m) = if parts.len() >= 2 {
        (
            parts[0].parse::<u32>().unwrap_or(7),
            parts[1].parse::<u32>().unwrap_or(0),
        )
    } else {
        (7, 0)
    };
    // Schedule for today, or tomorrow if time has passed
    let mut dt = now
        .date_naive()
        .and_hms_opt(h, m, 0)
        .unwrap_or_default();
    if dt < now.naive_local() {
        dt += chrono::Duration::days(1);
    }
    dt.format("%Y-%m-%dT%H:%M:%S.000").to_string()
}

pub async fn delete_alarm(client: &ApiClient, alarm_id: &str) -> Result<(), AlexaError> {
    client.delete(&format!("/api/alarms/{}", alarm_id)).await
}

pub async fn set_alarm_enabled(
    client: &ApiClient,
    alarm_id: &str,
    enabled: bool,
) -> Result<serde_json::Value, AlexaError> {
    let status = if enabled { "ON" } else { "OFF" };
    let body = json!({ "status": status });
    client.put(&format!("/api/alarms/{}", alarm_id), &body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
    use std::sync::Arc;
    use crate::config::Settings;

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
    fn test_format_alarm_time_valid_hhmm() {
        let result = format_alarm_time("07:30");
        // Should contain T07:30:00.000
        assert!(result.contains("T07:30:00.000"), "got: {}", result);
        // Should match YYYY-MM-DDTHH:MM:SS.000 pattern
        assert_eq!(result.len(), "2026-04-08T07:30:00.000".len());
    }

    #[test]
    fn test_format_alarm_time_invalid_uses_defaults() {
        let result = format_alarm_time("invalid");
        // Falls back to h=7, m=0
        assert!(result.contains("T07:00:00.000"), "got: {}", result);
    }

    #[test]
    fn test_format_alarm_time_23_59() {
        let result = format_alarm_time("23:59");
        assert!(result.ends_with("T23:59:00.000"), "got: {}", result);
    }

    #[tokio::test]
    async fn test_list_alarms_returns_alarms() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/alerts/running?cached=false")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"alarms":[{"deviceSerialNumber":"ABC","deviceType":"T1","status":"ON"}]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_alarms(&client).await;
        assert!(result.is_ok());
        let alarms = result.unwrap();
        assert_eq!(alarms.len(), 1);
        assert_eq!(alarms[0].device_serial_number, "ABC");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_create_alarm_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/alarms")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"deviceSerialNumber":"SN1","deviceType":"T1","status":"ON"}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = create_alarm(&client, "SN1", "T1", "08:00", Some("morning alarm")).await;
        assert!(result.is_ok());
        let alarm = result.unwrap();
        assert_eq!(alarm.status, Some("ON".to_string()));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delete_alarm_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/api/alarms/alarm-1")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = delete_alarm(&client, "alarm-1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_alarm_enabled_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/alarms/alarm-1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = set_alarm_enabled(&client, "alarm-1", true).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_alarm_disabled_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/alarms/alarm-1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({"status": "OFF"})))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = set_alarm_enabled(&client, "alarm-1", false).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
