//! Process CPU and memory metrics.
//!
//! Backs the diagnostics panel and the periodic log line. The mapping from
//! `sysinfo` to [`SystemStats`] is split into a pure [`build_stats`] plus a
//! thin [`SystemMonitor`], so the arithmetic (rounding, defaults) is unit
//! tested without needing a live process.
//!
//! The app is more than one process: Tauri's WebKitGTK backend runs the UI in
//! child processes (`WebKitWebProcess`, `WebKitNetworkProcess`). Measuring only
//! the main process would under-report, so a snapshot sums the whole process
//! tree -- the main process plus every descendant.

use serde::Serialize;
use sysinfo::{get_current_pid, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// How many levels of parent links to walk before giving up.
///
/// Guards against a pathological or cyclic parent chain turning the descendant
/// check into an infinite loop. Real trees are only a few levels deep.
const MAX_ANCESTOR_HOPS: usize = 64;

/// A snapshot of the app's resource usage, plus system context.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    /// CPU usage as a percentage of ONE core, summed over the process tree.
    /// May exceed 100 on a multi-core machine: sysinfo sums across cores, so a
    /// two-thread workload reads around 200 on a two-core box. Divide by
    /// `cpu_cores` for a machine-wide figure comparable to a task manager.
    pub cpu_percent: f32,
    /// `cpu_percent` normalised to 0..100 across all logical cores.
    pub cpu_percent_of_machine: f32,
    /// Logical cores on the machine, for interpreting `cpu_percent`.
    pub cpu_cores: usize,
    /// Resident set size of the process tree: physical memory held, in bytes.
    pub rss_bytes: u64,
    /// Virtual memory size of the process tree, in bytes.
    pub virtual_bytes: u64,
    /// Threads (tasks) in the main process, including the main thread.
    pub thread_count: usize,
    /// Processes in the tree: the main process plus its descendants.
    pub process_count: usize,
    /// Seconds the main process has been running.
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

/// Normalise a per-core CPU figure to a machine-wide 0..100 percentage.
///
/// `cpu_percent` sums across cores, so a fully busy 8-core box reads ~800.
/// Dividing by the core count puts it on the same scale as a task manager's
/// "total CPU" bar. A zero core count (which sysinfo should never report) is
/// treated as 1 so this cannot divide by zero.
pub fn to_machine_percent(cpu_percent: f32, cores: usize) -> f32 {
    if !cpu_percent.is_finite() || cpu_percent <= 0.0 {
        return 0.0;
    }
    let cores = cores.max(1) as f32;
    round_cpu_percent(cpu_percent / cores)
}

/// Assemble a [`SystemStats`] from raw numbers.
///
/// Pure, so the rounding and the defaults are tested without a
/// `sysinfo::System`. A zero thread or process count is normalised to 1: a live
/// process always has at least itself, and reporting 0 would be a lie.
#[allow(clippy::too_many_arguments)]
pub fn build_stats(
    cpu_percent: f32,
    cpu_cores: usize,
    rss_bytes: u64,
    virtual_bytes: u64,
    thread_count: usize,
    process_count: usize,
    uptime_seconds: u64,
    system_total_bytes: u64,
    system_available_bytes: u64,
) -> SystemStats {
    SystemStats {
        cpu_percent: round_cpu_percent(cpu_percent),
        cpu_percent_of_machine: to_machine_percent(cpu_percent, cpu_cores),
        cpu_cores: cpu_cores.max(1),
        rss_bytes,
        virtual_bytes,
        thread_count: thread_count.max(1),
        process_count: process_count.max(1),
        uptime_seconds,
        system_total_bytes,
        system_available_bytes,
    }
}

/// Reads the app's own resource usage, summed over its process tree.
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
        refresh_tree(&mut system);
        system.refresh_memory();

        Self { system, pid }
    }

    /// A fresh snapshot of the app's CPU and memory.
    pub fn snapshot(&mut self) -> SystemStats {
        refresh_tree(&mut self.system);
        self.system.refresh_memory();

        // Sum the whole tree. Borrowing `self.system` immutably twice (the
        // iteration and the descendant walk) is fine: neither mutates.
        let mut cpu = 0.0f32;
        let mut rss = 0u64;
        let mut virt = 0u64;
        let mut processes = 0usize;

        for (pid, process) in self.system.processes() {
            // On Linux, a refresh keeps tasks enabled, and sysinfo lists each
            // thread of every process as its own entry. Such an entry shares
            // its process's address space, so summing its `memory()` would
            // multiply RSS by the thread count (a WebKit process with ~37
            // threads reported ~20 GB that way), and its parent is the process
            // itself, so it would count as a descendant too. Threads belong in
            // `thread_count` (read from `tasks()`), not in this sum.
            if process.thread_kind().is_some() {
                continue;
            }
            if *pid == self.pid || is_descendant(&self.system, *pid, self.pid) {
                cpu += process.cpu_usage();
                rss += process.memory();
                virt += process.virtual_memory();
                processes += 1;
            }
        }

        // Threads are only meaningful for the main process; child WebKit
        // processes report their own task lists separately.
        let threads = self
            .system
            .process(self.pid)
            .and_then(|p| p.tasks())
            .map(|tasks| tasks.len())
            .unwrap_or(1);

        let uptime = self
            .system
            .process(self.pid)
            .map(|p| p.run_time())
            .unwrap_or(0);

        build_stats(
            cpu,
            logical_cores(),
            rss,
            virt,
            threads,
            processes,
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

/// Logical cores on the machine, at least 1.
///
/// `available_parallelism` is the standard-library answer and needs no sysinfo
/// refresh, which keeps the core count stable from the first snapshot.
fn logical_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1)
}

