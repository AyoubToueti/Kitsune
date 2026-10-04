//! The loopback server that serves the trailer embed page.
//!
//! YouTube's embedded player refuses to configure for a document with no valid
//! HTTP(S) referer. On Linux and macOS Tauri serves the app from
//! `tauri://localhost`, a custom scheme that sends none, so every trailer embed
//! fails with Error 153. Serving the embed page itself over `http://127.0.0.1`
//! gives the YouTube iframe a real HTTP referer -- a "potentially trustworthy"
//! origin the player accepts -- while the app keeps its own scheme and IPC.
//! See tauri-apps/tauri#14422.
//!
//! Deliberately tiny and dependency-free: a read-only GET that returns one HTML
//! page. It binds to `127.0.0.1` on an OS-assigned port, so it is never
//! reachable off the machine and never collides.

use std::net::TcpListener as StdTcpListener;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// The running embed server, managed as Tauri state.
///
/// Wrapped in an `Option` because the bind can fail; a missing server is not
/// fatal, it only means a YouTube trailer falls back to the direct embed (and
/// its Error 153) rather than taking the app down.
pub struct EmbedState(pub Option<EmbedServer>);

/// The loopback port the embed page is served on.
#[derive(Clone, Copy)]
pub struct EmbedServer {
    port: u16,
}

impl EmbedServer {
    /// The base URL a caller can append `/embed?v=<id>` to.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

/// The base URL for the embed page, or `null` when the server is not running.
#[tauri::command]
pub fn trailer_embed_base(state: tauri::State<'_, EmbedState>) -> Option<String> {
    state.0.as_ref().map(EmbedServer::base_url)
}

/// Bind a loopback port and spawn the accept loop.
///
/// Returns `None` when the socket cannot be bound. The bind is synchronous and
/// the listener is converted to a tokio one INSIDE the spawned task, because
/// `from_std` needs the reactor -- which is why this is not `async fn`.
pub fn start() -> Option<EmbedServer> {
    let listener = StdTcpListener::bind(("127.0.0.1", 0)).ok()?;
    listener.set_nonblocking(true).ok()?;
    let port = listener.local_addr().ok()?.port();

    tauri::async_runtime::spawn(async move {
        let Ok(listener) = TcpListener::from_std(listener) else {
            tracing::warn!("the trailer embed listener could not attach to the runtime");
            return;
        };

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    tauri::async_runtime::spawn(serve(stream));
                }
                Err(err) => {
                    tracing::warn!("trailer embed accept failed: {err}");
                }
            }
        }
    });

    Some(EmbedServer { port })
}

/// Answer one request. GET-only; the body is ignored.
async fn serve(mut stream: TcpStream) {
    let mut buf = [0u8; 4096];
    let read = match stream.read(&mut buf).await {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };

    // Only the request line is needed: the path carries everything.
    let request = String::from_utf8_lossy(&buf[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    let (status, body) = match embed_id(target) {
        Some(id) => ("200 OK", embed_page(&id)),
        None => ("404 Not Found", "Not found".to_string()),
    };

    let response = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Cache-Control: no-store\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        len = body.len(),
    );

    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// The video id from `GET /embed?v=<id>`, when the path and id are valid.
fn embed_id(target: &str) -> Option<String> {
    let (path, query) = target.split_once('?')?;
    if path != "/embed" {
        return None;
    }
    let id = param(query, "v")?;
    valid_id(&id).then_some(id)
}

/// The value of `key` in an `a=1&b=2` query string.
fn param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key).then(|| value.to_string())
    })
}

/// Whether `id` is safe to interpolate into the embed URL.
///
/// A YouTube id is `[A-Za-z0-9_-]`; anything else could break out of the URL or
/// the attribute, so it is rejected rather than escaped.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The embed page: a full-bleed YouTube iframe on a loopback origin.
fn embed_page(id: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Trailer</title>
<style>
  html, body {{ margin: 0; height: 100%; background: #000; overflow: hidden; }}
  iframe {{ display: block; width: 100%; height: 100%; border: 0; }}
</style>
</head>
<body>
<iframe
  src="https://www.youtube.com/embed/{id}?autoplay=1"
  title="Trailer"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
  allowfullscreen></iframe>
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_id_reads_the_video_id() {
        assert_eq!(embed_id("/embed?v=abc123"), Some("abc123".into()));
    }

    #[test]
    fn embed_id_rejects_a_foreign_path() {
        assert_eq!(embed_id("/other?v=abc123"), None);
        assert_eq!(embed_id("/embed"), None);
    }

    #[test]
    fn embed_id_rejects_an_unsafe_id() {
        // A quote could break out of the attribute, a slash out of the URL.
        assert_eq!(embed_id("/embed?v=a\"><script>"), None);
        assert_eq!(embed_id("/embed?v=a/b"), None);
        assert_eq!(embed_id("/embed?v="), None);
    }

    #[test]
    fn param_ignores_other_keys() {
        assert_eq!(param("a=1&v=xyz", "v"), Some("xyz".into()));
        assert_eq!(param("a=1&v=xyz", "z"), None);
        assert_eq!(param("", "v"), None);
    }

    #[test]
    fn embed_page_carries_the_id() {
        assert!(embed_page("abc123").contains("youtube.com/embed/abc123"));
    }

    #[test]
    fn base_url_is_loopback() {
        let server = EmbedServer { port: 1234 };
        assert_eq!(server.base_url(), "http://127.0.0.1:1234");
    }
}