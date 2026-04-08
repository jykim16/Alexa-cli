use serde::Serialize;
use serde_json::json;

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BehaviorPreviewRequest {
    behavior_id: String,
    sequence_json: String,
    status: String,
}

async fn post_behavior(client: &ApiClient, sequence: serde_json::Value) -> Result<(), AlexaError> {
    let seq_json = serde_json::to_string(&sequence)
        .map_err(|e| AlexaError::Other(e.to_string()))?;

    let req = BehaviorPreviewRequest {
        behavior_id: "PREVIEW".to_string(),
        sequence_json: seq_json,
        status: "ENABLED".to_string(),
    };

    let _: serde_json::Value = client.post("/api/behaviors/preview", &req).await?;
    Ok(())
}

/// Make a device speak text via TTS.
pub async fn speak(
    client: &ApiClient,
    text: &str,
    serial_number: &str,
    device_type: &str,
    locale: &str,
) -> Result<(), AlexaError> {
    let sequence = json!({
        "@type": "com.amazon.alexa.behaviors.model.Sequence",
        "startNode": {
            "@type": "com.amazon.alexa.behaviors.model.OpaquePayloadOperationNode",
            "type": "Alexa.Speak",
            "operationPayload": {
                "deviceType": device_type,
                "deviceSerialNumber": serial_number,
                "locale": locale,
                "textToSpeak": text
            }
        }
    });
    post_behavior(client, sequence).await
}

/// Broadcast an announcement to all/specified devices.
pub async fn announce(
    client: &ApiClient,
    text: &str,
    devices: &[(String, String)], // (serialNumber, deviceType) pairs
    locale: &str,
) -> Result<(), AlexaError> {
    let targets: Vec<serde_json::Value> = devices
        .iter()
        .map(|(sn, dt)| {
            json!({
                "deviceSerialNumber": sn,
                "deviceType": dt,
                "locale": locale
            })
        })
        .collect();

    let sequence = json!({
        "@type": "com.amazon.alexa.behaviors.model.Sequence",
        "startNode": {
            "@type": "com.amazon.alexa.behaviors.model.OpaquePayloadOperationNode",
            "type": "AlexaAnnouncement",
            "operationPayload": {
                "expireAfter": "PT5S",
                "content": [{
                    "locale": locale,
                    "display": {
                        "title": "Announcement",
                        "body": text
                    },
                    "speak": {
                        "type": "text",
                        "value": text
                    }
                }],
                "target": {
                    "customerId": "",
                    "devices": targets
                }
            }
        }
    });
    post_behavior(client, sequence).await
}

/// Play music by search phrase on a device.
pub async fn play_music(
    client: &ApiClient,
    query: &str,
    serial_number: &str,
    device_type: &str,
    locale: &str,
    service: Option<&str>,
) -> Result<(), AlexaError> {
    let provider = match service {
        Some("spotify") => "SPOTIFY",
        Some("amazon-music") | Some("amazon") => "AMAZON_MUSIC",
        Some("apple-music") | Some("apple") => "APPLE_MUSIC",
        Some("pandora") => "PANDORA",
        Some("tunein") => "TUNEIN",
        Some("iheartradio") => "I_HEART_RADIO",
        _ => "AMAZON_MUSIC",
    };

    let sequence = json!({
        "@type": "com.amazon.alexa.behaviors.model.Sequence",
        "startNode": {
            "@type": "com.amazon.alexa.behaviors.model.OpaquePayloadOperationNode",
            "type": "Alexa.Music.PlaySearchPhrase",
            "operationPayload": {
                "deviceType": device_type,
                "deviceSerialNumber": serial_number,
                "locale": locale,
                "musicProviderId": provider,
                "searchPhrase": query
            }
        }
    });
    post_behavior(client, sequence).await
}

/// Run a routine by its sequence JSON (fetched from automations API).
pub async fn run_routine_sequence(
    client: &ApiClient,
    sequence_json: &str,
) -> Result<(), AlexaError> {
    let req = BehaviorPreviewRequest {
        behavior_id: "PREVIEW".to_string(),
        sequence_json: sequence_json.to_string(),
        status: "ENABLED".to_string(),
    };
    let _: serde_json::Value = client.post("/api/behaviors/preview", &req).await?;
    Ok(())
}
