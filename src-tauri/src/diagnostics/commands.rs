//! Tauri commands exposing diagnostics to the frontend.

use tauri::State;

use super::metrics::{SystemMonitor, SystemStats};

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