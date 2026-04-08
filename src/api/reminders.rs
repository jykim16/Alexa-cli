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
