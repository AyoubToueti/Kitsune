//! Tauri commands for playing a torrent.
//!
//! Thin wrappers, like [`crate::commands`]: the logic lives on
//! [`PlayerState`] so it can be exercised without a Tauri app.

use std::path::PathBuf;

use tauri::State;

use super::launch::SUGGESTED_PLAYERS;
use super::state::{PlayerState, TorrentHandle};

/// Add a `.torrent` file, returning its id and the files inside it.
#[tauri::command]
pub async fn add_torrent(
    state: State<'_, PlayerState>,
    path: String,
) -> Result<TorrentHandle, String> {
    state
        .add_torrent(&PathBuf::from(path))
        .await
        .map_err(|err| err.to_string())
}

/// The loopback URL that streams one file of one torrent.
#[tauri::command]
pub async fn get_stream_url(
    state: State<'_, PlayerState>,
    torrent_id: usize,
    file_idx: usize,
) -> Result<String, String> {
    state
        .stream_url(torrent_id, file_idx)
        .await
        .map_err(|err| err.to_string())
}

/// Open a URL in an external player, returning the player that was launched.
#[tauri::command]
pub fn open_in_player(
    state: State<'_, PlayerState>,
    url: String,
    player: Option<String>,
) -> Result<String, String> {
    state
        .open_in_player(&url, player.as_deref())
        .map_err(|err| err.to_string())
}

/// The chosen external player.
#[tauri::command]
pub fn get_player(state: State<'_, PlayerState>) -> String {
    state.player()
}

/// Choose the external player. A blank name restores the default.
#[tauri::command]
pub fn set_player(state: State<'_, PlayerState>, name: String) {
    state.set_player(&name);
}

/// Players the UI offers, in preference order.
#[tauri::command]
pub fn suggested_players() -> Vec<String> {
    SUGGESTED_PLAYERS
        .iter()
        .map(|name| name.to_string())
        .collect()
}
