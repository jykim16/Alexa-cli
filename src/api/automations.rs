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

#[derive(Debug, Deserialize)]
struct AutomationsResponse(Vec<Automation>);

pub async fn list_automations(client: &ApiClient) -> Result<Vec<Automation>, AlexaError> {
    // The response is a bare JSON array
    let resp: serde_json::Value = client
        .get("/api/behaviors/automations?limit=2000")
        .await?;

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
pub fn find_automation<'a>(
    automations: &'a [Automation],
    name: &str,
) -> Option<&'a Automation> {
    let lower = name.to_lowercase();
    automations.iter().find(|a| {
        a.name
            .as_deref()
            .map(|n| n.to_lowercase().contains(&lower))
            .unwrap_or(false)
    })
}
