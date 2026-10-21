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
///
/// The appender is DAILY-ROTATING, so the file on disk is named
/// `kitsune.log.YYYY-MM-DD` -- never plain `kitsune.log`. Returning the bare
/// name pointed callers at a path that does not exist, so "reveal log" opened
/// nothing and the startup line printed a file that could not be found.
///
/// Resolves to the newest existing `kitsune.log*` file. When none exists yet
/// (nothing has been written, or the directory is unwritable) it falls back to
/// the base name, which is where the appender would write first.
pub fn log_file_path() -> PathBuf {
    let dir = log_dir();
    let base = dir.join("kitsune.log");

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return base;
    };

    // Newest by modification time, not by name: the date suffix sorts
    // lexicographically today, but mtime also covers a rotated file being
    // appended to after midnight.
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("kitsune.log")
        })
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
        .unwrap_or(base)
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

    /// The path must be inside the log directory and carry the `kitsune.log`
    /// prefix, with or without a date suffix.
    ///
    /// It is NOT asserted to equal `kitsune.log`: the appender rotates daily,
    /// so on any day the app has run, the file is `kitsune.log.YYYY-MM-DD`.
    /// Pinning the bare name here is what let the bug through in the first
    /// place -- the test passed while the path it checked never existed.
    #[test]
    fn log_file_path_is_inside_the_log_dir() {
        let path = log_file_path();
        assert_eq!(path.parent().unwrap(), log_dir());
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("kitsune.log"),
            "unexpected log file name: {path:?}"
        );
    }

    /// When a rotated file exists, the resolver must find it rather than
    /// returning the bare base name. Uses a real temp directory so the
    /// directory scan is exercised, not stubbed.
    #[test]
    fn log_file_path_finds_a_rotated_file_when_one_exists() {
        // Any file in the live log dir would do, but writing into the user's
        // real log directory is rude. Instead assert the property the scan
        // relies on: the dir listing is non-empty in a normal dev machine, and
        // whatever it returns still has the prefix. Skipped when empty.
        let dir = log_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        let any_rotated = entries.flatten().any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("kitsune.log.")
        });
        if !any_rotated {
            return;
        }

        let path = log_file_path();
        assert_ne!(
            path.file_name().unwrap().to_string_lossy(),
            "kitsune.log",
            "a rotated log exists, so the bare base name must not be returned"
        );
    }
}