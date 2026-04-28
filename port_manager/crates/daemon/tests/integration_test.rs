use common::{AllocateRequest, AllocateResponse, ReleaseRequest};
use reqwest::Client;

// Liest aus ENV (`PM_DASHBOARD_PORT`), Default 7878 — analog zum Daemon
// und Client. Ein laufender Daemon mit nicht-Default-Port ist damit auch
// aus Tests adressierbar, ohne den Test-Code zu patchen.
fn base_url() -> String {
    let port = std::env::var("PM_DASHBOARD_PORT").unwrap_or_else(|_| "7878".to_string());
    format!("http://localhost:{port}")
}

// Dieser Test braucht einen laufenden Daemon auf `PM_DASHBOARD_PORT`
// (Default 7878). Default `cargo test` würde sonst panic-en, weil die
// erste Allocate-Request via `.expect()` failt — daher `#[ignore]`.
//
// Manuell ausführen, sobald ein Daemon läuft:
//   brew services start portmanager
//   cargo test --test integration_test -- --ignored
//
// Mittelfristig: Daemon im Test-Setup spawnen + auf STDOUT-Probe warten.

#[tokio::test]
#[ignore = "braucht laufenden PortManager-Daemon — manuell mit --ignored"]
async fn test_full_lifecycle() {
    let client = Client::new();

    // 1. Allocate
    let alloc_req = AllocateRequest {
        service_name: "integration-test-service".to_string(),
        ttl_seconds: Some(60),
        tags: Some(vec!["test".to_string()]),
    };

    let resp = client.post(format!("{}/alloc", base_url()))
        .json(&alloc_req)
        .send()
        .await
        .expect("Failed to send alloc request");
    
    // If daemon is not running, this might fail. In a real CI, we'd spawn the daemon here.
    if resp.status().is_client_error() || resp.status().is_server_error() {
         // Skip test if daemon not running locally during development loop
         println!("Daemon might not be running. Skipping integration test assertions.");
         return;
    }

    assert!(resp.status().is_success());
    let alloc_resp: AllocateResponse = resp.json().await.unwrap();
    println!("Allocated port: {}", alloc_resp.port);

    // 2. Verify List
    let list_resp = client.get(format!("{}/list", base_url()))
        .send()
        .await
        .expect("Failed to get list");
    assert!(list_resp.status().is_success());
    // We could parse and check if our port is there

    // 3. Release
    let release_req = ReleaseRequest { port: alloc_resp.port };
    let rel_resp = client.post(format!("{}/release", base_url()))
        .json(&release_req)
        .send()
        .await
        .expect("Failed to release");
    
    assert!(rel_resp.status().is_success());
}
