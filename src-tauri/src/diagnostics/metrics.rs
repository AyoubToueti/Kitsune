//! Process CPU and memory metrics.
//!
//! Backs the diagnostics panel and the periodic log line. The mapping from
//! `sysinfo` to [`SystemStats`] is split into a pure [`build_stats`] plus a
//! thin [`SystemMonitor`], so the arithmetic (rounding, defaults) is unit
//! tested without needing a live process.

use serde::Serialize;
use sysinfo::{get_current_pid, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// A snapshot of this process's resource usage, plus system memory for context.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    /// CPU usage of this process as a percentage. May exceed 100 on a
    /// multi-core machine: sysinfo sums across cores, so a busy two-thread
    /// workload reads around 200 on a two-core box.
    pub cpu_percent: f32,
    /// Resident set size: physical memory currently held, in bytes.
    pub rss_bytes: u64,
    /// Virtual memory size in bytes.
    pub virtual_bytes: u64,
    /// Threads (tasks) in this process, including the main thread.
    pub thread_count: usize,
    /// Seconds this process has been running.
    pub uptime_seconds: u64,
    /// Total physical memory on the machine, in bytes.
    pub system_total_bytes: u64,
    /// Memory currently available on the machine, in bytes.
    pub system_available_bytes: u64,
}

/// Round a raw CPU percentage to one decimal.
///
/// sysinfo returns `f32`; the panel shows one decimal and the log line needs
/// no more. A non-finite value is reported as `0.0` rather than propagated.
pub fn round_cpu_percent(raw: f32) -> f32 {
    if !raw.is_finite() {
        return 0.0;
    }
    (raw * 10.0).round() / 10.0
}

/// Assemble a [`SystemStats`] from raw numbers.
///
/// Pure, so the rounding and the defaults are tested without a
/// `sysinfo::System`. A zero thread count is normalised to 1: a live process
/// always has at least its main thread, and reporting 0 would be a lie.
pub fn build_stats(
    cpu_percent: f32,
    rss_bytes: u64,
    virtual_bytes: u64,
    thread_count: usize,
    uptime_seconds: u64,
    system_total_bytes: u64,
    system_available_bytes: u64,
) -> SystemStats {
    SystemStats {
        cpu_percent: round_cpu_percent(cpu_percent),
        rss_bytes,
        virtual_bytes,
        thread_count: thread_count.max(1),
        uptime_seconds,
        system_total_bytes,
        system_available_bytes,
    }
}

/// Reads this process's own resource usage.
///
/// Holds a `System` so the CPU delta between polls can be computed: sysinfo
/// needs two refreshes separated in time before `cpu_usage` is meaningful, so
/// the first snapshot after construction reads `0.0%`.
pub struct SystemMonitor {
    system: System,
    pid: Pid,
}

impl SystemMonitor {
    /// A monitor for the current process.
    pub fn new() -> Self {
        let mut system = System::new();
        let pid = current_pid();

        // Prime the CPU counter: without one refresh first, the first
        // snapshot would report 0%.
        refresh_process(&mut system, pid);
        system.refresh_memory();

        Self { system, pid }
    }

    /// A fresh snapshot of this process's CPU and memory.
    pub fn snapshot(&mut self) -> SystemStats {
        refresh_process(&mut self.system, self.pid);
        self.system.refresh_memory();

        let process = self.system.process(self.pid);

        let cpu = process.map(|p| p.cpu_usage()).unwrap_or(0.0);
        let rss = process.map(|p| p.memory()).unwrap_or(0);
        let virt = process.map(|p| p.virtual_memory()).unwrap_or(0);
        let threads = process
            .and_then(|p| p.tasks())
            .map(|tasks| tasks.len())
            .unwrap_or(1);
        let uptime = process.map(|p| p.run_time()).unwrap_or(0);

        build_stats(
            cpu,
            rss,
            virt,
            threads,
            uptime,
            self.system.total_memory(),
            self.system.available_memory(),
        )
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// The current process's pid, falling back to the OS value.
///
/// `get_current_pid` only fails on platforms sysinfo cannot identify the pid
/// on, where `std::process::id` still gives the right number.
fn current_pid() -> Pid {
    get_current_pid().unwrap_or_else(|_| Pid::from_u32(std::process::id()))
}

/// Refresh only this process, with every field populated.
///
/// Targeted rather than `refresh_all`: polling every 2s for the panel would
/// otherwise walk the whole process table for a single number.
fn refresh_process(system: &mut System, pid: Pid) {
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::everything(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_cpu_percent_keeps_one_decimal() {
        assert_eq!(round_cpu_percent(12.3456), 12.3);
        assert_eq!(round_cpu_percent(0.04), 0.0);
        assert_eq!(round_cpu_percent(199.99), 200.0);
    }

    #[test]
    fn round_cpu_percent_treats_non_finite_as_zero() {
        assert_eq!(round_cpu_percent(f32::NAN), 0.0);
        assert_eq!(round_cpu_percent(f32::INFINITY), 0.0);
        assert_eq!(round_cpu_percent(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn build_stats_rounds_cpu_and_normalises_zero_threads() {
        let stats = build_stats(50.567, 1024, 2048, 0, 30, 100, 40);
        assert_eq!(stats.cpu_percent, 50.6);
        assert_eq!(stats.rss_bytes, 1024);
        assert_eq!(stats.virtual_bytes, 2048);
        // A live process always has at least its main thread.
        assert_eq!(stats.thread_count, 1);
        assert_eq!(stats.uptime_seconds, 30);
        assert_eq!(stats.system_total_bytes, 100);
        assert_eq!(stats.system_available_bytes, 40);
    }

    #[test]
    fn build_stats_preserves_a_real_thread_count() {
        let stats = build_stats(0.0, 0, 0, 7, 0, 0, 0);
        assert_eq!(stats.thread_count, 7);
    }

    #[test]
    fn stats_serialise_in_camel_case() {
        let stats = build_stats(1.0, 2, 3, 4, 5, 6, 7);
        let json = serde_json::to_value(&stats).expect("stats should serialise");
        assert!(json.get("cpuPercent").is_some(), "json: {json}");
        assert!(json.get("rssBytes").is_some(), "json: {json}");
        assert!(json.get("systemAvailableBytes").is_some(), "json: {json}");
    }

    /// A live snapshot of the test process must be self-consistent. CPU can
    /// read 0 on the first sample, so only the always-true invariants are
    /// asserted here.
    #[test]
    fn snapshot_of_the_current_process_is_sane() {
        let mut monitor = SystemMonitor::new();
        let stats = monitor.snapshot();

        assert!(stats.rss_bytes > 0, "test process should hold memory");
        assert!(stats.thread_count >= 1);
        assert!(
            stats.cpu_percent >= 0.0,
            "cpu usage should never be negative, got {}",
            stats.cpu_percent
        );
        assert!(stats.system_total_bytes > 0, "machine has memory");
    }
}