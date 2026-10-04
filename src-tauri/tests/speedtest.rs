//! Integration test for the speed test against a mock Cloudflare endpoint.
//!
//! The pure helpers are unit tested inside `diagnostics::speedtest`; this
//! exercises the HTTP path -- the ramp-up, the chunked byte counting, and the
//! latency sampling -- without touching the real network. The config shrinks
//! every dimension so the test finishes in well under a second.

use std::sync::Mutex;
use std::time::Duration;

use kitsune_lib::diagnostics::speedtest::{
    build_client, run_measurement, SpeedTestConfig, SpeedTestProgress, SpeedVerdict,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn measurement_reports_latency_and_download_from_the_endpoint() {
    let server = MockServer::start().await;

    // Every `__down` request returns a fixed body. The requested size is
    // ignored here: the test asserts the measurement path works, not a
    // specific rate.
    Mock::given(method("GET"))
        .and(path("/__down"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 64 * 1024]))
        .mount(&server)
        .await;

    let client = build_client().expect("client builds");
    let url = format!("{}/__down", server.uri());

    // A tiny budget and few samples keep the test fast.
    let config = SpeedTestConfig {
        latency_samples: 3,
        first_payload_bytes: 64 * 1024,
        max_payload_bytes: 256 * 1024,
        download_budget: Duration::from_millis(300),
    };

    // Collect progress updates so the phases can be asserted.
    let updates: Mutex<Vec<SpeedTestProgress>> = Mutex::new(Vec::new());
    let result = run_measurement(&client, &url, &config, |progress| {
        updates.lock().expect("lock").push(progress);
    })
    .await
    .expect("measurement succeeds");

    // Latency was sampled, so it is a real positive number.
    assert!(result.latency_ms > 0.0, "latency: {}", result.latency_ms);
    // The download phase ran against real bytes, so it moved something.
    assert!(result.bytes_downloaded > 0, "no bytes downloaded");
    assert!(
        result.download_mbps > 0.0,
        "download: {}",
        result.download_mbps
    );
    assert!(result.duration_ms > 0, "duration: {}", result.duration_ms);

    // The phase sequence includes both phases and ends on "done".
    let phases: Vec<String> = updates
        .lock()
        .expect("lock")
        .iter()
        .map(|u| u.phase.clone())
        .collect();
    assert!(phases.iter().any(|p| p == "latency"), "phases: {phases:?}");
    assert!(phases.iter().any(|p| p == "download"), "phases: {phases:?}");
    assert_eq!(phases.last().map(String::as_str), Some("done"));

    // The verdict is one of the three known values.
    assert!(matches!(
        result.verdict,
        SpeedVerdict::Good | SpeedVerdict::Ok | SpeedVerdict::Poor
    ));
}