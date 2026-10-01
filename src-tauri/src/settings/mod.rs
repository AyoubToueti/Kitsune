//! The reader's preferences: which external player to launch and how, where
//! downloads go, how releases are ranked, and the appearance.
//!
//! One JSON file, read and written by Rust. Every value has a default, so a
//! fresh install and a corrupt file both behave identically -- the app runs on
//! its built-in choices until the reader changes one.

pub mod store;

pub use store::{Settings, SettingsStore};

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

/// The shared settings store, as Tauri manages it.
///
/// An `Arc` so the player state can hold the same instance -- a second store
/// would read a different file and the two would disagree about the reader's
/// choices.
pub type SharedSettings = Arc<SettingsStore>;

/// Event emitted after the settings are written.
///
/// Carries the new settings, so a listener that only needs the fresh values can
/// use them directly instead of re-reading. No secret lives here -- settings are
/// the reader's own configuration -- so shipping the whole object is fine.
pub const SETTINGS_CHANGED_EVENT: &str = "settings-changed";

/// The current settings, or the defaults when nothing has been saved.
#[tauri::command]
pub fn get_settings(store: State<'_, SharedSettings>) -> Settings {
    store.read()
}

/// Replace the stored settings.
///
/// Returns the settings actually persisted, so a caller can see the effect of
/// any clamping (the auto-launch threshold) without a second round trip. Also
/// emits [`SETTINGS_CHANGED_EVENT`], so any other surface showing a setting
/// (the watch page's player picker, say) updates without a reload.
#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    store: State<'_, SharedSettings>,
    settings: Settings,
) -> Result<Settings, String> {
    store.write(&settings).map_err(|err| err.to_string())?;
    let stored = store.read();
    // A failed emit is not worth failing the save over: the write already
    // happened, and the only cost is a stale display elsewhere.
    let _ = app.emit(SETTINGS_CHANGED_EVENT, &stored);
    Ok(stored)
}