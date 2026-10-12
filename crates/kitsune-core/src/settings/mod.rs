//! The reader's preferences: which external player to launch and how, where
//! downloads go, how releases are ranked, and the appearance.
//!
//! One JSON file, read and written by Rust. Every value has a default, so a
//! fresh install and a corrupt file both behave identically -- the app runs on
//! its built-in choices until the reader changes one.
//!
//! Only the store lives here. The Tauri command wrappers (which emit an event
//! and read `State`) stay in the frontend.

pub mod store;

pub use store::{Settings, SettingsStore};