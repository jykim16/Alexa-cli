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

pub async fn get_dnd_status(client: &ApiClient) -> Result<Vec<DndStatus>, AlexaError> {
    let raw: serde_json::Value = client.get("/api/dnd/status?cached=false").await?;

    // Handle both response shapes
    if let Some(arr) = raw
        .get("doNotDisturbDeviceStatusList")
        .and_then(|v| v.as_array())
    {
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

    #[tokio::test]
    async fn test_get_dnd_status_using_device_status_list() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"doNotDisturbDeviceStatusList":[{"deviceSerialNumber":"A","enabled":true}]}"#,
            )
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_dnd_status(&client).await;
        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].device_serial_number.as_deref(), Some("A"));
        assert_eq!(list[0].enabled, Some(true));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_dnd_status_using_dnd_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"dndResponse":[{"deviceSerialNumber":"B","enabled":false}]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_dnd_status(&client).await;
        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].device_serial_number.as_deref(), Some("B"));
        assert_eq!(list[0].enabled, Some(false));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_dnd_status_empty_object_returns_empty_vec() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_dnd_status(&client).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_dnd_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/dnd/status")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = set_dnd(&client, "SN1", "T1", true).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
