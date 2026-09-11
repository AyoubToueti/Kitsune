//! Local HTTP bridge that serves torrent pieces to a video player.
//!
//! Rather than hand-rolling a piece-waiting, seek-aware byte server, this
//! mounts `librqbit`'s own HTTP API (`feature = "http-api"`), which already
//! streams with `Range` support and blocks on missing pieces. See the
//! upstream route:
//!
//! ```text
//! GET /torrents/{id_or_infohash}/stream/{file_idx}
//! ```
//!
//! The server is bound to loopback only, and runs read-only: torrents are
//! added through [`super::engine::TorrentEngine`], which holds the session
//! directly, so the HTTP surface never needs to mutate state. That keeps
//! torrent creation off the network even though the API would allow it.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use librqbit::http_api::{HttpApi, HttpApiOptions};
use librqbit::Api;
use librqbit_dualstack_sockets::TcpListener;
use tokio::task::JoinHandle;

/// Port the stream server listens on by default.
///
/// Matches rqbit's own default so anyone testing with the `rqbit` CLI sees
/// the same URLs, and so the number is not a new invention to remember.
pub const DEFAULT_STREAM_PORT: u16 = 3030;

/// Address the bridge binds to. Loopback only, never `0.0.0.0`.
pub fn bind_addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

/// Build the URL a player should open for one file of one torrent.
///
/// Pure so the path shape can be asserted without starting a server; the
/// route is owned by librqbit and must match exactly.
pub fn stream_url(port: u16, torrent_id: usize, file_idx: usize) -> String {
    format!("http://127.0.0.1:{port}/torrents/{torrent_id}/stream/{file_idx}")
}

/// Options for the bridge server.
///
/// Extracted so the security-relevant flags are asserted in tests rather
/// than buried in a call site.
pub fn bridge_options() -> HttpApiOptions {
    HttpApiOptions {
        // Serve streams only; never mutate torrent state over HTTP.
        read_only: true,
        // Explicit, even though read_only already implies no creation.
        allow_create: false,
        // No auth: the socket is loopback-only, so nothing off-machine can
        // reach it, and anything on-machine already has the files.
        basic_auth: None,
        ..Default::default()
    }
}

/// A running bridge server.
pub struct HttpBridge {
    port: u16,
    task: JoinHandle<()>,
}

impl HttpBridge {
    /// Bind the loopback listener and start serving.
    ///
    /// Must be called from within a Tokio runtime: the server future is
    /// spawned as a background task.
    ///
    /// Pass port `0` to let the OS pick a free port; read it back with
    /// [`HttpBridge::port`].
    pub async fn start(api: Api, port: u16) -> Result<Self> {
        let addr = bind_addr(port);

        let listener = TcpListener::bind_tcp(addr, Default::default())
            .with_context(|| format!("failed to bind stream server on {addr}"))?;

        // Read the port back from the listener rather than trusting the
        // request: passing 0 asks the OS to choose, and callers need the
        // real port in order to build stream URLs.
        let bound_port = listener.bind_addr().port();

        let server = HttpApi::new(api, Some(bridge_options()))
            // No UPnP router: nothing outside this machine should reach it.
            .make_http_api_and_run(listener, None);

        // A failure here would otherwise vanish silently, so log it rather
        // than dropping the error.
        let task = tokio::spawn(async move {
            if let Err(err) = server.await {
                tracing::error!("stream server stopped: {err:#}");
            }
        });

        Ok(Self {
            port: bound_port,
            task,
        })
    }

    /// Port the bridge is actually listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Convenience: the URL to open for a given torrent file.
    pub fn url_for(&self, torrent_id: usize, file_idx: usize) -> String {
        stream_url(self.port, torrent_id, file_idx)
    }

    /// Stop serving.
    ///
    /// Aborts the server task; the socket is released when the task drops.
    pub fn shutdown(self) {
        self.task.abort();
    }
}

/// Build an `Api` facade over a session.
pub fn api_for(session: Arc<librqbit::Session>) -> Api {
    // Both optional channels serve debug endpoints we do not expose:
    // RUST_LOG reloading and log-line streaming. A read-only loopback
    // bridge needs neither.
    Api::new(session, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_url_matches_the_librqbit_route() {
        assert_eq!(
            stream_url(3030, 7, 0),
            "http://127.0.0.1:3030/torrents/7/stream/0"
        );
    }

    #[test]
    fn stream_url_handles_multi_digit_indices() {
        assert_eq!(
            stream_url(8765, 123, 45),
            "http://127.0.0.1:8765/torrents/123/stream/45"
        );
    }

    #[test]
    fn bind_addr_is_loopback_only() {
        let addr = bind_addr(3030);
        assert!(addr.ip().is_loopback(), "must not bind a public interface");
        assert_eq!(addr.port(), 3030);
    }

    #[test]
    fn bind_addr_refuses_wildcard_semantics() {
        // Guards against a future edit swapping 127.0.0.1 for 0.0.0.0.
        assert!(!bind_addr(3030).ip().is_unspecified());
    }

    #[test]
    fn default_port_matches_rqbit() {
        assert_eq!(DEFAULT_STREAM_PORT, 3030);
    }

    #[test]
    fn bridge_is_read_only() {
        assert!(bridge_options().read_only);
    }

    #[test]
    fn bridge_disallows_torrent_creation() {
        assert!(!bridge_options().allow_create);
    }

    #[test]
    fn bridge_has_no_basic_auth() {
        assert!(bridge_options().basic_auth.is_none());
    }

    /// Starts a real listener and issues a real HTTP request against it.
    ///
    /// This is the end-to-end proof that the bridge actually serves: it
    /// would catch a wrong route, a failed bind, or a server that dies at
    /// startup, none of which the pure tests above can detect.
    ///
    /// Ignored by default because it binds a socket. Run with:
    /// `cargo test -- --ignored`
    #[tokio::test]
    #[ignore = "binds a socket; run explicitly"]
    async fn bridge_serves_http_and_shuts_down() {
        let dir = tempfile::tempdir().unwrap();
        let session = librqbit::Session::new(dir.path().to_path_buf())
            .await
            .expect("session should start");

        let api = api_for(session.clone());

        // Port 0 lets the OS pick a free port, avoiding collisions.
        let bridge = HttpBridge::start(api, 0)
            .await
            .expect("bridge should start");

        // The reported port must be the real one the OS chose, not 0, or
        // every stream URL built from it would be broken.
        let port = bridge.port();
        assert_ne!(port, 0, "should report the OS-assigned port");

        let url = format!("http://127.0.0.1:{port}/");
        let response = reqwest::get(&url)
            .await
            .expect("request to the bridge should succeed");

        assert!(
            response.status().is_success(),
            "bridge root should respond 2xx, got {}",
            response.status()
        );

        // The root endpoint lists the available API routes, so the body
        // should mention the streaming route we depend on.
        let body = response.text().await.expect("body should be readable");
        assert!(
            body.contains("stream"),
            "API listing should mention the stream route, got: {body}"
        );

        bridge.shutdown();
        session.stop().await;
    }
}