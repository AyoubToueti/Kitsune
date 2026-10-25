//! One running Kitsune, and a way to hand it a deep link.
//!
//! The problem this solves: `kitsune://auth#access_token=...` launches a NEW
//! process. Without this module the reader gets a second window (and, when the
//! scheme points at a stale build, a blank one). With it, the second process
//! forwards the URL to the first over the session bus and exits without ever
//! building a window.
//!
//! D-Bus is the right transport on Linux: every desktop session already runs a
//! bus, the name is a natural mutex ("whoever owns `com.kitsune.app` is the
//! running instance"), and no socket file or lock directory has to be managed
//! or cleaned up after a crash.
//!
//! A failure to reach the bus is NOT fatal. The app works fine as a normal
//! single window; the only thing lost is deep-link forwarding, and refusing to
//! start over that would be a poor trade.

use std::sync::{Arc, Mutex};

use zbus::{Connection, interface};

/// A one-slot mailbox for a forwarded deep link.
///
/// The bus service runs on its own task and cannot touch a `QObject` (the
/// `Backend` is created by QML, not by `main`, and cxx-qt offers no safe way to
/// reach it from another thread). So the service parks the URL here and the
/// Backend drains it when the window is activated -- see
/// `Backend::takePendingAuthUrl`.
///
/// A slot rather than a queue: only the newest link matters, and two redirects
/// in flight would be the same token twice.
pub type PendingAuth = Arc<Mutex<Option<String>>>;

/// The slot for this process.
///
/// A `OnceLock` rather than a parameter because the bus task is spawned before
/// the `Backend` exists, and threading a handle through both would add a
/// parameter to every caller for a value there is exactly one of.
pub fn pending() -> PendingAuth {
    static SLOT: std::sync::OnceLock<PendingAuth> = std::sync::OnceLock::new();
    Arc::clone(SLOT.get_or_init(|| Arc::new(Mutex::new(None))))
}

/// Callback invoked when a deep link arrives via D-Bus.
///
/// The Backend sets this to emit a signal that wakes the Qt thread, so the app
/// can auto-raise and drain the pending URL without waiting for manual focus.
pub type NotifyCallback = Arc<dyn Fn() + Send + Sync>;

/// The notification callback for this process.
///
/// Set by the Backend after construction. The D-Bus service calls it when a
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

/// The well-known bus name. Only one process can own it at a time, which is
/// exactly the "am I the first instance" question.
const BUS_NAME: &str = "com.kitsune.app";
const OBJECT_PATH: &str = "/com/kitsune/app";
const INTERFACE: &str = "com.kitsune.app.Auth";

/// The object the running instance exports.
///
/// `apply_auth_url` is the only method: it takes the redirect URL and hands it
/// to the Qt thread. The reply is whether a token was applied, so a forwarding
/// caller can log a useful outcome.
struct AuthService {
    slot: PendingAuth,
}

#[interface(name = "com.kitsune.app.Auth")]
impl AuthService {
    /// Park `url` for the running app to pick up.
    ///
    /// Returns whether the slot was free. `false` means a link is already
    /// waiting, which is not a failure -- the newer one replaces it below --
    /// but it tells the forwarder the app has not drained yet.
    async fn apply_auth_url(&self, url: String) -> bool {
        let mut slot = self.slot.lock().expect("pending-auth mutex poisoned");
        let was_empty = slot.is_none();
        *slot = Some(url);
        drop(slot); // Release lock before notifying
        notify(); // Wake the Qt thread to drain the pending URL
        was_empty
    }
}

/// Proof that this process owns the bus name.
///
/// Held for the life of the process: dropping it releases the name, which would
/// let a second instance believe it is the first.
pub struct Guard {
    _connection: Connection,
}

/// Try to become the running instance.
///
/// - `Ok(Some(guard))` -- this process is the first; it owns the name and
///   should serve `apply` for any later launch.
/// - `Ok(None)` -- another instance owns the name, so this process is a
///   forwarder: it should send its URL and exit.
/// - `Err(..)` -- the session bus is unavailable. Treated as "first instance"
///   by the caller, because a single-window app with no forwarding is still
///   correct, while refusing to start would not be.
pub async fn acquire() -> zbus::Result<Option<Guard>> {
    let service = AuthService { slot: pending() };
    let connection = Connection::session().await?;

    // Register the service object BEFORE requesting the name. zbus warns if
    // the name is acquired first: method calls arriving before the interface
    // is registered may be lost. With the object server ready, any forwarded
    // `ApplyAuthUrl` call is answered immediately once the name is claimed.
    connection
        .object_server()
        .at(OBJECT_PATH, service)
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
/// Best-effort: a failed forward is logged and swallowed. The reader sees the
/// same thing either way (their browser tab returns to an app that did not
/// react), and there is no useful action to offer.
pub async fn forward(url: &str) -> bool {
    let Ok(connection) = Connection::session().await else {
        return false;
    };

    let proxy = match zbus::Proxy::new(
        &connection,
        BUS_NAME,
        OBJECT_PATH,
        INTERFACE,
    )
    .await
    {
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

/// The first `kitsune://` URL among the arguments, if any.
///
/// Separate from [`acquire`] so it can be tested without a bus. Only the
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_redirect_url() {
        let args = ["/usr/bin/kitsune-qt", "kitsune://auth#access_token=abc"];
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
        let args = ["/usr/bin/kitsune-qt", "--help", "file.mkv"];
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