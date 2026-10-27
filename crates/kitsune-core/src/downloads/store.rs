//! Persistent store for downloaded episodes.
//!
//! A plain JSON file in the OS data directory, written atomically (temp file +
//! rename). Tracks download history, status, and file locations.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Status of a download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    /// Currently downloading.
    Downloading,
    /// Waiting for an active download slot.
    Queued,
    /// Paused by the user.
    Paused,
    /// Download completed successfully.
    Complete,
    /// Download failed.
    Failed,
}

/// Scheduling priority for a download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadPriority {
    High,
    Normal,
    Low,
}

impl Default for DownloadPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl DownloadPriority {
    pub fn rank(self) -> u8 {
        match self {
            Self::High => 0,
            Self::Normal => 1,
            Self::Low => 2,
        }
    }
}

/// A single downloaded episode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    /// Unique identifier.
    pub id: u64,
    /// AniList/Kitsu ID of the anime.
    pub anime_id: i64,
    /// Provider that supplied the metadata (e.g. "anilist", "kitsu").
    pub provider: String,
    /// Episode number within the series, if known.
    pub episode: Option<u32>,
    /// Anime title for display.
    pub title: String,
    /// Anime cover image URL, for recognising the card at a glance.
    ///
    /// Persisted when the download starts. The provider CDN URL is stable
    /// enough to store directly; QML downloads it on demand and caches it.
    #[serde(default)]
    pub cover_image: Option<String>,
    /// Episode title, if known.
    pub episode_title: Option<String>,
    /// Release title that was picked (e.g. "[Subs] Show - 01 [1080p]").
    pub release_title: String,
    /// Linked torrent session ID, if the torrent is still active.
    pub torrent_id: Option<usize>,
    /// Magnet URI, if the download started from a magnet.
    pub magnet: Option<String>,
    /// Torrent file URL, if the download started from a .torrent file.
    pub torrent_url: Option<String>,
    /// Stable torrent identity, lowercase hex.
    ///
    /// The engine's numeric torrent ID is process-local, so it cannot identify
    /// a torrent after a restart. This is the key used to regroup sibling
    /// records and to find the cached `.torrent` metadata.
    #[serde(default)]
    pub info_hash: Option<String>,
    /// Directory holding this torrent's partial data.
    ///
    /// Recorded per torrent rather than derived from the current setting: a
    /// reader who changes the download directory must still be able to resume
    /// what was already downloaded elsewhere.
    #[serde(default)]
    pub cache_dir: Option<String>,
    /// Index of the file within the torrent.
    pub file_idx: u32,
    /// Name of the video file (e.g. "Show - 01.mkv").
    pub file_name: String,
    /// Current status of the download.
    pub status: DownloadStatus,
    /// Bytes downloaded so far.
    pub progress_bytes: u64,
    /// Total bytes to download.
    pub total_bytes: u64,
    /// The release's seeder count as the INDEXER claimed it, when known.
    ///
    /// Kept beside the live peer count rather than instead of it: an indexer's
    /// number can be hours stale, so showing only that is how a dead release
    /// looks healthy. The UI shows this as "~N seeders" and the engine's
    /// connected peers separately.
    ///
    /// `default` so a `downloads.json` written before this field still parses.
    #[serde(default)]
    pub release_seeders: Option<u32>,
    /// Final path to the downloaded file in the library directory.
    pub output_path: String,
    /// Unix timestamp (seconds) when the download was added.
    pub added_at: i64,
    /// Unix timestamp (seconds) when the download completed, if applicable.
    pub completed_at: Option<i64>,
    /// Queue priority. Missing in older stores means Normal.
    #[serde(default)]
    pub priority: DownloadPriority,
    /// True when the reader explicitly paused this download.
    #[serde(default)]
    pub user_paused: bool,
    /// Season number from the request that started the download.
    ///
    /// Persisted because the finish path needs it after a restart; it used to
    /// live in a process-local hint map.
    #[serde(default)]
    pub season: Option<u32>,
    /// Library directory chosen when the download started.
    ///
    /// Persisted for the same reason as `season`: the setting may change while
    /// a download is paused, but the file still belongs where it was started.
    #[serde(default)]
    pub library_dir: Option<String>,
    /// Number of files the torrent contained when it was added.
    #[serde(default)]
    pub torrent_file_count: Option<u32>,
    /// Why the download failed, for the Retry UI.
    #[serde(default)]
    pub error: Option<String>,
}

