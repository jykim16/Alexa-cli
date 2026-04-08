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
fn format_alarm_time(time_str: &str) -> String {
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