/// True when `pid` is a descendant of `root`.
///
/// Walks the parent links up from `pid`. A missing parent (the process ended
/// between refreshes) or an implausibly long chain both stop the walk and
/// report `false`, so this can neither panic nor loop.
fn is_descendant(system: &System, pid: Pid, root: Pid) -> bool {
    let mut current = system.process(pid).and_then(|p| p.parent());
    let mut hops = 0;

    while let Some(parent) = current {
        if parent == root {
            return true;
        }
        if hops >= MAX_ANCESTOR_HOPS {
            return false;
        }
        hops += 1;
        current = system.process(parent).and_then(|p| p.parent());
    }

    false
}

/// Refresh every process's CPU and memory, dropping dead ones.
///
/// All processes, not just ours: finding our descendants needs the whole
/// table. Only CPU and memory are refreshed -- command line, environment and
/// disk usage are not read here, which keeps the walk cheap enough to run on
/// the panel's poll.
fn refresh_tree(system: &mut System) {
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
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
    fn to_machine_percent_divides_by_cores() {
        // 800% of one core on 8 cores is 100% of the machine.
        assert_eq!(to_machine_percent(800.0, 8), 100.0);
        assert_eq!(to_machine_percent(50.0, 4), 12.5);
    }

    #[test]
    fn to_machine_percent_guards_zero_and_negative() {
        // A zero core count must not divide by zero.
        assert_eq!(to_machine_percent(50.0, 0), 50.0);
        assert_eq!(to_machine_percent(-5.0, 4), 0.0);
        assert_eq!(to_machine_percent(f32::NAN, 4), 0.0);
    }

    #[test]
    fn build_stats_rounds_cpu_and_normalises_counts() {
        let stats = build_stats(50.567, 8, 1024, 2048, 0, 0, 30, 100, 40);
        assert_eq!(stats.cpu_percent, 50.6);
        assert_eq!(stats.cpu_percent_of_machine, 6.3);
        assert_eq!(stats.cpu_cores, 8);
        assert_eq!(stats.rss_bytes, 1024);
        assert_eq!(stats.virtual_bytes, 2048);
        // A live process always has at least its main thread and itself.
        assert_eq!(stats.thread_count, 1);
        assert_eq!(stats.process_count, 1);
        assert_eq!(stats.uptime_seconds, 30);
        assert_eq!(stats.system_total_bytes, 100);
        assert_eq!(stats.system_available_bytes, 40);
    }

    #[test]
    fn build_stats_preserves_real_counts() {
        let stats = build_stats(0.0, 4, 0, 0, 7, 3, 0, 0, 0);
        assert_eq!(stats.thread_count, 7);
        assert_eq!(stats.process_count, 3);
    }

    #[test]
    fn stats_serialise_in_camel_case() {
        let stats = build_stats(1.0, 4, 2, 3, 4, 1, 5, 6, 7);
        let json = serde_json::to_value(&stats).expect("stats should serialise");
        assert!(json.get("cpuPercent").is_some(), "json: {json}");
        assert!(json.get("cpuPercentOfMachine").is_some(), "json: {json}");
        assert!(json.get("cpuCores").is_some(), "json: {json}");
        assert!(json.get("rssBytes").is_some(), "json: {json}");
        assert!(json.get("processCount").is_some(), "json: {json}");
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
        // The test process is a leaf: it has no descendants. If this is more
        // than 1, the descendant walk is matching unrelated processes.
        assert!(
            stats.process_count <= 3,
            "process_count was {} (expected ~1)",
            stats.process_count
        );
        assert!(stats.cpu_cores >= 1);
        assert!(
            stats.cpu_percent >= 0.0,
            "cpu usage should never be negative, got {}",
            stats.cpu_percent
        );
        assert!(stats.cpu_percent_of_machine >= 0.0);
        assert!(stats.system_total_bytes > 0, "machine has memory");
    }

    #[test]
    fn a_process_is_not_its_own_descendant() {
        // The descendant walk starts from the parent, so the root itself is
        // never matched -- the snapshot adds the main process separately.
        let system = System::new();
        assert!(!is_descendant(&system, Pid::from_u32(1), Pid::from_u32(1)));
    }
}