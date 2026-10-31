//! Fetching a `.torrent` file from an indexer to a path the reader chose.
//!
//! The fetch happens in the backend rather than the frontend: the webview
//! cannot reach nyaa.si directly (CORS), and a native request can write the
//! bytes straight to disk instead of holding them in a JS string. The Qt
//! frontend needs the same ability, so the logic lives here rather than in
//! the Tauri command layer.

use std::path::Path;

/// Why a torrent download failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    /// The URL was not `http(s)`.
    InvalidUrl(String),
    /// The request itself failed (DNS, connection, timeout).
    Transport(String),
    /// The server answered with a non-success status.
    Status(u16),
    /// The bytes could not be written to disk.
    Write(String),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl(url) => write!(f, "not a downloadable URL: {url}"),
            Self::Transport(msg) => write!(f, "could not reach the indexer: {msg}"),
            Self::Status(code) => write!(f, "the indexer answered with status {code}"),
            Self::Write(msg) => write!(f, "could not write the torrent file: {msg}"),
        }
    }
}

impl std::error::Error for DownloadError {}

/// Fetch `url` and write the body to `path`.
///
/// `url` is validated to be `http(s)` before the request, so a `file://` or
/// other scheme smuggled in from a feed cannot be used to read local files.
///
/// Split from any command wrapper so the fetch-and-write can be tested against
/// a mock server without a UI framework in the way.
pub async fn download_to(url: &str, path: &Path) -> Result<(), DownloadError> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(DownloadError::InvalidUrl(url.to_string()));
    }

    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| DownloadError::Transport(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(DownloadError::Status(status.as_u16()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| DownloadError::Transport(e.to_string()))?;

    std::fs::write(path, &bytes).map_err(|e| DownloadError::Write(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn download_rejects_a_non_http_url() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("release.torrent");

        let result = download_to("file:///etc/passwd", &path).await;
        assert!(matches!(result, Err(DownloadError::InvalidUrl(_))));
        // Nothing was written.
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn download_writes_the_response_body_to_disk() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"d8:announce".to_vec()))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("release.torrent");

        download_to(&format!("{}/download/1.torrent", server.uri()), &path)
            .await
            .expect("should download");

        let written = std::fs::read(&path).expect("read back");
        assert_eq!(written, b"d8:announce");
    }

    #[tokio::test]
    async fn download_surfaces_a_non_success_status() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("release.torrent");

        let result = download_to(&format!("{}/missing.torrent", server.uri()), &path).await;
        assert!(matches!(result, Err(DownloadError::Status(404))));
        assert!(!path.exists());
    }
}