impl DownloadRecord {
    /// Identity shared by every file selected from the same torrent.
    ///
    /// `info_hash` is stable across restarts; the engine ID is the fallback for
    /// records written before the hash was persisted, and a record-local key is
    /// the last resort so a lone completed row still renders.
    pub fn group_key(&self) -> String {
        if let Some(hash) = self
            .info_hash
            .as_deref()
            .map(str::trim)
            .filter(|hash| !hash.is_empty())
        {
            return hash.to_ascii_lowercase();
        }
        if let Some(id) = self.torrent_id {
            return format!("torrent:{id}");
        }
        format!("record:{}", self.id)
    }
}

/// The persistent store for download records.
///
/// Follows the same JSON-file pattern as [`crate::settings::SettingsStore`]:
/// atomic writes via temp file + rename, human-readable, no migrations.
#[derive(Debug, Clone)]
pub struct DownloadsStore {
    path: PathBuf,
    /// Serializes read-modify-write sequences within this process.
    ///
    /// Polling, completion tasks, and Qt-thread actions all touch the same
    /// file. The atomic rename prevents torn JSON, but without this lock two
    /// writers can each read the old list and the later rename silently drops
    /// the other's changes.
    lock: Arc<Mutex<()>>,
}

impl DownloadsStore {
    /// Open (or create) the store at the default location.
    ///
    /// The file is created on first write, not on construction, so a missing
    /// file simply means "no downloads yet".
    pub fn new() -> Result<Self> {
        let path = data_dir()?.join("downloads.json");
        Ok(Self::at(path))
    }

    /// Open the store at a custom path (for testing).
    pub fn at(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn read_unlocked(&self) -> Vec<DownloadRecord> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    fn write_unlocked(&self, records: &[DownloadRecord]) -> Result<()> {
        let json = serde_json::to_string_pretty(records).context("serialize downloads")?;
        atomic_write(&self.path, json.as_bytes())
    }

    /// Read all download records.
    ///
    /// Returns an empty vec if the file does not exist or is unparseable.
    pub fn read_all(&self) -> Vec<DownloadRecord> {
        let _guard = self.lock();
        self.read_unlocked()
    }

    /// Write the full list of records atomically.
    pub fn write_all(&self, records: &[DownloadRecord]) -> Result<()> {
        let _guard = self.lock();
        self.write_unlocked(records)
    }

    /// Atomically read, modify, and persist the whole list.
    ///
    /// The closure must not call back into this store.
    pub fn mutate<T>(&self, update: impl FnOnce(&mut Vec<DownloadRecord>) -> T) -> Result<T> {
        let _guard = self.lock();
        let mut records = self.read_unlocked();
        let result = update(&mut records);
        self.write_unlocked(&records)?;
        Ok(result)
    }

    /// Insert a new record and persist.
    pub fn insert(&self, record: DownloadRecord) -> Result<()> {
        let _guard = self.lock();
        let mut records = self.read_unlocked();
        records.push(record);
        self.write_unlocked(&records)
    }

    /// Insert several records, assigning consecutive unique IDs.
    ///
    /// IDs are allocated while the store is locked, so two concurrent starts
    /// cannot produce the same ID.
    pub fn insert_many(&self, mut records: Vec<DownloadRecord>) -> Result<Vec<u64>> {
        let _guard = self.lock();
        let mut all = self.read_unlocked();
        let mut next = all.iter().map(|record| record.id).max().unwrap_or(0) + 1;
        let mut ids = Vec::with_capacity(records.len());

        for record in &mut records {
            record.id = next;
            ids.push(next);
            next += 1;
        }

        all.extend(records);
        self.write_unlocked(&all)?;
        Ok(ids)
    }

    /// Update an existing record by ID and persist.
    ///
    /// Returns `Ok(true)` if the record was found and updated, `Ok(false)` if
    /// no record with that ID exists.
    pub fn update(&self, updated: &DownloadRecord) -> Result<bool> {
        let _guard = self.lock();
        let mut records = self.read_unlocked();
        let found = records.iter_mut().any(|r| {
            if r.id == updated.id {
                *r = updated.clone();
                true
            } else {
                false
            }
        });
        if found {
            self.write_unlocked(&records)?;
        }
        Ok(found)
    }

    /// Remove a record by ID and persist.
    ///
    /// Returns `Ok(true)` if the record was found and removed, `Ok(false)` if
    /// no record with that ID exists.
    pub fn remove(&self, id: u64) -> Result<bool> {
        let _guard = self.lock();
        let mut records = self.read_unlocked();
        let before = records.len();
        records.retain(|r| r.id != id);
        let removed = records.len() < before;
        if removed {
            self.write_unlocked(&records)?;
        }
        Ok(removed)
    }

    /// Generate the next unique ID.
    ///
    /// Returns `max(existing ids) + 1`, or `1` if the store is empty.
    pub fn next_id(&self) -> u64 {
        let _guard = self.lock();
        let records = self.read_unlocked();
        records.iter().map(|r| r.id).max().unwrap_or(0) + 1
    }
}

/// The OS data directory for Kitsune (e.g. `~/.local/share/kitsune`).
fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("no OS data directory")?
        .join("kitsune");
    std::fs::create_dir_all(&dir).context("create data directory")?;
    Ok(dir)
}

