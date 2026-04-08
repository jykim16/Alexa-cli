use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: Option<String>,
    pub reminder_label: Option<String>,
    pub trigger_time: Option<u64>,
    pub status: Option<String>,
    pub device_serial_number: Option<String>,
    pub device_type: Option<String>,
    pub recurrence_rule: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemindersResponse {
    reminders: Vec<Reminder>,
}

pub async fn list_reminders(
    client: &ApiClient,
    serial_number: &str,
) -> Result<Vec<Reminder>, AlexaError> {
    let path = format!(
        "/api/reminders/device?deviceSerialNumber={}&cached=false",
        serial_number
    );
    let resp: RemindersResponse = client.get(&path).await?;
    Ok(resp.reminders)
}

pub async fn create_reminder(
    client: &ApiClient,
    serial_number: &str,
    device_type: &str,
    text: &str,
    iso8601_time: &str,
) -> Result<serde_json::Value, AlexaError> {
    // Convert ISO8601 to Unix milliseconds
    let trigger_time = chrono::DateTime::parse_from_rfc3339(iso8601_time)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|_| {
            (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp_millis()
        });

    let body = json!({
        "deviceSerialNumber": serial_number,
        "deviceType": device_type,
        "reminderLabel": text,
        "triggerTime": trigger_time,
        "status": "ON",
        "type": "Reminder"
    });
    client.post("/api/reminders", &body).await
}

pub async fn delete_reminder(client: &ApiClient, reminder_id: &str) -> Result<(), AlexaError> {
    client
        .delete(&format!("/api/reminders/{}", reminder_id))
        .await
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

    #[tokio::test]
    async fn test_list_reminders_returns_reminders() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"reminders":[{"id":"r1","reminderLabel":"dentist"}]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_reminders(&client, "ABC").await;
        assert!(result.is_ok());
        let reminders = result.unwrap();
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].id.as_deref(), Some("r1"));
        assert_eq!(reminders[0].reminder_label.as_deref(), Some("dentist"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_create_reminder_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/reminders")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = create_reminder(
            &client,
            "SN1",
            "T1",
            "Doctor appointment",
            "2026-05-01T09:00:00Z",
        )
        .await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delete_reminder_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/api/reminders/r-1")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = delete_reminder(&client, "r-1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
