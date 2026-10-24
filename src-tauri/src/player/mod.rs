//! Playing a torrent: the in-app stream bridge and an external player.
//!
//! The torrent engine and HTTP bridge already existed under
//! [`crate::torrent`] and were exercised only by an ignored smoke test. This
//! module gives them a lifecycle the app can drive: start lazily, add a
//! torrent, hand the frontend a stream URL, and optionally hand the same URL
//! to a desktop player.

pub mod choice;
pub mod chooser;
pub mod commands;
pub mod external;
pub mod icons;
pub mod launch;

// The torrent session itself stays in `kitsune-core`, because the Qt frontend
// drives it too. Only the external-player pieces above are Tauri's alone.
pub use kitsune_core::player::state;

pub use chooser::{
    choose_player_blocking, list_players, registered_players, PlayerChoice, PlayerOption,
    CHOOSER_UNSUPPORTED, VIDEO_CONTENT_TYPE,
};
pub use launch::{resolve_player, spawn_player, DEFAULT_PLAYER, SUGGESTED_PLAYERS};
pub use state::{PlayerState, TorrentFile, TorrentHandle};