/// Write a file atomically: write to a temp file, then rename.
///
/// Prevents partial writes from corrupting the store if the process is
/// interrupted mid-write.
fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, data).context("write temp file")?;
    std::fs::rename(&temp, path).context("rename temp to final")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_store() -> (DownloadsStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("downloads.json");
        (DownloadsStore::at(path), dir)
    }

    fn sample_record(id: u64) -> DownloadRecord {
        DownloadRecord {
            id,
            anime_id: 1,
            provider: "anilist".to_string(),
            episode: Some(1),
            title: "Test Anime".to_string(),
            cover_image: None,
            episode_title: Some("Episode 1".to_string()),
            release_title: "[Subs] Test Anime - 01 [1080p]".to_string(),
            torrent_id: None,
            magnet: None,
            torrent_url: None,
            info_hash: None,
            cache_dir: None,
            file_idx: 0,
            file_name: "Test Anime - 01.mkv".to_string(),
            status: DownloadStatus::Complete,
            progress_bytes: 1000,
            total_bytes: 1000,
            release_seeders: None,
            output_path: "/tmp/test.mkv".to_string(),
            added_at: 1000,
            completed_at: Some(2000),
            priority: DownloadPriority::Normal,
            user_paused: false,
            season: None,
            library_dir: None,
            torrent_file_count: None,
            error: None,
        }
    }

    #[test]
    fn empty_store_returns_empty_vec() {
        let (store, _dir) = test_store();
        assert_eq!(store.read_all(), Vec::new());
    }

    #[test]
    fn insert_and_read() {
        let (store, _dir) = test_store();
        let record = sample_record(1);
        store.insert(record.clone()).unwrap();
        let all = store.read_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], record);
    }

    #[test]
    fn update_existing() {
        let (store, _dir) = test_store();
        let mut record = sample_record(1);
        store.insert(record.clone()).unwrap();

        record.status = DownloadStatus::Downloading;
        record.progress_bytes = 500;
        assert!(store.update(&record).unwrap());

        let all = store.read_all();
        assert_eq!(all[0].status, DownloadStatus::Downloading);
        assert_eq!(all[0].progress_bytes, 500);
    }

    #[test]
    fn update_nonexistent_returns_false() {
        let (store, _dir) = test_store();
        let record = sample_record(999);
        assert!(!store.update(&record).unwrap());
    }

    #[test]
    fn remove_existing() {
        let (store, _dir) = test_store();
        store.insert(sample_record(1)).unwrap();
        store.insert(sample_record(2)).unwrap();

        assert!(store.remove(1).unwrap());
        let all = store.read_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, 2);
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let (store, _dir) = test_store();
        assert!(!store.remove(999).unwrap());
    }

    #[test]
    fn next_id_empty() {
        let (store, _dir) = test_store();
        assert_eq!(store.next_id(), 1);
    }

    #[test]
    fn next_id_with_records() {
        let (store, _dir) = test_store();
        store.insert(sample_record(1)).unwrap();
        store.insert(sample_record(5)).unwrap();
        store.insert(sample_record(3)).unwrap();
        assert_eq!(store.next_id(), 6);
    }

    #[test]
    fn cover_image_round_trips_and_old_records_default_to_none() {
        let (store, _dir) = test_store();
        let mut record = sample_record(1);
        record.cover_image = Some("https://cdn.example/cover.jpg".to_string());
        store.insert(record.clone()).unwrap();
        assert_eq!(
            store.read_all()[0].cover_image,
            Some("https://cdn.example/cover.jpg".to_string())
        );

        let mut legacy = serde_json::to_value([&record]).unwrap();
        legacy[0].as_object_mut().unwrap().remove("coverImage");
        std::fs::write(&store.path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        assert_eq!(store.read_all()[0].cover_image, None);
    }

    #[test]
    fn group_key_prefers_info_hash_then_torrent_then_record() {
        let mut record = sample_record(7);
        assert_eq!(record.group_key(), "record:7");

        record.torrent_id = Some(42);
        assert_eq!(record.group_key(), "torrent:42");

        record.info_hash = Some("ABCDEF".to_string());
        assert_eq!(record.group_key(), "abcdef");
    }

    #[test]
    fn insert_many_allocates_unique_ids_atomically() {
        let (store, _dir) = test_store();
        store.insert(sample_record(9)).unwrap();

        let mut first = sample_record(0);
        first.file_idx = 1;
        let mut second = sample_record(0);
        second.file_idx = 2;
        let ids = store.insert_many(vec![first, second]).unwrap();

        assert_eq!(ids, vec![10, 11]);
        let all = store.read_all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[1].id, 10);
        assert_eq!(all[2].id, 11);
    }

    #[test]
    fn mutate_persists_in_one_read_modify_write() {
        let (store, _dir) = test_store();
        store.insert(sample_record(1)).unwrap();
        store.insert(sample_record(2)).unwrap();

        let removed = store
            .mutate(|records| {
                let before = records.len();
                records.retain(|record| record.id != 1);
                before - records.len()
            })
            .unwrap();

        assert_eq!(removed, 1);
        assert_eq!(store.read_all().len(), 1);
    }

    #[test]
    fn old_records_without_new_fields_still_parse() {
        let (store, _dir) = test_store();
        let old = serde_json::json!([{
            "id": 1,
            "animeId": 1,
            "provider": "anilist",
            "episode": 1,
            "title": "Old",
            "episodeTitle": null,
            "releaseTitle": "release",
            "torrentId": 5,
            "magnet": null,
            "torrentUrl": null,
            "fileIdx": 0,
            "fileName": "old.mkv",
            "status": "paused",
            "progressBytes": 10,
            "totalBytes": 20,
            "outputPath": "",
            "addedAt": 1,
            "completedAt": null
        }]);
        std::fs::write(&store.path, serde_json::to_vec(&old).unwrap()).unwrap();

        let records = store.read_all();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].group_key(), "torrent:5");
        assert_eq!(records[0].user_paused, false);
        assert_eq!(records[0].season, None);
    }

    #[test]
    fn atomic_write_prevents_corruption() {
        let (store, _dir) = test_store();
        store.insert(sample_record(1)).unwrap();

        // Verify the file is valid JSON
        let content = std::fs::read_to_string(&store.path).unwrap();
        let _: Vec<DownloadRecord> = serde_json::from_str(&content).unwrap();
    }
}
