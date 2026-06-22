/// Integration tests for the `routine` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_routines -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const AUTOMATIONS_JSON: &str = r#"[
    {"automationId":"r1","name":"Good Morning","status":"ENABLED",
     "sequence":{"sequenceJson":"{\"startNode\":{\"type\":\"AlexaSpeak\"}}"}},
    {"automationId":"r2","name":"Good Night","status":"ENABLED",
     "sequence":{"sequenceJson":"{\"startNode\":{\"type\":\"AlexaSpeak\"}}"}}
]"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_routine_list_returns_json_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/behaviors/automations.*", 200, AUTOMATIONS_JSON)
        .await;

    let (ok, stdout, _) = run_binary(&wm.url, &["routine", "list", "--output", "json"]);
    assert!(ok, "routine list should succeed");
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "Good Morning");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_routine_run_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/behaviors/automations.*", 200, AUTOMATIONS_JSON)
        .await;
    wm.stub_post("/api/behaviors/preview", 200, "{}").await;

    let (ok, _, _) = run_binary(&wm.url, &["routine", "run", "Good Morning"]);
    assert!(ok, "routine run should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_routine_run_not_found_exits_nonzero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/behaviors/automations.*", 200, AUTOMATIONS_JSON)
        .await;

    let (ok, _, _) = run_binary(&wm.url, &["routine", "run", "Nonexistent Routine"]);
    assert!(!ok, "routine run with unknown name should fail");
}
