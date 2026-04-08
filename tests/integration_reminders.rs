/// Integration tests for the `reminder` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_reminders -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const DEVICES_JSON: &str = r#"{"devices":[
    {"accountName":"Office Echo","deviceFamily":"ECHO","deviceType":"A3C9PE6TNYLTCH",
     "serialNumber":"SN-OFF","online":true}
]}"#;

const REMINDERS_JSON: &str = r#"{"reminders":[
    {"id":"rem-1","reminderLabel":"Take medication","status":"ON","triggerTime":1714100000},
    {"id":"rem-2","reminderLabel":"Call mom","status":"ON","triggerTime":1714200000}
]}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_reminder_list_returns_json_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_get("/api/reminders/device.*", 200, REMINDERS_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["reminder", "list", "--device", "Office", "--output", "json"]);
    assert!(ok, "reminder list should succeed");
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["reminderLabel"], "Take medication");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_reminder_create_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_post(
        "/api/reminders",
        200,
        r#"{"id":"rem-new","reminderLabel":"Team meeting","status":"ON"}"#,
    )
    .await;

    let (ok, _, _) = run_binary(
        &wm.url,
        &[
            "reminder",
            "create",
            "--text",
            "Team meeting",
            "--time",
            "2026-05-01T14:00:00Z",
            "--device",
            "Office",
        ],
    );
    assert!(ok, "reminder create should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_reminder_delete_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_delete("/api/reminders/rem-1", 200).await;

    let (ok, _, _) = run_binary(&wm.url, &["reminder", "delete", "rem-1"]);
    assert!(ok, "reminder delete should succeed");
}
