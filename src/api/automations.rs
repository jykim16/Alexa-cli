use serde::{Deserialize, Serialize};

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Automation {
    pub automation_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub triggers: Option<Vec<serde_json::Value>>,
    pub sequence: Option<Sequence>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    pub sequence_json: Option<String>,
}

pub async fn list_automations(client: &ApiClient) -> Result<Vec<Automation>, AlexaError> {
    // The response is a bare JSON array
    let resp: serde_json::Value = client.get("/api/behaviors/automations?limit=2000").await?;

    if let Some(arr) = resp.as_array() {
        let automations: Vec<Automation> = arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        Ok(automations)
    } else {
        Ok(vec![])
    }
}

/// Find automation by name (case-insensitive contains).
pub fn find_automation<'a>(automations: &'a [Automation], name: &str) -> Option<&'a Automation> {
    let lower = name.to_lowercase();
    automations.iter().find(|a| {
        a.name
            .as_deref()
            .map(|n| n.to_lowercase().contains(&lower))
            .unwrap_or(false)
    })
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

    fn make_automation(name: Option<&str>) -> Automation {
        Automation {
            automation_id: Some("auto-1".to_string()),
            name: name.map(|s| s.to_string()),
            status: Some("ENABLED".to_string()),
            triggers: None,
            sequence: None,
        }
    }

    #[test]
    fn test_find_automation_exact_name() {
        let automations = vec![
            make_automation(Some("Good Morning")),
            make_automation(Some("Good Night")),
        ];
        let found = find_automation(&automations, "Good Morning");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name.as_deref(), Some("Good Morning"));
    }

    #[test]
    fn test_find_automation_case_insensitive_partial() {
        let automations = vec![
            make_automation(Some("Good Morning Routine")),
            make_automation(Some("Evening Wind Down")),
        ];
        let found = find_automation(&automations, "morning");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name.as_deref(), Some("Good Morning Routine"));
    }

    #[test]
    fn test_find_automation_returns_none_when_not_found() {
        let automations = vec![
            make_automation(Some("Good Morning")),
            make_automation(Some("Good Night")),
        ];
        let found = find_automation(&automations, "exercise");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_automation_works_when_name_is_none() {
        let automations = vec![make_automation(None), make_automation(Some("Good Night"))];
        // The None-named one should be skipped, still finds Good Night
        let found = find_automation(&automations, "good night");
        assert!(found.is_some());
        // None-named should not cause a panic
        let not_found = find_automation(&automations, "unnamed");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_automations_returns_automations() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/behaviors/automations?limit=2000")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"automationId":"a1","name":"Routine One","status":"ENABLED"},
                {"automationId":"a2","name":"Routine Two","status":"DISABLED"}
            ]"#,
            )
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_automations(&client).await;
        assert!(result.is_ok());
        let automations = result.unwrap();
        assert_eq!(automations.len(), 2);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_automations_graceful_fallback_on_object_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/behaviors/automations?limit=2000")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = list_automations(&client).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        mock.assert_async().await;
    }
}
