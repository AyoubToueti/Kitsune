//! Install a panic hook that records crashes in the log.
//!
//! A release GUI build has no console, so a panic would otherwise leave the
//! user with a vanished window and no evidence of why. The hook chains the
//! previous (default) hook as well, so the normal stderr message still prints
//! in development.
//!
//! Note the release profile sets `panic = "abort"`: the hook still runs before
//! the abort, so the crash is recorded, but no `catch_unwind` can recover it.

use std::panic;

/// Install the logging panic hook.
///
/// Idempotent enough to call once at startup. Calling it twice would chain the
/// hook onto itself, so it is intended to be called exactly once.
pub fn install_panic_hook() {
    let previous = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        // Location and payload are the two things worth having; a backtrace is
        // left to the default hook's RUST_BACKTRACE handling.
        let location = info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown location".to_string());

        let payload = payload_message(info.payload());

        tracing::error!(target: "panic", location = %location, "panicked: {payload}");

        // Still run the default hook: it prints to stderr (useful in dev) and
        // honours RUST_BACKTRACE.
        previous(info);
    }));
}

/// The panic message as a string.
///
/// `payload` is usually a `&str` or `String`; anything else gets a placeholder
/// rather than a panic inside the panic hook.
fn payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_message_reads_a_str() {
        let payload: &(dyn std::any::Any + Send) = &"boom";
        assert_eq!(payload_message(payload), "boom");
    }

    #[test]
    fn payload_message_reads_a_string() {
        let owned = String::from("owned boom");
        let payload: &(dyn std::any::Any + Send) = &owned;
        assert_eq!(payload_message(payload), "owned boom");
    }

    #[test]
    fn payload_message_handles_a_non_string() {
        let value = 42u32;
        let payload: &(dyn std::any::Any + Send) = &value;
        assert_eq!(payload_message(payload), "<non-string panic payload>");
    }
}