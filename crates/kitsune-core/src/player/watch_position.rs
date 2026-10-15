//! Where the reader stopped, per work.
//!
//! The AniList list records HOW MANY episodes were watched, not where in an
//! episode the reader paused. A resume button that jumps to the right episode
//! is only half the job; the disc's `MM:SS` needs the seconds too. That is a
//! local fact, so it is kept locally.
//!
//! A capped map keyed by anime id rather than a single record: the reader may
//! bounce between several shows, and the row's resume button needs each work's
//! own position. Oldest entries are evicted once the cap is reached, so the
//! file cannot grow without bound.
//!
//! Field names are camelCase to match the frontend, and the file is a plain
//! JSON in the OS data directory, alongside the token and last-played stores.
//! Not a credential, so it is not permission-restricted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// How many works are remembered. Enough for an active reader's rotation
/// without the file growing forever.
pub const MAX_ENTRIES: usize = 50;

/// One work's saved position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchPosition {
    pub anime_id: i64,
    /// The episode number, when one was known.
    pub episode: Option<u32>,
    /// Seconds into the episode where the reader stopped.
    pub position_seconds: f64,
    /// The episode's total length in seconds, when known.
    pub duration_seconds: f64,
    /// Unix seconds when it was recorded. Drives eviction and "last watched".
    pub at: i64,
}

/// The on-disk shape: the entries plus nothing else.
///
/// A map keyed by the anime id as a string (JSON object keys are strings), so a
/// lookup does not scan a list and the file stays human-readable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    entries: BTreeMap<String, WatchPosition>,
}

/// Reads and writes saved watch positions.
pub struct WatchPositionStore {
    path: PathBuf,
}

impl WatchPositionStore {
    /// The store at the default location for this platform.
    pub fn new() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("kitsune")
            .join("watch-positions.json");

        Self { path }
    }

    /// A store at an explicit path, for tests.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The saved position for `anime_id`, or `None` when nothing is recorded.
    ///
    /// A missing file, unreadable file, or malformed JSON all mean "nothing
    /// recorded" rather than an error: there is nothing useful to do about a
    /// corrupt hint file, and the resume button simply falls back to the start
    /// of the episode.
    pub fn read(&self, anime_id: i64) -> Option<WatchPosition> {
        self.read_all().remove(&anime_id.to_string())
    }

    /// Every saved position, keyed by anime id.
    pub fn read_all(&self) -> BTreeMap<String, WatchPosition> {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return BTreeMap::new();
        };
        serde_json::from_str::<Stored>(&text)
            .map(|stored| stored.entries)
            .unwrap_or_default()
    }

    /// Record where the reader stopped in `anime_id`'s `episode`.
    ///
    /// Overwrites any previous position for the work: only the latest stop
    /// matters. Evicts the oldest entry when the cap is exceeded.
    pub fn write(
        &self,
        anime_id: i64,
        episode: Option<u32>,
        position_seconds: f64,
        duration_seconds: f64,
    ) -> Result<()> {
        let mut entries = self.read_all();
        entries.insert(
            anime_id.to_string(),
            WatchPosition {
                anime_id,
                episode,
                position_seconds,
                duration_seconds,
                at: now_seconds(),
            },
        );

        // Evict the oldest entries beyond the cap. `at` is the recency key.
        while entries.len() > MAX_ENTRIES {
            let oldest = entries
                .iter()
                .min_by_key(|(_, entry)| entry.at)
                .map(|(key, _)| key.clone());
            match oldest {
                Some(key) => {
                    entries.remove(&key);
                }
                None => break,
            }
        }

        self.write_bytes(&Stored { entries })
    }

    /// Forget one work's position.
    ///
    /// A missing file or entry is success, so clearing twice is harmless.
    pub fn clear(&self, anime_id: i64) -> Result<()> {
        let mut entries = self.read_all();
        if entries.remove(&anime_id.to_string()).is_none() {
            return Ok(());
        }
        self.write_bytes(&Stored { entries })
    }

    /// Serialise and write, creating the directory if needed.
    ///
    /// Written to a sibling temp file and renamed into place, so a crash
    /// mid-write cannot leave a half-written record behind -- the same
    /// reasoning as `LastPlayedStore`.
    fn write_bytes(&self, stored: &Stored) -> Result<()> {
        let dir = self
            .path
            .parent()
            .context("watch-position path has no parent directory")?;

        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let json = serde_json::to_string_pretty(stored)
            .context("failed to encode watch positions")?;

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

impl Default for WatchPositionStore {
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

    fn store() -> (tempfile::TempDir, WatchPositionStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = WatchPositionStore::at(dir.path().join("nested").join("watch-positions.json"));
        (dir, store)
    }

    #[test]
    fn a_fresh_store_has_nothing() {
        let (_dir, store) = store();

        assert_eq!(store.read(21), None);
    }

    #[test]
    fn a_position_round_trips() {
        let (_dir, store) = store();

        store.write(21, Some(3), 123.5, 1440.0).expect("write");

        let read = store.read(21).expect("a position");
        assert_eq!(read.anime_id, 21);
        assert_eq!(read.episode, Some(3));
        assert_eq!(read.position_seconds, 123.5);
        assert_eq!(read.duration_seconds, 1440.0);
    }

    #[test]
    fn an_episode_is_optional() {
        let (_dir, store) = store();

        store.write(21, None, 10.0, 0.0).expect("write");

        assert_eq!(store.read(21).expect("a position").episode, None);
    }

    #[test]
    fn a_second_write_for_the_same_work_replaces_the_first() {
        let (_dir, store) = store();

        store.write(21, Some(1), 10.0, 100.0).expect("write");
        store.write(21, Some(2), 20.0, 200.0).expect("write");

        let read = store.read(21).expect("a position");
        assert_eq!(read.episode, Some(2));
        assert_eq!(read.position_seconds, 20.0);
    }

    #[test]
    fn several_works_are_kept_apart() {
        let (_dir, store) = store();

        store.write(1, Some(1), 10.0, 100.0).expect("write");
        store.write(2, Some(5), 50.0, 500.0).expect("write");

        assert_eq!(store.read(1).expect("work 1").episode, Some(1));
        assert_eq!(store.read(2).expect("work 2").episode, Some(5));
    }

    #[test]
    fn clearing_a_work_forgets_only_that_work() {
        let (_dir, store) = store();

        store.write(1, Some(1), 10.0, 100.0).expect("write");
        store.write(2, Some(2), 20.0, 200.0).expect("write");
        store.clear(1).expect("clear");

        assert_eq!(store.read(1), None);
        assert!(store.read(2).is_some());
    }

    /// The cap keeps the file bounded: writing one more than the cap must not
    /// leave `MAX_ENTRIES + 1` entries behind.
    #[test]
    fn the_oldest_entry_is_evicted_at_the_cap() {
        let (_dir, store) = store();

        for id in 0..(MAX_ENTRIES as i64 + 5) {
            store.write(id, Some(1), 1.0, 2.0).expect("write");
        }

        assert_eq!(store.read_all().len(), MAX_ENTRIES);
        // The very first ids are gone; the most recent survive.
        assert_eq!(store.read(0), None);
        assert!(store.read(MAX_ENTRIES as i64 + 4).is_some());
    }

    #[test]
    fn a_corrupt_file_reads_as_nothing() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), "{ not json").unwrap();

        assert_eq!(store.read(21), None);
    }
}