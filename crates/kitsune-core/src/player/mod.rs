//! Playing a torrent: the in-app stream bridge and an external player.
//!
//! The torrent engine and HTTP bridge live under [`crate::torrent`]. This
//! module gives them a lifecycle any frontend can drive: start lazily, add a
//! torrent, hand the frontend a stream URL, and optionally hand the same URL
//! to a desktop player.
//!
//! Frontend-specific pieces stay in the frontend: the Tauri app keeps
//! `chooser` (a GTK/winreg dialog) and `commands`; the Qt6 app will supply its
//! own dialog. Only the pure launch/icon logic lives here.

pub mod choice;
pub mod icons;
pub mod last_release;
pub mod launch;
pub mod state;
pub mod watch_position;

pub use choice::{PlayerChoice, PlayerOption, VIDEO_CONTENT_TYPE};
pub use last_release::{LastRelease, LastReleaseStore};
pub use launch::{DEFAULT_PLAYER, SUGGESTED_PLAYERS};
pub use state::{PlayerState, TorrentFile, TorrentHandle};
pub use watch_position::{WatchPosition, WatchPositionStore};