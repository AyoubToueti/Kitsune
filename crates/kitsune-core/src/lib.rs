//! Kitsune's shared, framework-agnostic backend logic.
//!
//! Everything here is pure enough to be used by any frontend: the Tauri app
//! today, the Qt6 frontend after the migration. Modules are moved in one at a
//! time from `src-tauri/src`, and `src-tauri` re-exports each one with a
//! `pub use kitsune_core::X;` shim so its existing `crate::X::...` paths keep
//! resolving with ZERO call-site edits.
//!
//! HARD RULE: this crate must not depend on a UI framework (tauri, slint, Qt,
//! ...). A frontend dependency here would pull one UI stack into the other.

pub mod auth;
pub mod diagnostics;
pub mod indexer;
pub mod player;
pub mod providers;
pub mod settings;
pub mod torrent;
pub mod types;