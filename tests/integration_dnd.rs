/// Integration tests for the `dnd` (Do Not Disturb) command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_dnd -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const DEVICES_JSON: &str = r#"{"devices":[
    {"accountName":"Bedroom Echo","deviceFamily":"ECHO","deviceType":"A3C9PE6TNYLTCH",
     "serialNumber":"SN-BED","online":true}
]}"#;

const DND_STATUS_JSON: &str = r#"{"doNotDisturbDeviceStatusList":[
    {"deviceSerialNumber":"SN-BED","deviceType":"A3C9PE6TNYLTCH","enabled":true},
    {"deviceSerialNumber":"SN-KIT","deviceType":"A3FX4UWBHEQKAL","enabled":false}
]}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_dnd_status_returns_device_list() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/dnd/status.*", 200, DND_STATUS_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["dnd", "status", "--output", "json"]);
    assert!(ok, "dnd status should succeed");
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["deviceSerialNumber"], "SN-BED");
    assert_eq!(arr[0]["enabled"], true);
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_dnd_enable_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_put("/api/dnd/status", 200, r#"{"enabled":true}"#).await;

    let (ok, _, _) = run_binary(&wm.url, &["dnd", "enable", "--device", "Bedroom"]);
    assert!(ok, "dnd enable should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_dnd_disable_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/devices-v2/device.*", 200, DEVICES_JSON).await;
    wm.stub_put("/api/dnd/status", 200, r#"{"enabled":false}"#).await;

    let (ok, _, _) = run_binary(&wm.url, &["dnd", "disable", "--device", "Bedroom"]);
    assert!(ok, "dnd disable should succeed");
}
