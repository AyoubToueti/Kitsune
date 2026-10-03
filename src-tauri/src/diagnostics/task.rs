//! Background diagnostics tasks.

use std::time::Duration;

use super::metrics::SystemMonitor;

/// How often the resource snapshot is written to the log.
pub const RESOURCE_LOG_INTERVAL: Duration = Duration::from_secs(30);

/// Spawn a task that logs a resource snapshot every
/// [`RESOURCE_LOG_INTERVAL`].
///
/// A separate `SystemMonitor` from the one the panel polls, so the two never
/// contend on a lock: each only tracks this one process, so the cost is small.
///
/// Spawned through `tauri::async_runtime` rather than `tokio::spawn`: `setup`
/// runs on the main thread outside a runtime context, where a bare
/// `tokio::spawn` panics. Tauri's runtime is entered by this call regardless
/// of the caller's context.
pub fn spawn_periodic_log() {
    tauri::async_runtime::spawn(async move {
        let mut monitor = SystemMonitor::new();

        loop {
            // Sleep first, so nothing is logged before the app has started
            // doing anything.
            tokio::time::sleep(RESOURCE_LOG_INTERVAL).await;

            let stats = monitor.snapshot();
            tracing::info!(
                cpu_percent = stats.cpu_percent,
                rss_mb = stats.rss_bytes / (1024 * 1024),
                virtual_mb = stats.virtual_bytes / (1024 * 1024),
                threads = stats.thread_count,
                uptime_s = stats.uptime_seconds,
                "resource usage"
            );
        }
    });
}