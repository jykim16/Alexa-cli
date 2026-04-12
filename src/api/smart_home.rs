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

pub async fn list_smart_home_devices(
    client: &ApiClient,
) -> Result<Vec<SmartHomeDevice>, AlexaError> {
    let raw: serde_json::Value = client.get("/api/phoenix?cached=false").await?;

    // Phoenix response nests devices deeply; flatten into a usable list
    let mut devices = Vec::new();
    collect_appliances(&raw, &mut devices);
    Ok(devices)
}

pub(crate) fn collect_appliances(value: &serde_json::Value, out: &mut Vec<SmartHomeDevice>) {
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

    fn make_sh_device(name: Option<&str>) -> SmartHomeDevice {
        SmartHomeDevice {
            appliance_id: Some("app-1".to_string()),
            friendly_name: name.map(|s| s.to_string()),
            appliance_types: None,
            capabilities: None,
            is_enabled: Some(true),
            reachability: None,
        }
    }

    #[test]
    fn test_find_device_case_insensitive_partial_match() {
        let devices = vec![
            make_sh_device(Some("Living Room Light")),
            make_sh_device(Some("Bedroom Lamp")),
        ];
        let found = find_device(&devices, "bedroom");
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().friendly_name.as_deref(),
            Some("Bedroom Lamp")
        );
    }

    #[test]
    fn test_find_device_returns_none_when_not_found() {
        let devices = vec![make_sh_device(Some("Kitchen Light"))];
        let found = find_device(&devices, "garage");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_device_works_when_friendly_name_is_none() {
        let devices = vec![make_sh_device(None), make_sh_device(Some("Porch Light"))];
        // None-named device is skipped, still finds Porch Light
        let found = find_device(&devices, "porch");
        assert!(found.is_some());
        // None-named should not panic
        let not_found = find_device(&devices, "unnamed");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_collect_appliances_from_object_with_appliance_id() {
        let json_val = serde_json::json!({
            "applianceId": "app-123",
            "friendlyName": "My Light"
        });
        let mut out = Vec::new();
        collect_appliances(&json_val, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].appliance_id.as_deref(), Some("app-123"));
    }

    #[test]
    fn test_collect_appliances_from_array() {
        let json_val = serde_json::json!([
            {"applianceId": "app-1", "friendlyName": "Light 1"},
            {"applianceId": "app-2", "friendlyName": "Light 2"}
        ]);
        let mut out = Vec::new();
        collect_appliances(&json_val, &mut out);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn test_collect_appliances_ignores_plain_string_no_panic() {
        let json_val = serde_json::json!("just a string");
        let mut out = Vec::new();
        collect_appliances(&json_val, &mut out);
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn test_list_smart_home_devices_returns_device() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/phoenix?cached=false")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"applianceId":"app-1","friendlyName":"Kitchen Light"}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_smart_home_devices(&client).await;
        assert!(result.is_ok());
        let devices = result.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].friendly_name.as_deref(), Some("Kitchen Light"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_power_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/phoenix/state")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = power(&client, "app-1", "turnOn").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_brightness_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/phoenix/state")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .match_body(mockito::Matcher::Regex("setBrightness".to_string()))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = set_brightness(&client, "app-1", 75).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_color_sends_set_color_action() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/phoenix/state")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex("setColor".to_string()),
                mockito::Matcher::Regex("blue".to_string()),
            ]))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = set_color(&client, "app-1", "blue").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_thermostat_sends_correct_action() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/phoenix/state")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex("setTargetTemperature".to_string()),
                mockito::Matcher::Regex("72".to_string()),
            ]))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = set_thermostat(&client, "app-1", 72.0, "FAHRENHEIT").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_lock_true_sends_lock_door_action() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/phoenix/state")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .match_body(mockito::Matcher::Regex("lockDoor".to_string()))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = lock(&client, "app-1", true).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_lock_false_sends_unlock_door_action() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/phoenix/state")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .match_body(mockito::Matcher::Regex("unlockDoor".to_string()))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = lock(&client, "app-1", false).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
