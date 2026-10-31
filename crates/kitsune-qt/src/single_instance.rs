//! One running Kitsune, and a way to hand it a deep link.
//!
//! The problem this solves: `kitsune://auth#access_token=...` launches a NEW
//! process. Without this module the reader gets a second window (and, when the
//! scheme points at a stale build, a blank one). With it, the second process
//! forwards the URL to the first and exits without ever building a window.
//!
//! Two transports, because the platforms offer different primitives:
//!
//!   * **Unix** -- D-Bus. Every desktop session already runs a bus, the name is
//!     a natural mutex ("whoever owns `com.kitsune.app` is the running
//!     instance"), and no socket file or lock directory has to be cleaned up
//!     after a crash.
//!   * **Windows** -- there is no session bus, so a loopback TCP listener takes
//!     its place. Binding the port *is* the mutex: whoever binds `127.0.0.1`
//!     first owns the name, and a later process connects and writes the URL.
//!     No extra crate and no OS handle that outlives the process.
//!
//! In both cases a failure to acquire is NOT fatal. The app works fine as a
//! normal single window; the only thing lost is deep-link forwarding, and
//! refusing to start over that would be a poor trade.

use std::sync::{Arc, Mutex};

/// A one-slot mailbox for a forwarded deep link.
///
/// The transport runs on its own thread/task and cannot touch a `QObject` (the
/// `Backend` is created by QML, not by `main`, and cxx-qt offers no safe way to
/// reach it from another thread). So the transport parks the URL here and the
/// Backend drains it when the window is activated -- see
/// `Backend::takePendingAuthUrl`.
///
/// A slot rather than a queue: only the newest link matters, and two redirects
/// in flight would be the same token twice.
pub type PendingAuth = Arc<Mutex<Option<String>>>;

/// The slot for this process.
///
/// A `OnceLock` rather than a parameter because the transport task is spawned
/// before the `Backend` exists, and threading a handle through both would add a
/// parameter to every caller for a value there is exactly one of.
pub fn pending() -> PendingAuth {
    static SLOT: std::sync::OnceLock<PendingAuth> = std::sync::OnceLock::new();
    Arc::clone(SLOT.get_or_init(|| Arc::new(Mutex::new(None))))
}

/// Callback invoked when a deep link arrives.
///
/// The Backend sets this to emit a signal that wakes the Qt thread, so the app
/// can auto-raise and drain the pending URL without waiting for manual focus.
pub type NotifyCallback = Arc<dyn Fn() + Send + Sync>;

/// The notification callback for this process.
///
/// Set by the Backend after construction. The transport calls it when a
/// forwarded URL arrives, triggering immediate window activation.
static NOTIFY: std::sync::OnceLock<NotifyCallback> = std::sync::OnceLock::new();

pub fn set_notify(callback: NotifyCallback) {
    let _ = NOTIFY.set(callback);
}

fn notify() {
    if let Some(callback) = NOTIFY.get() {
        callback();
    }
}

/// Park `url` for the running app to pick up.
///
/// Returns whether the slot was free. `false` means a link is already waiting,
/// which is not a failure -- the newer one replaces it -- but it tells the
/// forwarder the app has not drained yet.
fn deliver(url: String) -> bool {
    let slot = pending();
    let mut slot = slot.lock().expect("pending-auth mutex poisoned");
    let was_empty = slot.is_none();
    *slot = Some(url);
    drop(slot); // Release lock before notifying
    notify(); // Wake the Qt thread to drain the pending URL
    was_empty
}

/// The first `kitsune://` URL among the arguments, if any.
///
/// Separate from [`acquire`] so it can be tested without a transport. Only the
/// arguments are scanned, never `argv[0]`, so an executable whose path happens
/// to contain the scheme cannot be mistaken for a redirect.
pub fn auth_url_from_args<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let prefix = format!("{}://", kitsune_core::auth::flow::REDIRECT_SCHEME);
    args.into_iter()
        .skip(1)
        .map(|arg| arg.as_ref().to_string())
        .find(|arg| arg.starts_with(&prefix))
}

