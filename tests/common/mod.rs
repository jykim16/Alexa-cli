/// Shared helpers for container-based integration tests.
///
/// These tests require a running Docker daemon.
///
/// Run all integration tests:
///   `cargo test --test integration_\* -- --include-ignored`
///
/// Or via docker-compose (recommended for CI):
///   `docker compose -f docker/docker-compose.test.yml run test-runner`
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

/// A running WireMock container with its base URL.
pub struct WireMock {
    pub url: String,
    // Holds the container alive for the test duration
    _container: ContainerAsync<GenericImage>,
}

impl WireMock {
    /// Start a WireMock 3 container and wait until it is ready to receive requests.
    pub async fn start() -> Self {
        let image = GenericImage::new("wiremock/wiremock", "3.9.1")
            .with_exposed_port(8080.tcp())
            // WireMock 3 prints this line to stderr when ready
            .with_wait_for(WaitFor::message_on_stderr("WireMock running on"));

        let container = image
            .start()
            .await
            .expect("Failed to start WireMock container — is Docker running?");

        let port = container
            .get_host_port_ipv4(8080)
            .await
            .expect("Failed to get WireMock host port");

        let url = format!("http://127.0.0.1:{}", port);
        Self { url, _container: container }
    }

    /// Register a stub mapping via the WireMock Admin REST API.
    pub async fn add_stub(&self, stub: serde_json::Value) {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/__admin/mappings", self.url))
            .json(&stub)
            .send()
            .await
            .expect("Failed to POST stub to WireMock admin");
        assert!(
            resp.status().is_success(),
            "WireMock rejected stub: {:?}",
            resp.text().await.unwrap_or_default()
        );
    }

    /// Stub a GET endpoint returning JSON.
    pub async fn stub_get(&self, url_pattern: &str, status: u16, body: &str) {
        self.add_stub(serde_json::json!({
            "request": { "method": "GET", "urlPathPattern": url_pattern },
            "response": {
                "status": status,
                "headers": { "Content-Type": "application/json" },
                "body": body
            }
        }))
        .await;
    }

    /// Stub a POST endpoint returning JSON.
    pub async fn stub_post(&self, url_pattern: &str, status: u16, body: &str) {
        self.add_stub(serde_json::json!({
            "request": { "method": "POST", "urlPathPattern": url_pattern },
            "response": {
                "status": status,
                "headers": { "Content-Type": "application/json" },
                "body": body
            }
        }))
        .await;
    }

    /// Stub a PUT endpoint returning JSON.
    pub async fn stub_put(&self, url_pattern: &str, status: u16, body: &str) {
        self.add_stub(serde_json::json!({
            "request": { "method": "PUT", "urlPathPattern": url_pattern },
            "response": {
                "status": status,
                "headers": { "Content-Type": "application/json" },
                "body": body
            }
        }))
        .await;
    }

    /// Stub a DELETE endpoint.
    pub async fn stub_delete(&self, url_pattern: &str, status: u16) {
        self.add_stub(serde_json::json!({
            "request": { "method": "DELETE", "urlPathPattern": url_pattern },
            "response": { "status": status }
        }))
        .await;
    }

    /// Register the `/api/bootstrap` stub that every `ApiClient::new()` call
    /// requires to fetch the CSRF token.  Must be called before `run_binary`.
    pub async fn stub_bootstrap(&self) {
        self.stub_get(
            "/api/bootstrap.*",
            200,
            r#"{"csrfToken":"integration-test-csrf"}"#,
        )
        .await;
    }
}

/// Run `alexa-cli <args>` as a subprocess with `ALEXA_BASE_URL` pointing at
/// the WireMock server.  Returns `(exit_success, stdout, stderr)`.
///
/// The compiled binary path is resolved at build time via the
/// `CARGO_BIN_EXE_alexa-cli` env var that Cargo sets for integration tests.
#[allow(dead_code)]
pub fn run_binary(wiremock_url: &str, args: &[&str]) -> (bool, String, String) {
    let binary = env!("CARGO_BIN_EXE_alexa-cli");
    let output = std::process::Command::new(binary)
        .args(args)
        .env("ALEXA_BASE_URL", wiremock_url)
        .output()
        .unwrap_or_else(|e| panic!("Failed to spawn {}: {}", binary, e));

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}
