//! Tauri commands for playing a torrent.
//!
//! Thin wrappers, like [`crate::commands`]: the logic lives on
//! [`PlayerState`] so it can be exercised without a Tauri app.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::launch::SUGGESTED_PLAYERS;
use super::state::{PlayerState, TorrentHandle};
use crate::torrent::TorrentProgress;

/// Event emitted when an external player the app launched exits.
///
/// The payload is the torrent id, so the frontend can tell whether the exit
/// belongs to the episode it is showing. Kept in sync with
/// [`crate::player`]'s frontend wrapper (`PLAYER_EXIT_EVENT` in
/// `src/lib/api/player.ts`).
pub const PLAYER_EXIT_EVENT: &str = "player-exit";

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

/// Add a magnet URI, returning its id and the files inside it.
///
/// The counterpart of [`add_torrent`] for releases the app found itself: a
/// search result carries a magnet, not a `.torrent` path.
#[tauri::command]
pub async fn add_magnet(
    state: State<'_, PlayerState>,
    magnet_uri: String,
) -> Result<TorrentHandle, String> {
    state
        .add_magnet(&magnet_uri)
        .await
        .map_err(|err| err.to_string())
}

/// Drop a torrent from the session, deleting the pieces it cached.
///
/// The counterpart of [`add_magnet`] and [`add_torrent`]: the watch page calls
/// this on the way out, so a closed episode stops consuming bandwidth.
#[tauri::command]
pub async fn remove_torrent(
    state: State<'_, PlayerState>,
    torrent_id: usize,
) -> Result<(), String> {
    state
        .remove_torrent(torrent_id)
        .await
        .map_err(|err| err.to_string())
}

/// Pause a torrent's transfer, keeping its partial data.
///
/// The buffering panel's pause button: the reader stops the download without
/// giving up what has already been fetched, so resuming continues from there.
#[tauri::command]
pub async fn pause_torrent(
    state: State<'_, PlayerState>,
    torrent_id: usize,
) -> Result<(), String> {
    state
        .pause_torrent(torrent_id)
        .await
        .map_err(|err| err.to_string())
}

/// Resume a paused torrent's transfer.
///
/// The counterpart of [`pause_torrent`].
#[tauri::command]
pub async fn resume_torrent(
    state: State<'_, PlayerState>,
    torrent_id: usize,
) -> Result<(), String> {
    state
        .resume_torrent(torrent_id)
        .await
        .map_err(|err| err.to_string())
}

/// The loopback URL that streams one file of one torrent.
/// Restrict a torrent's download to a single file index.
///
/// Called when a file is picked: librqbit otherwise downloads the torrent's
/// first files in order, so a season pack would fetch episode 1 while the
/// reader waits on episode 21 with no visible progress.
#[tauri::command]
pub async fn set_only_files(
    state: State<'_, PlayerState>,
    torrent_id: usize,
    only_files: Vec<usize>,
) -> Result<(), String> {
    state
        .set_only_files(torrent_id, only_files)
        .await
        .map_err(|err| err.to_string())
}

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

/// A download-progress snapshot for a torrent, for the watch page's status panel.
///
/// `None` means there is no session or no such torrent yet, which the panel
/// renders as "not started" rather than as an error.
#[tauri::command]
pub async fn get_torrent_stats(
    state: State<'_, PlayerState>,
    torrent_id: usize,
) -> Result<Option<TorrentProgress>, String> {
    Ok(state.torrent_progress(torrent_id))
}

/// Open a URL in an external player, returning the player that was launched.
///
/// `torrentId` is the torrent the URL streams from. While the player runs, a
/// removal of that torrent is deferred, so leaving the watch page does not cut
/// off a player that is still reading it. Omitted for a URL with no torrent
/// behind it.
///
/// When the player exits, a [`PLAYER_EXIT_EVENT`] is emitted with the torrent
/// id so the frontend can stop waiting and release the torrent -- otherwise a
/// page left open would keep downloading behind a "playing" panel.
#[tauri::command]
pub async fn open_in_player(
    app: AppHandle,
    state: State<'_, PlayerState>,
    url: String,
    player: Option<String>,
    torrent_id: Option<usize>,
) -> Result<String, String> {
    // The emitter is moved into the background wait task, so it must be `'static`
    // and cheap to clone. `AppHandle` is both.
    let on_exit: Option<Arc<dyn Fn(usize) + Send + Sync>> =
        torrent_id.map(|_| {
            let app = app.clone();
            Arc::new(move |id: usize| {
                if let Err(err) = app.emit(PLAYER_EXIT_EVENT, id) {
                    tracing::warn!("failed to emit {PLAYER_EXIT_EVENT} for torrent {id}: {err:#}");
                }
            }) as Arc<dyn Fn(usize) + Send + Sync>
        });

    state
        .open_in_player(&url, player.as_deref(), torrent_id, on_exit)
        .await
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
