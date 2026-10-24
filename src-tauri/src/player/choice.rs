//! The player-choice data types shared by the platform dialogs.
//!
//! The dialogs themselves are frontend/platform concerns (the Tauri build uses
//! GTK on Linux and the registry on Windows; the Qt6 build will use its own),
//! but the SHAPE of what a "chosen player" is travels across all of them, so it
//! lives here.

use serde::Serialize;

/// The MIME type a chooser lists applications for.
///
/// Anime releases are overwhelmingly Matroska, and every desktop video player
/// registers `video/x-matroska`.
pub const VIDEO_CONTENT_TYPE: &str = "video/x-matroska";

/// A chosen player, resolved from a desktop application's command line.
///
/// `program` is the executable and `extra_args` are its own arguments with the
/// URL placeholder removed. Together they fit `spawn_player` directly: the
/// program is spawned, the extra args come first, and the stream URL is
/// appended last as a positional argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerChoice {
    /// The executable to spawn.
    pub program: String,
    /// The application's own arguments, in order, minus the URL placeholder.
    pub extra_args: Vec<String>,
}

/// A player offered in the in-app picker, before anything is launched.
///
/// Carries enough to render a row AND to launch it later, so the picker does
/// not have to re-resolve anything. `id` is the executable path, which is what
/// de-duplicates a player found both by a platform registry and by the PATH
/// scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerOption {
    /// Stable identity: the resolved executable path.
    pub id: String,
    /// Human name for the row, e.g. "VLC media player" or "mpv".
    pub name: String,
    /// The program to spawn.
    pub program: String,
    /// The program's own arguments, in order, minus any URL placeholder.
    pub extra_args: Vec<String>,
    /// True for the reader's stored preference, so the picker can mark it.
    pub is_default: bool,
    /// The player's icon as a `data:` URL, or `None` when none was found.
    pub icon: Option<String>,
}