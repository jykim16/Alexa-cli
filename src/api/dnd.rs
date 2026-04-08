use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DndStatus {
    pub device_serial_number: Option<String>,
    pub device_type: Option<String>,
    pub enabled: Option<bool>,
    pub expiry_time: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DndResponse {
    dnd_response: Option<Vec<DndStatus>>,
    // Some versions use a flat list
    #[serde(rename = "doNotDisturbDeviceStatusList")]
    device_status_list: Option<Vec<DndStatus>>,
}

pub async fn get_dnd_status(client: &ApiClient) -> Result<Vec<DndStatus>, AlexaError> {
    let raw: serde_json::Value = client.get("/api/dnd/status?cached=false").await?;

    // Handle both response shapes
    if let Some(arr) = raw.get("doNotDisturbDeviceStatusList").and_then(|v| v.as_array()) {
        return Ok(arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect());
    }
    if let Some(arr) = raw.get("dndResponse").and_then(|v| v.as_array()) {
        return Ok(arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect());
    }
    Ok(vec![])
}

pub async fn set_dnd(
    client: &ApiClient,
    serial_number: &str,
    device_type: &str,
    enabled: bool,
) -> Result<serde_json::Value, AlexaError> {
    let body = json!({
        "deviceSerialNumber": serial_number,
        "deviceType": device_type,
        "enabled": enabled
    });
    client.put("/api/dnd/status", &body).await
}
