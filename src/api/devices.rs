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

    fn make_device(name: &str) -> Device {
        Device {
            account_name: name.to_string(),
            device_family: "ECHO".to_string(),
            device_type: "A3C9PE6TNYLTCH".to_string(),
            serial_number: "ABCDEF123".to_string(),
            software_version: None,
            online: Some(true),
            capabilities: None,
        }
    }

    #[test]
    fn test_find_device_exact_match() {
        let devices = vec![make_device("Living Room Echo"), make_device("Kitchen Echo")];
        let found = find_device(&devices, "Living Room Echo");
        assert!(found.is_some());
        assert_eq!(found.unwrap().account_name, "Living Room Echo");
    }

    #[test]
    fn test_find_device_case_insensitive_partial_match() {
        let devices = vec![make_device("Kitchen Echo"), make_device("Bedroom Echo")];
        let found = find_device(&devices, "kitchen");
        assert!(found.is_some());
        assert_eq!(found.unwrap().account_name, "Kitchen Echo");
    }

    #[test]
    fn test_find_device_returns_none_when_not_found() {
        let devices = vec![make_device("Kitchen Echo"), make_device("Bedroom Echo")];
        let found = find_device(&devices, "garage");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_devices_returns_devices() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/devices-v2/device?cached=false")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"devices":[
                {"accountName":"Echo 1","deviceFamily":"ECHO","deviceType":"T1","serialNumber":"SN1"},
                {"accountName":"Echo 2","deviceFamily":"ECHO","deviceType":"T2","serialNumber":"SN2"}
            ]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_devices(&client).await;
        assert!(result.is_ok());
        let devices = result.unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].account_name, "Echo 1");
        assert_eq!(devices[1].account_name, "Echo 2");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_devices_401_returns_session_expired() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/devices-v2/device?cached=false")
            .with_status(401)
            .with_body("unauthorized")
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_devices(&client).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::api::errors::AlexaError::SessionExpired
        ));
        mock.assert_async().await;
    }
}
