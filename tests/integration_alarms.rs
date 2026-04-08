/// Integration tests for the `alarm` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_alarms -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const DEVICES_JSON: &str = r#"{"devices":[
    {"accountName":"Bedroom Echo","deviceFamily":"ECHO","deviceType":"A3C9PE6TNYLTCH",
     "serialNumber":"SN-BED","online":true}
]}"#;

const ALARMS_JSON: &str = r#"{"alarms":[
    {"id":"alarm-1","deviceSerialNumber":"SN-BED","deviceType":"A3C9PE6TNYLTCH",
     "dateTime":"2026-05-01T07:00:00.000","status":"ON","type":"Alarm"},
    {"id":"alarm-2","deviceSerialNumber":"SN-BED","deviceType":"A3C9PE6TNYLTCH",
     "dateTime":"2026-05-01T08:00:00.000","status":"OFF","type":"Alarm"}
]}"#;

const NEW_ALARM_JSON: &str = r#"{"id":"alarm-new","deviceSerialNumber":"SN-BED",
    "deviceType":"A3C9PE6TNYLTCH","dateTime":"2026-05-01T09:30:00.000","status":"ON","type":"Alarm"}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_alarm_list_returns_json_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/alerts/running.*", 200, ALARMS_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["alarm", "list", "--output", "json"]);
    assert!(ok, "alarm list should succeed");
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("not JSON: {stdout}"));
    let arr = json.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], "alarm-1");
    assert_eq!(arr[0]["status"], "ON");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_alarm_create_returns_new_alarm_json() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post("/api/alarms", 200, NEW_ALARM_JSON).await;

    let (ok, stdout, _) = run_binary(
        &wm.url,
        &[
            "alarm",
            "create",
            "--time",
            "09:30",
            "--label",
            "Morning standup",
            "--device",
            "Bedroom",
            "--output",
            "json",
        ],
    );
    assert!(ok, "alarm create should succeed");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["id"], "alarm-new");
    assert_eq!(json["status"], "ON");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_alarm_delete_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_delete("/api/alarms/alarm-1", 200).await;

    let (ok, _, _) = run_binary(&wm.url, &["alarm", "delete", "alarm-1"]);
    assert!(ok, "alarm delete should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_alarm_enable_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_put("/api/alarms/alarm-1", 200, r#"{"status":"ON"}"#).await;

    let (ok, _, _) = run_binary(&wm.url, &["alarm", "enable", "alarm-1"]);
    assert!(ok, "alarm enable should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_alarm_disable_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_put("/api/alarms/alarm-1", 200, r#"{"status":"OFF"}"#).await;

    let (ok, _, _) = run_binary(&wm.url, &["alarm", "disable", "alarm-1"]);
    assert!(ok, "alarm disable should succeed");
}
