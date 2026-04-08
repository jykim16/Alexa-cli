use serde::{Deserialize, Serialize};

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub account_name: String,
    pub device_family: String,
    pub device_type: String,
    pub serial_number: String,
    pub software_version: Option<String>,
    pub online: Option<bool>,
    pub capabilities: Option<Vec<Capability>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub interface_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DevicesResponse {
    devices: Vec<Device>,
}

/// List all Alexa devices registered to the account.
pub async fn list_devices(client: &ApiClient) -> Result<Vec<Device>, AlexaError> {
    let resp: DevicesResponse = client.get("/api/devices-v2/device?cached=false").await?;
    Ok(resp.devices)
}

/// Find a device by name (case-insensitive substring match).
pub fn find_device<'a>(devices: &'a [Device], name: &str) -> Option<&'a Device> {
    let lower = name.to_lowercase();
    devices
        .iter()
        .find(|d| d.account_name.to_lowercase().contains(&lower))
}
