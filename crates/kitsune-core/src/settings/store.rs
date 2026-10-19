//! Where the reader's preferences are kept between runs.
//!
//! A plain JSON file in the OS data directory, alongside the token and
//! last-played stores and written the same way (temp file + atomic rename).
//! Not a credential, so it is not permission-restricted.
//!
//! Every field is optional on disk and has a default, so a settings file
//! written by an older build still parses and a missing file simply means
//! "everything at its default".

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::types::Resolution;

/// The whole of the reader's preferences.
///
/// `rename_all = "camelCase"` so the wire shape the frontend sees and the file
/// on disk agree, and so no separate DTO is needed. `default` on the struct
/// means every missing field falls back to [`Settings::default`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// The external player binary, e.g. `mpv` or `flatpak`.
    pub player: String,
    /// Extra arguments passed before the stream URL, e.g.
    /// `["run", "io.mpv.Mpv"]` for a flatpak wrapper. Never shell-parsed; each
    /// element is one argv entry.
    pub player_args: Vec<String>,
    /// Where downloaded pieces are written, or `None` for the default temp
    /// directory. An explicit path wins over `KITSUNE_DOWNLOAD_DIR`.
    pub download_dir: Option<String>,
    /// Resolutions to prefer, in order. Mirrors `ReleasePreference`.
    pub preferred_resolutions: Vec<Resolution>,
    /// Releases below this many seeders are dropped when better ones exist.
    pub min_seeders: u32,
    /// The fraction of the chosen file that must be present before the player
    /// is launched. Clamped to a sane range on read.
    pub ready_fraction: f64,
    /// Whether to ask which player to use before launching.
    ///
    /// `true` (the default) shows the picker at the threshold. `false` launches
    /// the stored [`Settings::player`] straight away, which is what the
    /// picker's "Always" button sets. The reader can still open the picker by
    /// hand, so this only controls the automatic prompt.
    pub ask_every_time: bool,
    /// `"light"`, `"dark"` or `"system"`.
    pub theme: String,
    /// Player keyboard shortcuts, keyed by action name (e.g. `playPause`).
    ///
    /// A map rather than fixed fields so adding an action needs no schema
    /// change, and so an unknown key written by a newer build is simply
    /// ignored. An absent action falls back to the player's built-in default.
    /// `BTreeMap` so the JSON order is stable and the settings UI is not
    /// reshuffled between saves.
    pub shortcuts: BTreeMap<String, String>,
    /// What to do when a saved watch position exists for an episode being
    /// played: `"ask"` (prompt), `"seek"` (jump automatically), or `"fresh"`
    /// (always start at 0).
    pub resume_behavior: String,
    /// How many seconds to rewind from the saved position when resuming, so
    /// the reader gets a run-up into the scene instead of landing mid-line.
    /// A whole number; the UI accepts 0 and large values (e.g. 75).
    pub resume_rewind_seconds: u32,
    /// Whether the speed test reports download speed in binary units
    /// (MiB/s) rather than decimal megabits (Mbps).
    ///
    /// Both are correct; they answer different questions. Mbps is how
    /// connections are sold, MiB/s is the unit the app already uses for
    /// torrent sizes and download rates, so a reader comparing a speed result
    /// against a file size can pick the matching one. Defaults to `false`,
    /// which keeps the familiar "42 Mbps" reading.
    pub use_binary_units: bool,
    /// Which title form the UI shows: `"english"` or `"romaji"`.
    ///
    /// A `String` rather than an enum, matching `theme` and `resume_behavior`:
    /// the vocabulary belongs to the frontend, and an unrecognised value must
    /// degrade to a usable default rather than fail the whole file's decode.
    /// Read through [`Settings::title_language`], which validates it.
    pub title_language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            player: crate::player::DEFAULT_PLAYER.to_string(),
            player_args: Vec::new(),
            download_dir: None,
            // Matches `ReleasePreference::default`: 1080p then 720p, no floor.
            preferred_resolutions: vec![Resolution::R1080p, Resolution::R720p],
            min_seeders: 0,
            ready_fraction: 0.05,
            // Ask by default: the picker is opt-out, so an existing reader sees
            // no change and a new one gets the choice.
            ask_every_time: true,
            theme: "system".to_string(),
            // Empty means "use the player's built-in defaults"; the UI writes
            // only the actions the reader actually changes.
            shortcuts: BTreeMap::new(),
            // Ask before seeking: the reader may want to start the episode
            // over, so a silent jump would be surprising.
            resume_behavior: "ask".to_string(),
            // 20s back by default: enough context without losing much.
            resume_rewind_seconds: 20,
            // Mbps by default: it is how connections are sold, and what the
            // result has always read as. MiB/s is opt-in.
            use_binary_units: false,
            // English first: that is what every surface showed before this
            // setting existed, so an existing reader sees no change.
            title_language: "english".to_string(),
        }
    }
}

