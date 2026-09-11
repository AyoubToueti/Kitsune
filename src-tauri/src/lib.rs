pub mod torrent;
pub mod types;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_includes_the_name() {
        let message = greet("World");
        assert!(
            message.contains("World"),
            "greeting should include the name, got: {message}"
        );
    }

    #[test]
    fn greet_uses_expected_prefix() {
        assert!(greet("Ayoub").starts_with("Hello, Ayoub!"));
    }

    /// Proves the `tempfile` dev-dependency is wired up and usable.
    #[test]
    fn tempfile_harness_works() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let file = dir.path().join("probe.txt");
        std::fs::write(&file, b"ok").expect("failed to write probe file");
        assert_eq!(std::fs::read(&file).expect("failed to read probe file"), b"ok");
    }

    /// Proves the async test runtime works via `tokio-test`.
    #[test]
    fn tokio_test_harness_works() {
        let value = tokio_test::block_on(async { 1 + 1 });
        assert_eq!(value, 2);
    }

    /// Proves the `wiremock` dev-dependency compiles and can start a server.
    /// Real request/response mocking is exercised in the provider tests.
    #[tokio::test]
    async fn wiremock_harness_starts_a_server() {
        let server = wiremock::MockServer::start().await;
        assert!(
            server.uri().starts_with("http://"),
            "unexpected mock server uri: {}",
            server.uri()
        );
    }
}
