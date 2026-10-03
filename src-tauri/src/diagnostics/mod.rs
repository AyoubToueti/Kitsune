//! Observability: logging, crash capture and process metrics.
//!
//! Everything here exists so a shipped build can be diagnosed after the fact.
//! A GUI release has no console, so a log file and captured crashes are the
//! only evidence a user or maintainer can inspect.

pub mod commands;
pub mod logging;
pub mod metrics;
pub mod task;

pub use metrics::{SystemMonitor, SystemStats};