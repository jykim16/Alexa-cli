use serde::{Deserialize, Serialize};

use super::{errors::AlexaError, ApiClient};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub provider_id: Option<String>,
    pub entity_id: Option<String>,
    pub state: Option<String>,
    pub main_art: Option<MainArt>,
    pub header_text: Option<String>,
    pub title: Option<String>,
    pub progress_seconds: Option<u64>,
    pub duration_seconds: Option<u64>,
    pub media_type: Option<String>,
    pub controls: Option<Vec<Control>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MainArt {
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Control {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NowPlayingResponse {
    player_info: Option<NowPlaying>,
}

pub async fn get_now_playing(
    client: &ApiClient,
    serial_number: &str,
    device_type: &str,
) -> Result<Option<NowPlaying>, AlexaError> {
    let path = format!(
        "/api/np/player?deviceSerialNumber={}&deviceType={}&screenWidth=1440",
        serial_number, device_type
    );
    let resp: NowPlayingResponse = client.get(&path).await?;
    Ok(resp.player_info)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaCommand {
    #[serde(rename = "type")]
    cmd_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    volume_setting: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_type: Option<String>,
}

async fn send_command(
    client: &ApiClient,
    cmd_type: &str,
    serial_number: &str,
    device_type: &str,
    volume: Option<u8>,
) -> Result<serde_json::Value, AlexaError> {
    let cmd = MediaCommand {
        cmd_type: cmd_type.to_string(),
        volume_setting: volume,
        device_serial_number: Some(serial_number.to_string()),
        device_type: Some(device_type.to_string()),
    };
    client.post("/api/np/command", &cmd).await
}

pub async fn play(client: &ApiClient, sn: &str, dt: &str) -> Result<(), AlexaError> {
    send_command(client, "PlayCommand", sn, dt, None).await?;
    Ok(())
}

pub async fn pause(client: &ApiClient, sn: &str, dt: &str) -> Result<(), AlexaError> {
    send_command(client, "PauseCommand", sn, dt, None).await?;
    Ok(())
}

pub async fn next(client: &ApiClient, sn: &str, dt: &str) -> Result<(), AlexaError> {
    send_command(client, "NextCommand", sn, dt, None).await?;
    Ok(())
}

pub async fn previous(client: &ApiClient, sn: &str, dt: &str) -> Result<(), AlexaError> {
    send_command(client, "PreviousCommand", sn, dt, None).await?;
    Ok(())
}

pub async fn set_volume(
    client: &ApiClient,
    sn: &str,
    dt: &str,
    level: u8,
) -> Result<(), AlexaError> {
    send_command(client, "SetVolumeCommand", sn, dt, Some(level)).await?;
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
    async fn test_play_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/np/command")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create_async()
            .await;

        let client = make_client(&server);
        let result = play(&client, "SN1", "T1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_pause_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/np/command")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create_async()
            .await;

        let client = make_client(&server);
        let result = pause(&client, "SN1", "T1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_next_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/np/command")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create_async()
            .await;

        let client = make_client(&server);
        let result = next(&client, "SN1", "T1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_previous_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/np/command")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create_async()
            .await;

        let client = make_client(&server);
        let result = previous(&client, "SN1", "T1").await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_volume_ok() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/np/command")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({"volumeSetting": 50})))
            .create_async()
            .await;

        let client = make_client(&server);
        let result = set_volume(&client, "SN1", "T1", 50).await;
        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_now_playing_with_player_info() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"playerInfo":{"state":"PLAYING","providerId":"SPOTIFY","entityId":"track123"}}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_now_playing(&client, "SN1", "T1").await;
        assert!(result.is_ok());
        let np = result.unwrap();
        assert!(np.is_some());
        let np = np.unwrap();
        assert_eq!(np.state, Some("PLAYING".to_string()));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_now_playing_when_player_info_null() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"playerInfo":null}"#)
            .create_async()
            .await;

        let client = make_client(&server);
        let result = get_now_playing(&client, "SN1", "T1").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        mock.assert_async().await;
    }
}
