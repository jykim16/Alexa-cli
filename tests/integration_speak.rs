/// Integration tests for the `speak` command group (TTS + announcements).
///
/// Requires Docker. Run with:
///   cargo test --test integration_speak -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const DEVICES_JSON: &str = r#"{"devices":[
    {"accountName":"Office Echo","deviceFamily":"ECHO","deviceType":"A3C9PE6TNYLTCH",
     "serialNumber":"SN-OFF","online":true}
]}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_speak_say_sends_behavior_preview() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post("/api/behaviors/preview", 200, "{}").await;

    let (ok, _, _) = run_binary(
        &wm.url,
        &[
            "speak",
            "say",
            "Build completed successfully",
            "--device",
            "Office",
        ],
    );
    assert!(ok, "speak say should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_speak_announce_sends_behavior_preview() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post("/api/behaviors/preview", 200, "{}").await;

    let (ok, _, _) = run_binary(
        &wm.url,
        &["speak", "announce", "Dinner is ready"],
    );
    assert!(ok, "speak announce should succeed");
}
