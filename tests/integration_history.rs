/// Integration tests for the `history` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_history -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const HISTORY_JSON: &str = r#"{"activities":[
    {"id":"act-1","creationTimestamp":1714000000,
     "device":{"deviceName":"Kitchen Echo","deviceSerialNumber":"SN-KIT"},
     "description":{"summaryText":"play some music"}},
    {"id":"act-2","creationTimestamp":1714000100,
     "device":{"deviceName":"Bedroom Echo","deviceSerialNumber":"SN-BED"},
     "description":{"summaryText":"what is the weather"}}
]}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_history_list_returns_activities_json() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/activities.*", 200, HISTORY_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["history", "list", "--output", "json"]);
    assert!(ok, "history list should succeed");
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], "act-1");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_history_list_with_limit_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/activities.*", 200, HISTORY_JSON).await;

    let (ok, stdout, _) = run_binary(
        &wm.url,
        &["history", "list", "--limit", "1", "--output", "json"],
    );
    assert!(ok, "history list --limit should succeed");
    // Might return up to 1 item depending on server response
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert!(!arr.is_empty());
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_history_list_empty_returns_empty_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/activities.*", 200, r#"{"activities":[]}"#)
        .await;

    let (ok, stdout, _) = run_binary(&wm.url, &["history", "list", "--output", "json"]);
    assert!(ok);
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert!(arr.is_empty());
}
