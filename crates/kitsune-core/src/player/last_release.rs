//! The release the reader last played, per work.
//!
//! A torrent id dies with the session, so "play the same release again" cannot
//! be answered from the engine. It has to be remembered as the magnet plus the
//! file index, which is exactly what is needed to re-add it and start the same
//! file.
//!
//! A capped map keyed by anime id, written the same way as the other stores
//! (temp file + atomic rename). Field names are camelCase to match the frontend.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// How many works are remembered. Matches the watch-position cap.
pub const MAX_ENTRIES: usize = 50;

/// One work's remembered release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRelease {
    pub anime_id: i64,
    /// The release's magnet URI, enough to re-add it.
    pub magnet: String,
    /// The file index within the torrent that was played.
    pub file_idx: u32,
    /// The file's display name, for a friendlier readout.
    #[serde(default)]
    pub name: String,
    /// Unix seconds when it was recorded. Drives eviction.
    pub at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    entries: BTreeMap<String, LastRelease>,
}

/// Reads and writes the remembered release per work.
pub struct LastReleaseStore {
    path: PathBuf,
}

impl LastReleaseStore {
    pub fn new() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("kitsune")
            .join("last-releases.json");

        Self { path }
    }

    /// A store at an explicit path, for tests.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The remembered release for `anime_id`, or `None`.
    pub fn read(&self, anime_id: i64) -> Option<LastRelease> {
        self.read_all().remove(&anime_id.to_string())
    }

    /// Every remembered release, keyed by anime id.
    pub fn read_all(&self) -> BTreeMap<String, LastRelease> {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return BTreeMap::new();
        };
        serde_json::from_str::<Stored>(&text)
            .map(|stored| stored.entries)
            .unwrap_or_default()
    }

    /// Remember that `anime_id` played `magnet` at `file_idx`.
    ///
    /// Overwrites any previous release for the work; evicts the oldest when the
    /// cap is exceeded.
    pub fn write(
        &self,
        anime_id: i64,
        magnet: &str,
        file_idx: u32,
        name: &str,
    ) -> Result<()> {
        let mut entries = self.read_all();
        entries.insert(
            anime_id.to_string(),
            LastRelease {
                anime_id,
                magnet: magnet.to_string(),
                file_idx,
                name: name.to_string(),
                at: now_seconds(),
            },
        );

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

    /// Forget one work's release.
    pub fn clear(&self, anime_id: i64) -> Result<()> {
        let mut entries = self.read_all();
        if entries.remove(&anime_id.to_string()).is_none() {
            return Ok(());
        }
        self.write_bytes(&Stored { entries })
    }

    fn write_bytes(&self, stored: &Stored) -> Result<()> {
        let dir = self
            .path
            .parent()
            .context("last-release path has no parent directory")?;

        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let json =
            serde_json::to_string_pretty(stored).context("failed to encode last releases")?;

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

impl Default for LastReleaseStore {
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

    fn store() -> (tempfile::TempDir, LastReleaseStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = LastReleaseStore::at(dir.path().join("nested").join("last-releases.json"));
        (dir, store)
    }

    #[test]
    fn a_fresh_store_has_nothing() {
        let (_dir, store) = store();
        assert_eq!(store.read(21), None);
    }

    #[test]
    fn a_release_round_trips() {
        let (_dir, store) = store();

        store.write(21, "magnet:?xt=urn:btih:abc", 2, "ep1.mkv").expect("write");

        let read = store.read(21).expect("a release");
        assert_eq!(read.anime_id, 21);
        assert_eq!(read.magnet, "magnet:?xt=urn:btih:abc");
        assert_eq!(read.file_idx, 2);
        assert_eq!(read.name, "ep1.mkv");
    }

    #[test]
    fn a_second_write_replaces_the_first() {
        let (_dir, store) = store();

        store.write(21, "magnet:a", 0, "a.mkv").expect("write");
        store.write(21, "magnet:b", 1, "b.mkv").expect("write");

        let read = store.read(21).expect("a release");
        assert_eq!(read.magnet, "magnet:b");
        assert_eq!(read.file_idx, 1);
    }

    #[test]
    fn works_are_kept_apart() {
        let (_dir, store) = store();

        store.write(1, "magnet:1", 0, "1.mkv").expect("write");
        store.write(2, "magnet:2", 3, "2.mkv").expect("write");

        assert_eq!(store.read(1).expect("work 1").magnet, "magnet:1");
        assert_eq!(store.read(2).expect("work 2").file_idx, 3);
    }

    #[test]
    fn the_oldest_entry_is_evicted_at_the_cap() {
        let (_dir, store) = store();

        for id in 0..(MAX_ENTRIES as i64 + 5) {
            store.write(id, "magnet:x", 0, "x.mkv").expect("write");
        }

        assert_eq!(store.read_all().len(), MAX_ENTRIES);
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