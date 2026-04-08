/// Integration tests for `shopping` and `todo` command groups.
///
/// Requires Docker. Run with:
///   cargo test --test integration_lists -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const SHOPPING_JSON: &str = r#"{"values":[
    {"itemId":"s1","value":"milk","completed":false},
    {"itemId":"s2","value":"eggs","completed":false},
    {"itemId":"s3","value":"bread","completed":true}
]}"#;

const TODO_JSON: &str = r#"{"values":[
    {"itemId":"t1","value":"call dentist","completed":false},
    {"itemId":"t2","value":"buy stamps","completed":true}
]}"#;

// ── Shopping ──────────────────────────────────────────────────────────────

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_shopping_list_returns_json_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/todos.*SHOPPING_ITEM.*", 200, SHOPPING_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["shopping", "list", "--output", "json"]);
    assert!(ok);
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["value"], "milk");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_shopping_add_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_post("/api/todos", 200, r#"{"itemId":"s-new","value":"butter"}"#)
        .await;

    let (ok, _, _) = run_binary(&wm.url, &["shopping", "add", "butter"]);
    assert!(ok, "shopping add should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_shopping_remove_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_delete("/api/todos/s1", 200).await;

    let (ok, _, _) = run_binary(&wm.url, &["shopping", "remove", "s1"]);
    assert!(ok, "shopping remove should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_shopping_clear_deletes_all_items() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/todos.*SHOPPING_ITEM.*", 200, SHOPPING_JSON).await;
    // clear deletes each item individually
    wm.stub_delete("/api/todos/.*", 200).await;

    let (ok, _, _) = run_binary(&wm.url, &["shopping", "clear"]);
    assert!(ok, "shopping clear should succeed");
}

// ── Todo ──────────────────────────────────────────────────────────────────

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_todo_list_returns_json_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    // Use a catch-all for the todo endpoint (query string varies)
    wm.stub_get("/api/todos.*TASK.*", 200, TODO_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["todo", "list", "--output", "json"]);
    assert!(ok);
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["value"], "call dentist");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_todo_add_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_post("/api/todos", 200, r#"{"itemId":"t-new","value":"water plants"}"#)
        .await;

    let (ok, _, _) = run_binary(&wm.url, &["todo", "add", "water plants"]);
    assert!(ok, "todo add should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_todo_complete_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_put("/api/todos/t1", 200, r#"{"itemId":"t1","completed":true}"#)
        .await;

    let (ok, _, _) = run_binary(&wm.url, &["todo", "complete", "t1"]);
    assert!(ok, "todo complete should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_todo_remove_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_delete("/api/todos/t2", 200).await;

    let (ok, _, _) = run_binary(&wm.url, &["todo", "remove", "t2"]);
    assert!(ok, "todo remove should succeed");
}
