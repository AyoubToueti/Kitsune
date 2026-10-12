//! Observability: logging, crash capture and process metrics.
//!
//! Everything here exists so a shipped build can be diagnosed after the fact.
//! A GUI release has no console, so a log file and captured crashes are the
//! only evidence a user or maintainer can inspect.

pub mod commands;
pub mod task;

// The pure modules moved to `kitsune-core`. Re-export them under the SAME
// names so `super::metrics`, `super::speedtest` and `super::logging` in
// commands.rs/task.rs keep resolving, and so `diagnostics::metrics::...`
// paths elsewhere in this crate are unchanged.
pub use kitsune_core::diagnostics::{logging, metrics, panic, speedtest};

pub use metrics::{SystemMonitor, SystemStats};