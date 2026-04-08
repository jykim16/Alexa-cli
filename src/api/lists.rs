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
    let path = format!("/api/todos?type={}&size=100&startTime=&endTime=&completed=false", list_type);
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

pub fn shopping_type() -> &'static str {
    SHOPPING
}

pub fn task_type() -> &'static str {
    TASK
}
