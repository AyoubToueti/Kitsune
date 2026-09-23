//! Tauri commands for reading and writing the reader's AniList list.
//!
//! Lives under `auth` rather than `commands` because every one of these needs a
//! token: they are account operations, not catalogue reads, and grouping them
//! with the sign-in flow keeps the two halves of "the reader's account" in one
//! place.
//!
//! The provider handle is concrete (`Arc<AniListProvider>`), not the
//! `Arc<dyn AnimeProvider>` the metadata commands use, because list writes are
//! not on that trait. See the note on `AniListProvider::save_list_entry`.

use tauri::State;

use super::SharedAniList;
use crate::types::{Anime, ListStatus};

/// A work's position on the reader's list, or `null` when it is not on it.
///
/// Modelled as an `Option` rather than a status with an "unset" variant: "not
/// on the list" is a different thing from any status the reader can choose, and
/// giving it a variant would put a value in the menu that is not selectable.
#[tauri::command]
pub async fn get_list_entry(
    provider: State<'_, SharedAniList>,
    media_id: i64,
) -> Result<Option<crate::providers::anilist::ListEntry>, String> {
    provider
        .list_entry(media_id)
        .await
        .map_err(|err| err.to_string())
}

/// Put a work on the reader's list, or move it between lists.
///
/// `progress` is omitted when the caller only wants to change the status, so an
/// existing progress value is left alone rather than reset to zero.
#[tauri::command]
pub async fn set_list_entry(
    provider: State<'_, SharedAniList>,
    media_id: i64,
    status: ListStatus,
    progress: Option<u32>,
) -> Result<(), String> {
    provider
        .save_list_entry(media_id, status, progress)
        .await
        .map_err(|err| err.to_string())
}

/// The works the reader is currently watching, most recently touched first.
///
/// Returns an empty list when signed out rather than an error: the home page
/// renders this unconditionally, and "nothing to show" is the correct state
/// for a reader with no account.
#[tauri::command]
pub async fn continue_watching(
    provider: State<'_, SharedAniList>,
    limit: u32,
) -> Result<Vec<Anime>, String> {
    // No token means nothing to read. Short-circuited here rather than letting
    // AniList reject the query, so a signed-out home page makes no request at
    // all.
    if !provider.has_token() {
        return Ok(Vec::new());
    }

    provider
        .continue_watching(limit)
        .await
        .map_err(|err| err.to_string())
}