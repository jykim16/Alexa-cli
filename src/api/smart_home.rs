use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartHomeDevice {
    pub appliance_id: Option<String>,
    pub friendly_name: Option<String>,
    pub appliance_types: Option<Vec<String>>,
    pub capabilities: Option<Vec<SmartHomeCapability>>,
    pub is_enabled: Option<bool>,
    pub reachability: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SmartHomeCapability {
    pub interface: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhoenixPayload {
    network_detail: Option<NetworkDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkDetail {
    location_details: Option<serde_json::Value>,
}

pub async fn list_smart_home_devices(
    client: &ApiClient,
) -> Result<Vec<SmartHomeDevice>, AlexaError> {
    let raw: serde_json::Value = client.get("/api/phoenix?cached=false").await?;

    // Phoenix response nests devices deeply; flatten into a usable list
    let mut devices = Vec::new();
    collect_appliances(&raw, &mut devices);
    Ok(devices)
}

fn collect_appliances(value: &serde_json::Value, out: &mut Vec<SmartHomeDevice>) {
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_appliances(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            // If the object looks like an appliance
            if map.contains_key("applianceId") || map.contains_key("friendlyName") {
                if let Ok(dev) = serde_json::from_value::<SmartHomeDevice>(
                    serde_json::Value::Object(map.clone()),
                ) {
                    out.push(dev);
                    return;
                }
            }
            for v in map.values() {
                collect_appliances(v, out);
            }
        }
        _ => {}
    }
}

pub fn find_device<'a>(devices: &'a [SmartHomeDevice], name: &str) -> Option<&'a SmartHomeDevice> {
    let lower = name.to_lowercase();
    devices.iter().find(|d| {
        d.friendly_name
            .as_deref()
            .map(|n| n.to_lowercase().contains(&lower))
            .unwrap_or(false)
    })
}

/// Generic smart home control via the phoenix/state endpoint.
pub async fn control_device(
    client: &ApiClient,
    appliance_id: &str,
    action: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, AlexaError> {
    let mut parameters = params;
    if let serde_json::Value::Object(ref mut m) = parameters {
        m.insert("action".to_string(), json!(action));
    } else {
        parameters = json!({ "action": action });
    }

    let body = json!({
        "controlRequests": [{
            "entityId": appliance_id,
            "entityType": "APPLIANCE",
            "parameters": parameters
        }]
    });
    client.put("/api/phoenix/state", &body).await
}

pub async fn power(
    client: &ApiClient,
    appliance_id: &str,
    state: &str, // "turnOn" | "turnOff"
) -> Result<(), AlexaError> {
    control_device(client, appliance_id, state, json!({})).await?;
    Ok(())
}

pub async fn set_brightness(
    client: &ApiClient,
    appliance_id: &str,
    level: u8,
) -> Result<(), AlexaError> {
    control_device(
        client,
        appliance_id,
        "setBrightness",
        json!({ "brightness": level }),
    )
    .await?;
    Ok(())
}

pub async fn set_color(
    client: &ApiClient,
    appliance_id: &str,
    color_name: &str,
) -> Result<(), AlexaError> {
    control_device(
        client,
        appliance_id,
        "setColor",
        json!({ "colorName": color_name }),
    )
    .await?;
    Ok(())
}

pub async fn set_thermostat(
    client: &ApiClient,
    appliance_id: &str,
    target_temp: f64,
    scale: &str, // "FAHRENHEIT" | "CELSIUS"
) -> Result<(), AlexaError> {
    control_device(
        client,
        appliance_id,
        "setTargetTemperature",
        json!({
            "targetTemperature": {
                "value": target_temp,
                "scale": scale
            }
        }),
    )
    .await?;
    Ok(())
}

pub async fn lock(client: &ApiClient, appliance_id: &str, locked: bool) -> Result<(), AlexaError> {
    let action = if locked { "lockDoor" } else { "unlockDoor" };
    control_device(client, appliance_id, action, json!({})).await?;
    Ok(())
}
