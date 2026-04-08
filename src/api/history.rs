use serde::{Deserialize, Serialize};

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: Option<String>,
    pub device: Option<ActivityDevice>,
    pub description: Option<ActivityDescription>,
    pub creation_timestamp: Option<u64>,
    pub registration_key: Option<ActivityRegistration>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDevice {
    pub device_name: Option<String>,
    pub device_serial_number: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDescription {
    pub app_id: Option<String>,
    pub app_device_list: Option<Vec<serde_json::Value>>,
    pub summary_text: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRegistration {
    pub registration_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivitiesResponse {
    activities: Option<Vec<Activity>>,
}

pub async fn get_history(client: &ApiClient, limit: usize) -> Result<Vec<Activity>, AlexaError> {
    let path = format!(
        "/api/activities?startTime=&endTime=&size={}&offset=-1",
        limit
    );
    let resp: ActivitiesResponse = client.get(&path).await?;
    Ok(resp.activities.unwrap_or_default())
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
    async fn test_get_history_returns_activities() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"activities":[{"id":"h1","creationTimestamp":1000}]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_history(&client, 10).await;
        assert!(result.is_ok());
        let activities = result.unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].id.as_deref(), Some("h1"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_history_null_activities_returns_empty_vec() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"activities":null}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_history(&client, 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        mock.assert_async().await;
    }
}
