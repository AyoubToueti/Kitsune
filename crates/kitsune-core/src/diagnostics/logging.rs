//! Process-wide logging.
//!
//! Installs a `tracing` subscriber that writes to BOTH stdout (for
//! `pnpm tauri dev`) and a rotating file under the OS data directory. Before
//! this existed the crate called `tracing::info!` and friends but never
//! installed a subscriber, so every log line was silently dropped -- and a
//! release build has no console to fall back on.
//!
//! The file lives at `<data_dir>/kitsune/logs/kitsune.log`, the same
//! `kitsune` directory as the token and settings stores, so everything the
//! app persists sits together. Rotation is daily and five files are kept.

use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

/// Directory the rotating log files are written to.
///
/// Mirrors the token and settings stores: the OS data directory with a
/// `kitsune` subdirectory. Falls back to the temp directory when the OS
/// reports no data directory, matching `TokenStore`, so logging never stops
/// the app from starting.
pub fn log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("kitsune")
        .join("logs")
}

/// Path of the active log file, for the "reveal log" button.
pub fn log_file_path() -> PathBuf {
    log_dir().join("kitsune.log")
}

/// Install the global subscriber.
///
/// Returns the non-blocking writer's guard: it MUST be kept alive for the
/// life of the process. Dropping it stops the logging thread and truncates
/// any buffered lines.
///
/// A second call (for example across tests) is a no-op rather than a panic --
/// `try_init` reports the "already set" case as an error we deliberately
/// ignore.
pub fn init_logging() -> WorkerGuard {
    let dir = log_dir();
    // Best effort: if the directory cannot be created the file layer simply
    // writes nowhere, and stdout logging still works.
    let _ = std::fs::create_dir_all(&dir);

    let file_appender = tracing_appender::rolling::daily(&dir, "kitsune.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // RUST_LOG wins; otherwise info and above. `EnvFilter` is why the
    // dependency enables the `env-filter` feature.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stdout_layer = fmt::layer().with_target(false);

    // No ANSI escapes in the file: they would show up as literal garbage in a
    // text editor or a pasted bug report.
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_writer(non_blocking);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .try_init();

    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_dir_ends_with_kitsune_logs() {
        let dir = log_dir();
        assert!(
            dir.ends_with("kitsune/logs"),
            "unexpected log dir: {dir:?}"
        );
    }

    #[test]
    fn log_file_path_is_inside_the_log_dir() {
        let path = log_file_path();
        assert_eq!(path.file_name().unwrap(), "kitsune.log");
        assert_eq!(path.parent().unwrap(), log_dir());
    }
}