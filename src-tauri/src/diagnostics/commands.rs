//! Tauri commands exposing diagnostics to the frontend.

use tauri::State;

use super::metrics::{SystemMonitor, SystemStats};

/// Severity a frontend error is logged at.
///
/// Parsed from the string the webview sends so the frontend does not need to
/// know about the tracing levels directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLevel {
    Warn,
    Error,
}

impl ErrorLevel {
    /// Parse a level name, defaulting to [`ErrorLevel::Error`].
    ///
    /// An unknown or missing level is treated as an error: a frontend report
    /// is only sent when something went wrong, so the cautious default is the
    /// louder one.
    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("warn") || value.eq_ignore_ascii_case("warning") {
            ErrorLevel::Warn
        } else {
            ErrorLevel::Error
        }
    }

    /// The `tracing` level this maps to, for the log macro.
    fn as_tracing(&self) -> tracing::Level {
        match self {
            ErrorLevel::Warn => tracing::Level::WARN,
            ErrorLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// A fresh snapshot of this process's CPU and memory.
///
/// The diagnostics panel polls this. Never fails: a process that has vanished
/// from the table still yields a zeroed snapshot rather than an error, so the
/// panel has nothing to special-case.
#[tauri::command]
pub fn get_system_stats(monitor: State<'_, std::sync::Mutex<SystemMonitor>>) -> SystemStats {
    // A poisoned lock means a previous snapshot panicked; recover the guard so
    // the panel keeps working rather than failing forever.
    let mut guard = monitor.lock().unwrap_or_else(|err| err.into_inner());
    guard.snapshot()
}

/// Absolute path of the active log file, for the "reveal log" button.
#[tauri::command]
pub fn get_log_path() -> String {
    super::logging::log_file_path().to_string_lossy().into_owned()
}

/// Record a frontend error in the same log file as the backend.
///
/// The webview calls this from its `window.onerror` and `unhandledrejection`
/// handlers. Kept deliberately tiny and infallible: it runs on the error path,
/// so it must never itself fail the caller. `stack` is optional because not
/// every error carries one.
#[tauri::command]
pub fn log_frontend_error(level: String, message: String, stack: Option<String>) {
    let level = ErrorLevel::parse(&level);
    // `stack` may contain newlines; `%` formats it into the structured field
    // rather than letting it break the log line into several.
    let stack = stack.as_deref().unwrap_or("");

    match level.as_tracing() {
        tracing::Level::WARN => {
            tracing::warn!(target: "frontend", message = %message, stack = %stack)
        }
        tracing::Level::ERROR => {
            tracing::error!(target: "frontend", message = %message, stack = %stack)
        }
        // Every other tracing level is unreachable from `ErrorLevel`, but the
        // match must be total.
        _ => tracing::error!(target: "frontend", message = %message, stack = %stack),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_level_parses_warn_case_insensitively() {
        assert_eq!(ErrorLevel::parse("warn"), ErrorLevel::Warn);
        assert_eq!(ErrorLevel::parse("WARN"), ErrorLevel::Warn);
        assert_eq!(ErrorLevel::parse("Warning"), ErrorLevel::Warn);
    }

    #[test]
    fn error_level_defaults_to_error() {
        assert_eq!(ErrorLevel::parse("error"), ErrorLevel::Error);
        assert_eq!(ErrorLevel::parse(""), ErrorLevel::Error);
        assert_eq!(ErrorLevel::parse("nonsense"), ErrorLevel::Error);
    }

    #[test]
    fn error_level_maps_to_tracing_levels() {
        assert_eq!(ErrorLevel::Warn.as_tracing(), tracing::Level::WARN);
        assert_eq!(ErrorLevel::Error.as_tracing(), tracing::Level::ERROR);
    }
}