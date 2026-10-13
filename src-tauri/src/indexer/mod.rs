//! Tauri command glue for finding releases.
//!
//! The search pipeline (parsing, ranking, the Nyaa transport, probing) lives
//! in `kitsune-core`. This module re-exports it under the same names so the
//! command layer's `super::search`, `super::rank`, `super::NyaaIndexer`, ...
//! paths keep resolving, and holds the Tauri command wrappers in `commands`.

pub mod commands;

// The shared pipeline, re-exported under its original names. A glob is used
// rather than a list so adding a module to core::indexer needs no change here.
pub use kitsune_core::indexer::*;

// The Tauri-facing surface (the commands lib.rs registers). `search_all`,
// `IndexerRegistry` and the rest of the pipeline come from the glob above,
// since they now live in `kitsune-core`.
pub use commands::{download_torrent, probe_releases, search_releases};