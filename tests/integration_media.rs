/// Integration tests for the `media` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_media -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const DEVICES_JSON: &str = r#"{"devices":[
    {"accountName":"Living Room Echo","deviceFamily":"ECHO","deviceType":"A3C9PE6TNYLTCH",
     "serialNumber":"SN001","online":true}
]}"#;

const NOW_PLAYING_JSON: &str = r#"{"playerInfo":{
    "state":"PLAYING","providerId":"SPOTIFY","entityId":"track42",
    "title":"Good Day Sunshine","headerText":"The Beatles"
}}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_media_status_returns_now_playing_json() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_get("/api/np/player.*", 200, NOW_PLAYING_JSON).await;

    let (ok, stdout, _) = run_binary(
        &wm.url,
        &["media", "status", "--device", "Living Room", "--output", "json"],
    );
    assert!(ok, "media status should succeed");
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("not JSON: {stdout}"));
    assert_eq!(json["state"], "PLAYING");
    assert_eq!(json["title"], "Good Day Sunshine");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_media_play_sends_play_command() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post("/api/np/command", 200, "{}").await;

    let (ok, _stdout, _stderr) =
        run_binary(&wm.url, &["media", "play", "--device", "Living Room"]);
    assert!(ok, "media play should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_media_pause_sends_pause_command() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post("/api/np/command", 200, "{}").await;

    let (ok, _, _) = run_binary(&wm.url, &["media", "pause", "--device", "Living Room"]);
    assert!(ok);
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_media_volume_sends_volume_command() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post("/api/np/command", 200, "{}").await;

    let (ok, _, _) = run_binary(
        &wm.url,
        &["media", "volume", "60", "--device", "Living Room"],
    );
    assert!(ok, "media volume should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_media_music_sends_behavior_preview() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post("/api/behaviors/preview", 200, "{}").await;

    let (ok, _, _) = run_binary(
        &wm.url,
        &[
            "media",
            "music",
            "classic rock",
            "--device",
            "Living Room",
            "--service",
            "spotify",
        ],
    );
    assert!(ok, "media music should succeed");
}
