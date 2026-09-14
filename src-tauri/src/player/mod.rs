//! Playing a torrent: the in-app stream bridge and an external player.
//!
//! The torrent engine and HTTP bridge already existed under
//! [`crate::torrent`] and were exercised only by an ignored smoke test. This
//! module gives them a lifecycle the app can drive: start lazily, add a
//! torrent, hand the frontend a stream URL, and optionally hand the same URL
//! to a desktop player.

pub mod commands;
pub mod launch;
pub mod state;

pub use launch::{resolve_player, spawn_player, DEFAULT_PLAYER, SUGGESTED_PLAYERS};
pub use state::{PlayerState, TorrentFile, TorrentHandle};
