//! The work the reader most recently OPENED.
//!
//! Separate from the AniList list on purpose. AniList's `updatedAt` records
//! the last CHANGE to an entry, so re-watching the episode you are already on
//! does not bump it -- which is correct for a list, but means the list order
//! cannot answer "what did I last open". That is a local fact, so it is kept
//! locally.
//!
//! A single record, not a history: only the most recent work is ever needed to
//! pick up where the reader left off. A plain JSON file in the OS data
//! directory, alongside the token store. Not a credential, so it is not
//! permission-restricted.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// The on-disk shape.
///
/// Field names are camelCase to match the frontend's `LastPlayed`, so the
/// file and the wire shape do not disagree. Optional so a partially-written
/// record still parses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    #[serde(skip_serializing_if = "Option::is_none")]
    anime_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    episode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<i64>,
}

/// The work the reader most recently opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastPlayed {
    pub anime_id: i64,
    /// The episode number, when one was known. `None` for a film or a work
    /// played without a selected episode.
    pub episode: Option<u32>,
    /// Unix seconds when it was recorded. Informational today; it lets a later
    /// feature show recency or expire a stale record without a migration.
    pub at: i64,
}

/// Reads and writes the last-opened record.
pub struct LastPlayedStore {
    path: PathBuf,
}

impl LastPlayedStore {
    /// The store at the default location for this platform.
    ///
    /// Falls back to the temp directory when the OS reports no data directory,
    /// matching `TokenStore`: the app keeps working rather than panicking, and
    /// the record simply does not survive a reboot.
    pub fn new() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("kitsune")
            .join("last-played.json");

        Self { path }
    }

    /// A store at an explicit path, for tests.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The most recently opened work, or `None` when nothing is recorded.
    ///
    /// A missing file, unreadable file, or malformed JSON all mean "nothing
    /// recorded" rather than an error: there is nothing useful to do about a
    /// corrupt hint file, and the disc simply falls back to the list.
    pub fn read(&self) -> Option<LastPlayed> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        let stored: Stored = serde_json::from_str(&text).ok()?;

        Some(LastPlayed {
            // `anime_id` is the one required field: without it there is no work
            // to resume, so the record is treated as absent.
            anime_id: stored.anime_id?,
            episode: stored.episode,
            at: stored.at.unwrap_or(0),
        })
    }

    /// Record that `anime_id` was opened, at `episode` when one is known.
    ///
    /// Overwrites whatever was there: only the most recent work matters.
    pub fn write(&self, anime_id: i64, episode: Option<u32>) -> Result<()> {
        self.write_bytes(&Stored {
            anime_id: Some(anime_id),
            episode,
            at: Some(now_seconds()),
        })
    }

    /// Forget the record.
    ///
    /// A missing file is success, so clearing twice is harmless.
    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err)
                .with_context(|| format!("failed to remove {}", self.path.display())),
        }
    }

    /// Serialise and write, creating the directory if needed.
    ///
    /// Written to a sibling temp file and renamed into place, so a crash
    /// mid-write cannot leave a half-written record behind -- the same
    /// reasoning as `TokenStore`.
    fn write_bytes(&self, stored: &Stored) -> Result<()> {
        let dir = self
            .path
            .parent()
            .context("last-played path has no parent directory")?;

        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let json =
            serde_json::to_string_pretty(stored).context("failed to encode last-played")?;

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

    /// Path the store reads and writes, for tests and diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Default for LastPlayedStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Unix seconds now, or `0` if the clock is before the epoch.
fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, LastPlayedStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = LastPlayedStore::at(dir.path().join("nested").join("last-played.json"));
        (dir, store)
    }

    #[test]
    fn a_fresh_store_has_nothing() {
        let (_dir, store) = store();

        assert_eq!(store.read(), None);
    }

    #[test]
    fn a_record_round_trips() {
        let (_dir, store) = store();

        store.write(21, Some(3)).expect("write");

        let read = store.read().expect("a record");
        assert_eq!(read.anime_id, 21);
        assert_eq!(read.episode, Some(3));
    }

    /// A work played with no episode selected (a film, or a release clicked
    /// without picking one) still needs a record -- that is the whole point.
    #[test]
    fn an_episode_is_optional() {
        let (_dir, store) = store();

        store.write(21, None).expect("write");

        let read = store.read().expect("a record");
        assert_eq!(read.anime_id, 21);
        assert_eq!(read.episode, None);
    }

    #[test]
    fn writing_creates_missing_directories() {
        let (_dir, store) = store();

        store.write(21, Some(1)).expect("write should create parents");

        assert!(store.path().exists());
    }

    /// Only the most recent work matters, so a second write replaces the first.
    #[test]
    fn writing_replaces_the_previous_record() {
        let (_dir, store) = store();

        store.write(21, Some(3)).expect("first");
        store.write(99, Some(1)).expect("second");

        assert_eq!(store.read().expect("a record").anime_id, 99);
    }

    /// The timestamp is recorded so a later feature can reason about recency
    /// without a migration.
    #[test]
    fn a_record_carries_a_timestamp() {
        let (_dir, store) = store();

        store.write(21, Some(3)).expect("write");

        assert!(store.read().expect("a record").at > 0);
    }

    #[test]
    fn clearing_removes_the_record() {
        let (_dir, store) = store();

        store.write(21, Some(3)).expect("write");
        store.clear().expect("clear");

        assert_eq!(store.read(), None);
    }

    /// A corrupt file is a missing record, not an error: the disc falls back
    /// to the list rather than failing.
    #[test]
    fn a_corrupt_file_reads_as_nothing() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), "{ not json").unwrap();

        assert_eq!(store.read(), None);
    }

    /// Without an anime id there is no work to resume, so the record is
    /// treated as absent rather than surfacing a broken entry.
    #[test]
    fn a_record_without_an_anime_id_is_ignored() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), r#"{ "episode": 3 }"#).unwrap();

        assert_eq!(store.read(), None);
    }
}