impl Settings {
    /// The auto-launch threshold, clamped to a usable range.
    ///
    /// A value outside `(0, 1]` would either launch nothing or launch before a
    /// byte arrived, so it is clamped rather than trusted. A non-finite value
    /// (NaN, inf) falls back to the default.
    pub fn ready_fraction(&self) -> f64 {
        if !self.ready_fraction.is_finite() || self.ready_fraction <= 0.0 {
            return Settings::default().ready_fraction;
        }
        self.ready_fraction.min(1.0)
    }

    /// The title form to display, validated.
    ///
    /// Only `"romaji"` and `"english"` are meaningful; anything else -- a value
    /// written by a newer build, a typo, or a hand-edited file -- falls back to
    /// `"english"` rather than propagating garbage into the UI. Same
    /// clamp-don't-trust stance as [`Settings::ready_fraction`].
    pub fn title_language(&self) -> &str {
        match self.title_language.as_str() {
            "romaji" => "romaji",
            _ => "english",
        }
    }
}

/// Reads and writes the settings file.
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// The store at the default location for this platform.
    ///
    /// Falls back to the temp directory when the OS reports no data directory,
    /// matching the other stores: the app keeps working and the settings simply
    /// do not survive a reboot.
    pub fn new() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("kitsune")
            .join("settings.json");

        Self { path }
    }

    /// A store at an explicit path, for tests.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The stored settings, or the defaults.
    ///
    /// A missing file, unreadable file, or malformed JSON all mean "defaults"
    /// rather than an error: there is nothing a caller could do about a corrupt
    /// preferences file except rewrite it, and refusing to start over one would
    /// be worse.
    pub fn read(&self) -> Settings {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Settings::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    /// Replace the stored settings.
    pub fn write(&self, settings: &Settings) -> Result<()> {
        self.write_bytes(settings)
    }

    /// Path the store reads and writes, for tests and diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Serialise and write, creating the directory if needed.
    ///
    /// Written to a sibling temp file and renamed into place, so a crash
    /// mid-write cannot leave a half-written settings file behind -- the same
    /// reasoning as `TokenStore`.
    fn write_bytes(&self, settings: &Settings) -> Result<()> {
        let dir = self
            .path
            .parent()
            .context("settings path has no parent directory")?;

        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let json = serde_json::to_string_pretty(settings).context("failed to encode settings")?;

        let temp = self.path.with_extension("json.tmp");
        std::fs::write(&temp, json)
            .with_context(|| format!("failed to write {}", temp.display()))?;

        std::fs::rename(&temp, &self.path).with_context(|| {
            format!(
                "failed to move {} into {}",
                temp.display(),
                self.path.display()
            )
        })?;

        Ok(())
    }
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = SettingsStore::at(dir.path().join("nested").join("settings.json"));
        (dir, store)
    }

    #[test]
    fn a_fresh_store_returns_defaults() {
        let (_dir, store) = store();
        let settings = store.read();
        assert_eq!(settings.player, crate::player::DEFAULT_PLAYER);
        assert_eq!(settings.theme, "system");
        assert!(!settings.preferred_resolutions.is_empty());
    }

    #[test]
    fn settings_round_trip() {
        let (_dir, store) = store();
        let settings = Settings {
            player: "flatpak".into(),
            player_args: vec!["run".into(), "io.mpv.Mpv".into()],
            download_dir: Some("/data/anime".into()),
            preferred_resolutions: vec![Resolution::R2160p],
            min_seeders: 5,
            ready_fraction: 0.1,
            ask_every_time: false,
            theme: "dark".into(),
            shortcuts: [("playPause".to_string(), "K".to_string())]
                .into_iter()
                .collect(),
            resume_behavior: "seek".to_string(),
            resume_rewind_seconds: 35,
            use_binary_units: true,
            title_language: "romaji".to_string(),
        };

        store.write(&settings).expect("write");
        assert_eq!(store.read(), settings);
    }

    /// A settings file written before `shortcuts` existed must still parse:
    /// the field defaults to empty, so every action keeps its built-in key.
    #[test]
    fn legacy_file_without_shortcuts_reads_as_empty_map() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(
            store.path(),
            r#"{"player":"mpv","theme":"dark","readyFraction":0.2}"#,
        )
        .unwrap();

        let s = store.read();
        assert_eq!(s.theme, "dark");
        assert!(s.shortcuts.is_empty());
    }

    #[test]
    fn a_corrupt_file_reads_as_defaults() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), "{ not json").unwrap();

        assert_eq!(store.read(), Settings::default());
    }

    #[test]
    fn a_partial_file_fills_missing_fields() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), r#"{"player":"vlc"}"#).unwrap();

        let settings = store.read();
        assert_eq!(settings.player, "vlc");
        // The unset fields fall back rather than failing the whole parse.
        assert_eq!(settings.theme, "system");
    }

    #[test]
    fn ready_fraction_is_clamped() {
        let mut settings = Settings::default();
        settings.ready_fraction = 5.0;
        assert_eq!(settings.ready_fraction(), 1.0);

        settings.ready_fraction = 0.0;
        assert_eq!(settings.ready_fraction(), 0.05);

        settings.ready_fraction = f64::NAN;
        assert_eq!(settings.ready_fraction(), 0.05);
    }

    #[test]
    fn ask_every_time_defaults_to_true() {
        // The prompt is opt-out, so an existing reader sees no change.
        assert!(Settings::default().ask_every_time);
    }

    #[test]
    fn title_language_defaults_to_english() {
        // Every surface showed the English-first order before this setting
        // existed, so a fresh install must not change what anyone sees.
        assert_eq!(Settings::default().title_language(), "english");
    }

    #[test]
    fn title_language_accepts_romaji() {
        let mut settings = Settings::default();
        settings.title_language = "romaji".to_string();
        assert_eq!(settings.title_language(), "romaji");
    }

    /// A value this build does not know -- written by a newer one, hand-edited,
    /// or a typo -- must not reach the UI.
    #[test]
    fn an_unknown_title_language_falls_back_to_english() {
        let mut settings = Settings::default();

        for nonsense in ["", "Romaji", "native", "klingon"] {
            settings.title_language = nonsense.to_string();
            assert_eq!(
                settings.title_language(),
                "english",
                "{nonsense:?} should not be usable"
            );
        }
    }

    /// A settings file from before the field existed must still parse, and the
    /// missing key must land on the English default.
    #[test]
    fn a_legacy_file_without_title_language_reads_as_english() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), r#"{"player":"mpv","theme":"dark"}"#).unwrap();

        let settings = store.read();
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.title_language(), "english");
    }

    #[test]
    fn title_language_round_trips() {
        let (_dir, store) = store();
        let mut settings = Settings::default();
        settings.title_language = "romaji".to_string();

        store.write(&settings).expect("write");
        assert_eq!(store.read().title_language(), "romaji");
    }
}