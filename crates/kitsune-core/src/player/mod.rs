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

pub mod last_release;
pub mod state;
pub mod watch_position;

pub use last_release::{LastRelease, LastReleaseStore};
pub use state::{PlayerState, TorrentFile, TorrentHandle};
pub use watch_position::{WatchPosition, WatchPositionStore};

/// The external player assumed when the reader has not chosen one.
///
/// Kept here, next to the settings that store it, rather than with the
/// process-spawning code: `Settings::default` needs it, and settings are a
/// frontend-agnostic concern.
pub const DEFAULT_PLAYER: &str = "mpv";

/// Resolve a stored player preference to the program to run.
///
/// A blank or missing preference falls back to [`DEFAULT_PLAYER`]. The name is
/// deliberately not validated against any list of known players: refusing to
/// launch a player the reader typed would be worse than letting the spawn fail
/// with a message that names it.
pub fn resolve_player(preference: Option<&str>) -> String {
    match preference.map(str::trim) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => DEFAULT_PLAYER.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_player_is_mpv() {
        assert_eq!(DEFAULT_PLAYER, "mpv");
    }

    #[test]
    fn resolve_player_falls_back_when_absent() {
        assert_eq!(resolve_player(None), DEFAULT_PLAYER);
    }

    #[test]
    fn resolve_player_falls_back_when_blank() {
        // A stored empty string must not become "launch nothing".
        assert_eq!(resolve_player(Some("")), DEFAULT_PLAYER);
        assert_eq!(resolve_player(Some("   ")), DEFAULT_PLAYER);
    }

    #[test]
    fn resolve_player_keeps_a_chosen_name() {
        assert_eq!(resolve_player(Some("vlc")), "vlc");
    }

    #[test]
    fn resolve_player_trims_whitespace() {
        assert_eq!(resolve_player(Some("  vlc  ")), "vlc");
    }

    #[test]
    fn resolve_player_accepts_a_name_not_in_any_suggestion_list() {
        assert_eq!(resolve_player(Some("my-player")), "my-player");
    }
}