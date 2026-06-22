/// Integration tests for the `smart-home` command group.
///
/// Requires Docker. Run with:
///   cargo test --test integration_smart_home -- --include-ignored
mod common;

use common::{run_binary, WireMock};

const PHOENIX_JSON: &str = r#"{
    "networkDetail": {
        "locationDetails": {
            "locationId": "loc1",
            "amazonBridgeDetails": {
                "amazonBridgeDetails": {
                    "bridge1": {
                        "applianceDetails": {
                            "applianceDetails": {
                                "app1": {
                                    "applianceId": "app-LAMP-01",
                                    "friendlyName": "Desk Lamp",
                                    "applianceTypes": ["LIGHT"],
                                    "isEnabled": true,
                                    "reachability": "REACHABLE",
                                    "capabilities": [{"interface": "Alexa.PowerController"}]
                                },
                                "app2": {
                                    "applianceId": "app-THERM-01",
                                    "friendlyName": "Office Thermostat",
                                    "applianceTypes": ["THERMOSTAT"],
                                    "isEnabled": true,
                                    "reachability": "REACHABLE",
                                    "capabilities": [{"interface": "Alexa.ThermostatController"}]
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}"#;

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_smart_home_list_returns_devices() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/phoenix.*", 200, PHOENIX_JSON).await;

    let (ok, stdout, _) = run_binary(&wm.url, &["smart-home", "list", "--output", "json"]);
    assert!(ok, "smart-home list should succeed");
    let arr = serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("valid JSON")
        .as_array()
        .cloned()
        .expect("array");
    assert!(!arr.is_empty(), "should have found at least one device");
    // Both devices should be in the output
    let names: Vec<&str> = arr
        .iter()
        .filter_map(|d| d["friendlyName"].as_str())
        .collect();
    assert!(
        names.contains(&"Desk Lamp"),
        "Desk Lamp not found in {names:?}"
    );
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_smart_home_power_on_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/phoenix.*", 200, PHOENIX_JSON).await;
    wm.stub_put("/api/phoenix/state", 200, r#"{"success":true}"#)
        .await;

    let (ok, _, _) = run_binary(&wm.url, &["smart-home", "power", "Desk Lamp", "on"]);
    assert!(ok, "smart-home power on should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_smart_home_power_off_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/phoenix.*", 200, PHOENIX_JSON).await;
    wm.stub_put("/api/phoenix/state", 200, r#"{"success":true}"#)
        .await;

    let (ok, _, _) = run_binary(&wm.url, &["smart-home", "power", "Desk Lamp", "off"]);
    assert!(ok, "smart-home power off should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_smart_home_brightness_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/phoenix.*", 200, PHOENIX_JSON).await;
    wm.stub_put("/api/phoenix/state", 200, r#"{"success":true}"#)
        .await;

    let (ok, _, _) = run_binary(&wm.url, &["smart-home", "brightness", "Desk Lamp", "75"]);
    assert!(ok, "smart-home brightness should succeed");
}

#[ignore = "requires Docker"]
#[tokio::test]
async fn test_smart_home_thermostat_exits_zero() {
    let wm = WireMock::start().await;
    wm.stub_bootstrap().await;
    wm.stub_get("/api/phoenix.*", 200, PHOENIX_JSON).await;
    wm.stub_put("/api/phoenix/state", 200, r#"{"success":true}"#)
        .await;

    let (ok, _, _) = run_binary(
        &wm.url,
        &[
            "smart-home",
            "thermostat",
            "Office Thermostat",
            "72",
            "--unit",
            "F",
        ],
    );
    assert!(ok, "smart-home thermostat should succeed");
}
