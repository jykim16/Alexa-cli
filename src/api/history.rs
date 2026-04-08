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
