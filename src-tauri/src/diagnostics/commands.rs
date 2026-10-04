//! Tauri commands exposing diagnostics to the frontend.

use std::sync::atomic::{AtomicBool, Ordering};

use reqwest::Client;
use tauri::{AppHandle, Emitter, State};

use super::metrics::{SystemMonitor, SystemStats};
use super::speedtest::{self, SpeedTestConfig, SpeedTestResult};

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

/// Shared state for the speed test: one pooled client, and a flag that keeps
/// two tests from running at once.
pub struct SpeedTestState {
    client: Client,
    /// Set while a test runs. A second caller gets an error instead of
    /// competing for bandwidth with the first (which would corrupt both
    /// numbers).
    running: AtomicBool,
}

impl SpeedTestState {
    /// Build the state, failing only if the HTTP client cannot be built.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: speedtest::build_client()?,
            running: AtomicBool::new(false),
        })
    }
}

/// Measure latency and download speed against Cloudflare.
///
/// Emits [`speedtest::PROGRESS_EVENT`] as it runs, then resolves with the final
/// result. Rejects a second call while one is in flight.
#[tauri::command]
pub async fn run_speed_test(
    app: AppHandle,
    state: State<'_, SpeedTestState>,
) -> Result<SpeedTestResult, String> {
    // Claim the guard; if it was already taken, another test is running.
    if state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("a speed test is already running".to_string());
    }

    // Release the guard however this returns.
    let _guard = RunningGuard(&state.running);

    let config = SpeedTestConfig::default();
    let emit_app = app.clone();
    speedtest::run_measurement(&state.client, speedtest::DOWN_URL, &config, move |progress| {
        // A failed emit is not worth failing the measurement over: the final
        // result is returned to the caller anyway.
        let _ = emit_app.emit(speedtest::PROGRESS_EVENT, progress);
    })
    .await
    .map_err(|err| err.to_string())
}

/// Clears the running flag on drop, so an early return (an error) still
/// releases it.
struct RunningGuard<'a>(&'a AtomicBool);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
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