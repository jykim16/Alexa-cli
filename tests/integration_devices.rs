/// Integration tests for the `devices` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_devices -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const DEVICES_JSON: &str = r#"{"devices":[
    {"accountName":"Living Room Echo","deviceFamily":"ECHO","deviceType":"A3C9PE6TNYLTCH",
     "serialNumber":"SN001","online":true,"softwareVersion":"1.2.3"},
    {"accountName":"Kitchen Dot","deviceFamily":"ECHO","deviceType":"A3FX4UWBHEQKAL",
     "serialNumber":"SN002","online":false}
]}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_devices_list_returns_json_array() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;

    let (ok, stdout, _stderr) = run_binary(&wm.url, &["devices", "list", "--output", "json"]);
    assert!(ok, "devices list should succeed");
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n---\n{stdout}"));
    let arr = json.as_array().expect("expected JSON array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["accountName"], "Living Room Echo");
    assert_eq!(arr[1]["accountName"], "Kitchen Dot");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_devices_get_returns_device_info() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;

    let (ok, stdout, _) = run_binary(
        &wm.url,
        &["devices", "get", "--device", "Kitchen", "--output", "json"],
    );
    assert!(ok, "devices get should succeed");
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("expected JSON output");
    assert_eq!(json["accountName"], "Kitchen Dot");
    assert_eq!(json["serialNumber"], "SN002");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_devices_list_session_expired_exits_nonzero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 401, "unauthorized").await;

    let (ok, _stdout, _stderr) = run_binary(&wm.url, &["devices", "list"]);
    assert!(!ok, "devices list on 401 should fail");
}
