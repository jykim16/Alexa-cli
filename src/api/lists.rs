use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub item_id: Option<String>,
    pub id: Option<String>,
    pub value: String,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub completed: Option<bool>,
    pub created_date_time: Option<String>,
    pub updated_date_time: Option<String>,
}

impl ListItem {
    pub fn id(&self) -> Option<&str> {
        self.item_id.as_deref().or(self.id.as_deref())
    }
}

#[derive(Debug, Deserialize)]
struct ListsResponse {
    values: Option<Vec<ListItem>>,
}

const SHOPPING: &str = "SHOPPING_ITEM";
const TASK: &str = "TASK";

pub async fn get_shopping_list(client: &ApiClient) -> Result<Vec<ListItem>, AlexaError> {
    get_list(client, SHOPPING).await
}

pub async fn get_todo_list(client: &ApiClient) -> Result<Vec<ListItem>, AlexaError> {
    get_list(client, TASK).await
}

async fn get_list(client: &ApiClient, list_type: &str) -> Result<Vec<ListItem>, AlexaError> {
    let path = format!(
        "/api/todos?type={}&size=100&startTime=&endTime=&completed=false",
        list_type
    );
    let resp: ListsResponse = client.get(&path).await?;
    Ok(resp.values.unwrap_or_default())
}

pub async fn add_item(
    client: &ApiClient,
    list_type: &str,
    text: &str,
) -> Result<serde_json::Value, AlexaError> {
    let body = json!({
        "type": list_type,
        "text": text,
        "complete": false,
        "deleted": false
    });
    client.post("/api/todos", &body).await
}

pub async fn add_shopping_item(client: &ApiClient, text: &str) -> Result<(), AlexaError> {
    add_item(client, SHOPPING, text).await?;
    Ok(())
}

pub async fn add_todo_item(client: &ApiClient, text: &str) -> Result<(), AlexaError> {
    add_item(client, TASK, text).await?;
    Ok(())
}

pub async fn delete_item(client: &ApiClient, item_id: &str) -> Result<(), AlexaError> {
    client.delete(&format!("/api/todos/{}", item_id)).await
}

pub async fn complete_item(
    client: &ApiClient,
    item_id: &str,
    list_type: &str,
) -> Result<serde_json::Value, AlexaError> {
    let body = json!({
        "type": list_type,
        "complete": true,
        "deleted": false
    });
    client.put(&format!("/api/todos/{}", item_id), &body).await
}

#[allow(dead_code)]
pub fn shopping_type() -> &'static str {
    SHOPPING
}

pub fn task_type() -> &'static str {
    TASK
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

    #[test]
    fn test_shopping_type_value() {
        assert_eq!(shopping_type(), "SHOPPING_ITEM");
    }

    #[test]
    fn test_task_type_value() {
        assert_eq!(task_type(), "TASK");
    }

    #[test]
    fn test_list_item_id_prefers_item_id() {
        let item = ListItem {
            item_id: Some("item-id-1".to_string()),
            id: Some("id-1".to_string()),
            value: "milk".to_string(),
            item_type: None,
            completed: None,
            created_date_time: None,
            updated_date_time: None,
        };
        assert_eq!(item.id(), Some("item-id-1"));
    }

    #[test]
    fn test_list_item_id_falls_back_to_id() {
        let item = ListItem {
            item_id: None,
            id: Some("id-1".to_string()),
            value: "eggs".to_string(),
            item_type: None,
            completed: None,
            created_date_time: None,
            updated_date_time: None,
        };
        assert_eq!(item.id(), Some("id-1"));
    }

    #[test]
    fn test_list_item_id_returns_none_when_both_absent() {
        let item = ListItem {
            item_id: None,
            id: None,
            value: "bread".to_string(),
            item_type: None,
            completed: None,
            created_date_time: None,
            updated_date_time: None,
        };
        assert_eq!(item.id(), None);
    }

    #[tokio::test]
    async fn test_get_shopping_list_returns_items() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"values":[{"value":"milk","completed":false}]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_shopping_list(&client).await;
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, "milk");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_todo_list_returns_items() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"values":[{"value":"call doctor","completed":false},{"value":"buy groceries","completed":true}]}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_todo_list(&client).await;
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_add_shopping_item_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/todos")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = add_shopping_item(&client, "butter").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delete_item_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/api/todos/item-1")
            .with_status(200)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = delete_item(&client, "item-1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_complete_item_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/api/todos/item-1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = complete_item(&client, "item-1", SHOPPING).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
