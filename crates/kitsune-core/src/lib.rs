//! Kitsune's shared, framework-agnostic backend logic.
//!
//! This crate is where the whole application lives: metadata providers, the
//! indexer, the torrent engine and stream bridge, downloads, the local library,
//! settings, auth and player state. `crates/kitsune-qt` is a thin QML frontend
//! on top -- it owns windows, widgets and the libmpv surface, and calls in here
//! for everything else.
//!
//! HARD RULE: this crate must not depend on a UI framework (Qt, slint, gtk,
//! tauri, ...). That is what keeps the logic testable without a display and
//! keeps the frontend replaceable.

pub mod auth;
pub mod diagnostics;
pub mod downloads;
pub mod episodes;
pub mod indexer;
pub mod list;
pub mod player;
pub mod providers;
pub mod settings;
pub mod torrent;
pub mod torrent_files;
pub mod types;