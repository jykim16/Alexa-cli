/// Integration tests for the `timer` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_timers -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const DEVICES_JSON: &str = r#"{"devices":[
    {"accountName":"Kitchen Echo","deviceFamily":"ECHO","deviceType":"A3FX4UWBHEQKAL",
     "serialNumber":"SN-KIT","online":true}
]}"#;

const TIMERS_JSON: &str = r#"{"timers":[
    {"id":"timer-1","timerLabel":"pasta","deviceSerialNumber":"SN-KIT",
     "originalDurationSeconds":600,"remainingTime":523,"status":"ON"},
    {"id":"timer-2","timerLabel":"bread","deviceSerialNumber":"SN-KIT",
     "originalDurationSeconds":3600,"remainingTime":0,"status":"PAUSED"}
]}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_timer_list_returns_json_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/timers/running.*", 200, TIMERS_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["timer", "list", "--output", "json"]);
    assert!(ok);
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], "timer-1");
    assert_eq!(arr[0]["timerLabel"], "pasta");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_timer_create_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON)
        .await;
    wm.stub_post("/api/timers", 200, r#"{"id":"timer-new"}"#)
        .await;

    let (ok, _, _) = run_binary(
        &wm.url,
        &[
            "timer",
            "create",
            "--duration",
            "10m",
            "--device",
            "Kitchen",
        ],
    );
    assert!(ok, "timer create should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_timer_cancel_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_delete("/api/timers/timer-1", 200).await;

    let (ok, _, _) = run_binary(&wm.url, &["timer", "cancel", "timer-1"]);
    assert!(ok, "timer cancel should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_timer_pause_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_put("/api/timers/timer-1/pause", 200, "").await;

    let (ok, _, _) = run_binary(&wm.url, &["timer", "pause", "timer-1"]);
    assert!(ok, "timer pause should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_timer_resume_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_put("/api/timers/timer-1/resume", 200, "").await;

    let (ok, _, _) = run_binary(&wm.url, &["timer", "resume", "timer-1"]);
    assert!(ok, "timer resume should succeed");
}
