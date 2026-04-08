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
    async fn test_speak_sends_alexa_speak_payload() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/behaviors/preview")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex("Alexa\\.Speak".to_string()),
                mockito::Matcher::Regex("hello world".to_string()),
            ]))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = speak(&client, "hello world", "SN1", "T1", "en-US").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_announce_sends_alexa_announcement_with_device_serials() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/behaviors/preview")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex("AlexaAnnouncement".to_string()),
                mockito::Matcher::Regex("SN-DEVICE-1".to_string()),
                mockito::Matcher::Regex("SN-DEVICE-2".to_string()),
            ]))
            .create_async()
            .await;

        let client = make_client(&server);
        let devices = vec![
            ("SN-DEVICE-1".to_string(), "T1".to_string()),
            ("SN-DEVICE-2".to_string(), "T2".to_string()),
        ];
        let result = announce(&client, "Good morning!", &devices, "en-US").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_play_music_spotify_sends_correct_provider() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/behaviors/preview")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex("SPOTIFY".to_string()),
                mockito::Matcher::Regex("Alexa\\.Music\\.PlaySearchPhrase".to_string()),
            ]))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = play_music(&client, "rock classics", "SN1", "T1", "en-US", Some("spotify")).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_play_music_amazon_music_sends_correct_provider() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/behaviors/preview")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_body(mockito::Matcher::Regex("AMAZON_MUSIC".to_string()))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = play_music(&client, "jazz", "SN1", "T1", "en-US", Some("amazon-music")).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_play_music_none_service_defaults_to_amazon_music() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/behaviors/preview")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_body(mockito::Matcher::Regex("AMAZON_MUSIC".to_string()))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = play_music(&client, "classical", "SN1", "T1", "en-US", None).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_run_routine_sequence_sends_sequence_json_in_body() {
        let mut server = Server::new_async().await;
        let raw_seq = r#"{"startNode":{"type":"AlexaSpeak"}}"#;
        // The sequence_json field will be a JSON-encoded string containing raw_seq
        let mock = server
            .mock("POST", "/api/behaviors/preview")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex("PREVIEW".to_string()),
                mockito::Matcher::Regex("ENABLED".to_string()),
                mockito::Matcher::Regex("AlexaSpeak".to_string()),
            ]))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = run_routine_sequence(&client, raw_seq).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }
}