// ---------------------------------------------------------------------------
// Unix: D-Bus
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod backend {
    use super::deliver;
    use zbus::{Connection, interface};

    /// The well-known bus name. Only one process can own it at a time, which is
    /// exactly the "am I the first instance" question.
    const BUS_NAME: &str = "com.kitsune.app";
    const OBJECT_PATH: &str = "/com/kitsune/app";
    const INTERFACE: &str = "com.kitsune.app.Auth";

    pub use zbus::Error;

    /// The object the running instance exports.
    ///
    /// `apply_auth_url` is the only method: it takes the redirect URL and hands
    /// it to the Qt thread. The reply is whether the slot was free, so a
    /// forwarding caller can log a useful outcome.
    struct AuthService;

    #[interface(name = "com.kitsune.app.Auth")]
    impl AuthService {
        /// Park `url` for the running app to pick up.
        async fn apply_auth_url(&self, url: String) -> bool {
            deliver(url)
        }
    }

    /// Proof that this process owns the bus name.
    ///
    /// Held for the life of the process: dropping it releases the name, which
    /// would let a second instance believe it is the first.
    pub struct Guard {
        _connection: Connection,
    }

    /// Try to become the running instance.
    ///
    /// - `Ok(Some(guard))` -- this process is the first; it owns the name and
    ///   should serve `apply` for any later launch.
    /// - `Ok(None)` -- another instance owns the name, so this process is a
    ///   forwarder: it should send its URL and exit.
    /// - `Err(..)` -- the session bus is unavailable. Treated as "first
    ///   instance" by the caller, because a single-window app with no
    ///   forwarding is still correct, while refusing to start would not be.
    pub async fn acquire() -> zbus::Result<Option<Guard>> {
        let connection = Connection::session().await?;

        // Register the service object BEFORE requesting the name. zbus warns if
        // the name is acquired first: method calls arriving before the
        // interface is registered may be lost. With the object server ready,
        // any forwarded `ApplyAuthUrl` call is answered immediately once the
        // name is claimed.
        connection
            .object_server()
            .at(OBJECT_PATH, AuthService)
            .await?;

        // `request_name` fails when the name is taken, which IS the answer we
        // want -- not an error to propagate.
        match connection.request_name(BUS_NAME).await {
            Ok(()) => {}
            Err(zbus::Error::NameTaken) => return Ok(None),
            Err(err) => return Err(err),
        }

        Ok(Some(Guard {
            _connection: connection,
        }))
    }

    /// Send `url` to the running instance.
    ///
    /// Best-effort: a failed forward is logged and swallowed. The reader sees
    /// the same thing either way (their browser tab returns to an app that did
    /// not react), and there is no useful action to offer.
    pub async fn forward(url: &str) -> bool {
        let Ok(connection) = Connection::session().await else {
            return false;
        };

        let proxy = match zbus::Proxy::new(&connection, BUS_NAME, OBJECT_PATH, INTERFACE).await {
            Ok(proxy) => proxy,
            Err(_) => return false,
        };

        proxy
            .call_method("ApplyAuthUrl", &(url,))
            .await
            .ok()
            .and_then(|reply| reply.body().deserialize::<bool>().ok())
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Windows: loopback TCP listener
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod backend {
    use super::deliver;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::time::Duration;

    /// Loopback port used as the single-instance mutex.
    ///
    /// Whoever binds it first is the running instance. Fixed (not ephemeral)
    /// because the forwarding process has no other way to find the listener;
    /// it is high enough to be well clear of system services.
    const PORT: u16 = 47823;

    /// First line a forwarder sends. The listener checks it so an unrelated
    /// program that happens to hold the port is never mistaken for Kitsune.
    const MAGIC: &str = "KITSUNE1";

    /// Reply when the URL was parked.
    const OK: &str = "OK";

    /// How long `forward` waits for the running instance to answer.
    const FORWARD_TIMEOUT: Duration = Duration::from_secs(3);

    /// Errors acquiring the single-instance lock.
    ///
    /// Deliberately a plain string: the only consumer logs it and carries on.
    #[derive(Debug)]
    pub struct Error(pub String);

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl std::error::Error for Error {}

    /// Proof that this process owns the loopback port.
    ///
    /// Dropping it closes the listener, which would let a second instance
    /// believe it is the first. Held for the life of the process.
    ///
    /// The inner `Option` exists for the degraded case described on
    /// [`acquire`]: we are the first instance but could not prove it.
    pub struct Guard {
        _listener: Option<TcpListener>,
    }

    fn addr() -> SocketAddr {
        SocketAddr::from((Ipv4Addr::LOCALHOST, PORT))
    }

    /// Try to become the running instance.
    ///
    /// - `Ok(Some(guard))` -- this process is the first and owns the port.
    /// - `Ok(None)` -- another Kitsune owns the port, so forward and exit.
    /// - `Err(..)` -- the port is held by something that is not us and not
    ///   recognisable. Treated as "first instance" by the caller so the window
    ///   still opens: refusing to start is worse than risking a second window.
    pub async fn acquire() -> Result<Option<Guard>, Error> {
        match TcpListener::bind(addr()) {
            Ok(listener) => {
                spawn_accept_loop(listener.try_clone().map_err(|e| Error(e.to_string()))?);
                Ok(Some(Guard {
                    _listener: Some(listener),
                }))
            }
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
                // Something holds the port. If it answers our handshake it is a
                // Kitsune instance and we are a forwarder; otherwise it is a
                // stranger and we must not silently suppress our own window.
                if probe().is_some() {
                    Ok(None)
                } else {
                    Err(Error(format!(
                        "loopback port {PORT} is held by another program"
                    )))
                }
            }
            Err(err) => Err(Error(err.to_string())),
        }
    }

    /// Send `url` to the running instance.
    ///
    /// Best-effort, like the Unix version: a failed forward is logged and
    /// swallowed because there is no useful action to offer the reader.
    pub async fn forward(url: &str) -> bool {
        let Ok(mut stream) = TcpStream::connect_timeout(&addr(), FORWARD_TIMEOUT) else {
            return false;
        };
        let _ = stream.set_read_timeout(Some(FORWARD_TIMEOUT));
        let _ = stream.set_write_timeout(Some(FORWARD_TIMEOUT));

        // Newlines are illegal in a URL fragment, so a line protocol is safe.
        if writeln!(stream, "{MAGIC} {url}").is_err() {
            return false;
        }
        let _ = stream.flush();

        let mut reply = String::new();
        let mut reader = BufReader::new(stream);
        if reader.read_line(&mut reply).is_err() {
            return false;
        }
        reply.trim() == OK
    }

    /// Ask the holder of the port to identify itself.
    ///
    /// `Some(true)` when it is a Kitsune listener that took the URL,
    /// `Some(false)` when it is a Kitsune listener that answered with
    /// something unexpected, `None` when it is not a Kitsune listener at all.
    fn probe() -> Option<bool> {
        let mut stream = TcpStream::connect_timeout(&addr(), FORWARD_TIMEOUT).ok()?;
        let _ = stream.set_read_timeout(Some(FORWARD_TIMEOUT));
        let _ = stream.set_write_timeout(Some(FORWARD_TIMEOUT));
        // An empty payload: the listener parks nothing and just replies, which
        // is enough to tell us who owns the port.
        writeln!(stream, "{MAGIC}").ok()?;
        let _ = stream.flush();

        let mut reply = String::new();
        BufReader::new(stream).read_line(&mut reply).ok()?;
        Some(reply.trim() == OK)
    }

    /// Accept connections forever on a background thread.
    ///
    /// A thread rather than a tokio task: this outlives the startup runtime's
    /// usefulness, and the work is a blocking accept loop anyway.
    fn spawn_accept_loop(listener: TcpListener) {
        std::thread::Builder::new()
            .name("kitsune-single-instance".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    // Handle each connection inline: they arrive one at a time
                    // (a second launch of the app) and each is a single line.
                    handle_connection(stream);
                }
            })
            .expect("could not spawn the single-instance listener thread");
    }

    fn handle_connection(mut stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(FORWARD_TIMEOUT));
        let _ = stream.set_write_timeout(Some(FORWARD_TIMEOUT));

        let mut line = String::new();
        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(clone) => clone,
            Err(_) => return,
        });
        if reader.read_line(&mut line).is_err() {
            return;
        }

        let line = line.trim_end();
        let Some(rest) = line.strip_prefix(MAGIC) else {
            // Not our protocol. Say nothing and drop the connection.
            return;
        };

        // `MAGIC` alone is the identity probe; anything after the space is a
        // URL to park.
        if let Some(url) = rest.strip_prefix(' ') {
            if !url.is_empty() {
                deliver(url.to_string());
            }
        }

        let _ = writeln!(stream, "{OK}");
        let _ = stream.flush();
    }
}

