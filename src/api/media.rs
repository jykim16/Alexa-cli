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
