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
    let resp: TimersResponse = client
        .get("/api/timers/running?cached=false")
        .await?;
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
    client
        .delete(&format!("/api/timers/{}", timer_id))
        .await
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