pub use backend::{acquire, forward};

// Re-exported so the return types of `acquire` are nameable by any caller that
// wants to store or match on them. Nothing in this crate names them today, so
// the import is allowed to look unused.
#[allow(unused_imports)]
pub use backend::{Error, Guard};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_redirect_url() {
        let args = ["/usr/bin/kitsune", "kitsune://auth#access_token=abc"];
        assert_eq!(
            auth_url_from_args(args).as_deref(),
            Some("kitsune://auth#access_token=abc")
        );
    }

    #[test]
    fn ignores_the_executable_path() {
        // `argv[0]` is the binary, and must never be read as a redirect even
        // if its path happens to contain the scheme.
        let args = ["kitsune://bin/app"];
        assert_eq!(auth_url_from_args(args), None);
    }

    #[test]
    fn ignores_unrelated_arguments() {
        let args = ["/usr/bin/kitsune", "--help", "file.mkv"];
        assert_eq!(auth_url_from_args(args), None);
    }

    #[test]
    fn picks_the_first_of_several() {
        let args = ["/bin/app", "kitsune://auth#a=1", "kitsune://auth#a=2"];
        assert_eq!(
            auth_url_from_args(args).as_deref(),
            Some("kitsune://auth#a=1")
        );
    }

    #[test]
    fn finds_a_url_after_other_flags() {
        let args = ["/bin/app", "--verbose", "kitsune://auth#access_token=x"];
        assert!(auth_url_from_args(args).is_some());
    }
}