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

/// One episode's remembered release.
///
/// Keyed per EPISODE, not per work. A single record per work meant watching
/// episode 6 overwrote episode 3's release, so resuming episode 3 replayed
/// episode 6's torrent and picked the wrong file out of it.
///
/// `anime_id` alone is not an identity either: Kitsu's `12` and AniList's `12`
/// are different works, so `provider` travels with it -- the same rule the
/// media list already follows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRelease {
    pub anime_id: i64,
    /// The id's namespace ("anilist" | "kitsu" | "tmdb").
    ///
    /// `default` so a record written before this field existed still parses;
    /// [`LastReleaseStore::read`] treats an empty namespace as AniList, which
    /// is what every caller meant before the parameter existed.
    #[serde(default)]
    pub provider: String,
    /// The episode number this release belongs to.
    ///
    /// `None` on a legacy record, which is what makes the fallback arm in
    /// [`LastReleaseStore::read`] reachable.
    #[serde(default)]
    pub episode: Option<u32>,
    /// The release's magnet URI, enough to re-add it.
    pub magnet: String,
    /// The torrent's info hash, when it was known.
    ///
    /// This is the key into the `.torrent` metadata cache, which is what lets a
    /// resume skip the DHT round trip entirely. Absent on a legacy record.
    #[serde(default)]
    pub info_hash: Option<String>,
    /// The file index within the torrent that was played.
    pub file_idx: u32,
    /// The file's display name, for a friendlier readout.
    #[serde(default)]
    pub name: String,
    /// Unix seconds when it was recorded. Drives eviction.
    pub at: i64,
}

impl LastRelease {
    /// The map key: one record per (namespace, work, episode).
    pub fn key(provider: &str, anime_id: i64, episode: u32) -> String {
        format!("{}:{}:{}", namespace(provider), anime_id, episode)
    }

    /// The key a legacy (pre-episode) record was written under.
    fn legacy_key(anime_id: i64) -> String {
        anime_id.to_string()
    }
}

/// Normalise a namespace spelling to the one stored on disk.
fn namespace(provider: &str) -> String {
    match provider.trim().to_ascii_lowercase().as_str() {
        "kitsu" => "kitsu".to_string(),
        "tmdb" => "tmdb".to_string(),
        _ => "anilist".to_string(),
    }
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

    /// The remembered release for one episode, or `None`.
    ///
    /// Two lookups, in order:
    ///
    /// 1. the composite `{provider}:{anime_id}:{episode}` key, which is what
    ///    every write since per-episode keying lands under;
    /// 2. the legacy bare-`anime_id` key, written by older builds that kept one
    ///    record per work. Without this arm an upgrade would look like "resume
    ///    forgot everything" until the reader watched something again.
    ///
    /// The legacy record is only accepted when its namespace and episode agree
    /// (or are absent) -- replaying another work's torrent, or another
    /// episode's, is worse than falling back to a fresh search.
    pub fn read(&self, provider: &str, anime_id: i64, episode: u32) -> Option<LastRelease> {
        let mut all = self.read_all();
        let ns = namespace(provider);

        if let Some(found) = all.remove(&LastRelease::key(&ns, anime_id, episode)) {
            return Some(found);
        }

        let legacy = all.remove(&LastRelease::legacy_key(anime_id))?;
        if legacy.anime_id != anime_id {
            return None;
        }
        // An empty namespace is what a pre-`provider` build wrote.
        if !legacy.provider.is_empty() && namespace(&legacy.provider) != ns {
            return None;
        }
        match legacy.episode {
            Some(legacy_episode) if legacy_episode != episode => None,
            _ => Some(legacy),
        }
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

    /// Remember that one episode of a work played `magnet` at `file_idx`.
    ///
    /// Overwrites any previous release for THAT episode only -- the whole point
    /// of per-episode keying. Evicts the oldest when the cap is exceeded.
    pub fn write(
        &self,
        provider: &str,
        anime_id: i64,
        episode: u32,
        magnet: &str,
        info_hash: Option<&str>,
        file_idx: u32,
        name: &str,
    ) -> Result<()> {
        let namespace = namespace(provider);
        let mut entries = self.read_all();
        entries.insert(
            LastRelease::key(&namespace, anime_id, episode),
            LastRelease {
                anime_id,
                provider: namespace,
                episode: Some(episode),
                magnet: magnet.to_string(),
                info_hash: info_hash.map(|hash| hash.trim().to_ascii_lowercase()),
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

    /// Forget one episode's release.
    pub fn clear(&self, provider: &str, anime_id: i64, episode: u32) -> Result<()> {
        let mut entries = self.read_all();
        if entries
            .remove(&LastRelease::key(provider, anime_id, episode))
            .is_none()
        {
            return Ok(());
        }
        self.write_bytes(&Stored { entries })
    }

    /// Forget every remembered release for a work, all episodes and the legacy
    /// record included. Used when a work is removed from the list.
    pub fn clear_work(&self, provider: &str, anime_id: i64) -> Result<()> {
        let ns = namespace(provider);
        let mut entries = self.read_all();
        let before = entries.len();
        entries.retain(|_, entry| {
            if entry.anime_id != anime_id {
                return true;
            }
            let entry_namespace = namespace(&entry.provider);
            entry_namespace != ns
        });
        // The legacy key is not namespace-tagged, so it is checked separately.
        entries.remove(&LastRelease::legacy_key(anime_id));
        if entries.len() == before {
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
        assert_eq!(store.read("anilist", 21, 1), None);
    }

    #[test]
    fn a_release_round_trips() {
        let (_dir, store) = store();

        store
            .write("anilist", 21, 1, "magnet:?xt=urn:btih:abc", None, 2, "ep1.mkv")
            .expect("write");

        let read = store.read("anilist", 21, 1).expect("a release");
        assert_eq!(read.anime_id, 21);
        assert_eq!(read.provider, "anilist");
        assert_eq!(read.episode, Some(1));
        assert_eq!(read.magnet, "magnet:?xt=urn:btih:abc");
        assert_eq!(read.file_idx, 2);
        assert_eq!(read.name, "ep1.mkv");
    }

    #[test]
    fn the_info_hash_is_stored_lowercase() {
        let (_dir, store) = store();

        store
            .write("anilist", 21, 1, "magnet:a", Some("ABCDEF0123"), 0, "a.mkv")
            .expect("write");

        assert_eq!(
            store.read("anilist", 21, 1).expect("a release").info_hash,
            Some("abcdef0123".to_string())
        );
    }

    /// THE BUG THIS GUARDS: one record per work meant watching episode 6
    /// overwrote episode 3's release, so resuming episode 3 replayed episode
    /// 6's torrent.
    #[test]
    fn episodes_of_one_work_are_kept_apart() {
        let (_dir, store) = store();

        store.write("anilist", 21, 3, "magnet:ep3", None, 2, "ep3.mkv").expect("write");
        store.write("anilist", 21, 6, "magnet:ep6", None, 5, "ep6.mkv").expect("write");

        assert_eq!(store.read("anilist", 21, 3).expect("ep3").magnet, "magnet:ep3");
        assert_eq!(store.read("anilist", 21, 6).expect("ep6").magnet, "magnet:ep6");
    }

    #[test]
    fn a_second_write_replaces_the_first_for_that_episode() {
        let (_dir, store) = store();

        store.write("anilist", 21, 1, "magnet:a", None, 0, "a.mkv").expect("write");
        store.write("anilist", 21, 1, "magnet:b", None, 1, "b.mkv").expect("write");

        let read = store.read("anilist", 21, 1).expect("a release");
        assert_eq!(read.magnet, "magnet:b");
        assert_eq!(read.file_idx, 1);
    }

    #[test]
    fn works_are_kept_apart() {
        let (_dir, store) = store();

        store.write("anilist", 1, 1, "magnet:1", None, 0, "1.mkv").expect("write");
        store.write("anilist", 2, 1, "magnet:2", None, 3, "2.mkv").expect("write");

        assert_eq!(store.read("anilist", 1, 1).expect("work 1").magnet, "magnet:1");
        assert_eq!(store.read("anilist", 2, 1).expect("work 2").file_idx, 3);
    }

    /// Kitsu `21` and AniList `21` are different works.
    #[test]
    fn namespaces_are_kept_apart() {
        let (_dir, store) = store();

        store.write("anilist", 21, 1, "magnet:anilist", None, 0, "a.mkv").expect("write");
        store.write("kitsu", 21, 1, "magnet:kitsu", None, 0, "k.mkv").expect("write");

        assert_eq!(store.read("anilist", 21, 1).expect("anilist").magnet, "magnet:anilist");
        assert_eq!(store.read("kitsu", 21, 1).expect("kitsu").magnet, "magnet:kitsu");
    }

    /// An upgrade must not look like "resume forgot everything".
    #[test]
    fn a_legacy_record_still_resolves() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(
            store.path(),
            r#"{"entries":{"21":{"animeId":21,"magnet":"magnet:old","fileIdx":4,"name":"old.mkv","at":1}}}"#,
        )
        .unwrap();

        let read = store.read("anilist", 21, 7).expect("legacy fallback");
        assert_eq!(read.magnet, "magnet:old");
        assert_eq!(read.file_idx, 4);
    }

    /// But a legacy record for another namespace must not answer.
    #[test]
    fn a_legacy_record_does_not_cross_namespaces() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(
            store.path(),
            r#"{"entries":{"21":{"animeId":21,"provider":"kitsu","magnet":"magnet:k","fileIdx":0,"name":"","at":1}}}"#,
        )
        .unwrap();

        assert_eq!(store.read("anilist", 21, 1), None);
        assert!(store.read("kitsu", 21, 1).is_some());
    }

    /// A legacy record that names an episode must not answer for another one.
    #[test]
    fn a_legacy_record_does_not_cross_episodes() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(
            store.path(),
            r#"{"entries":{"21":{"animeId":21,"episode":4,"magnet":"magnet:e4","fileIdx":0,"name":"","at":1}}}"#,
        )
        .unwrap();

        assert_eq!(store.read("anilist", 21, 3), None);
        assert!(store.read("anilist", 21, 4).is_some());
    }

    #[test]
    fn clearing_one_episode_leaves_its_siblings() {
        let (_dir, store) = store();

        store.write("anilist", 21, 1, "magnet:1", None, 0, "").expect("write");
        store.write("anilist", 21, 2, "magnet:2", None, 0, "").expect("write");

        store.clear("anilist", 21, 1).expect("clear");

        assert_eq!(store.read("anilist", 21, 1), None);
        assert!(store.read("anilist", 21, 2).is_some());
    }

    #[test]
    fn clearing_a_work_removes_every_episode() {
        let (_dir, store) = store();

        store.write("anilist", 21, 1, "magnet:1", None, 0, "").expect("write");
        store.write("anilist", 21, 2, "magnet:2", None, 0, "").expect("write");
        store.write("anilist", 22, 1, "magnet:other", None, 0, "").expect("write");

        store.clear_work("anilist", 21).expect("clear");

        assert_eq!(store.read("anilist", 21, 1), None);
        assert_eq!(store.read("anilist", 21, 2), None);
        assert!(store.read("anilist", 22, 1).is_some());
    }

    #[test]
    fn the_oldest_entry_is_evicted_at_the_cap() {
        let (_dir, store) = store();

        for id in 0..(MAX_ENTRIES as i64 + 5) {
            store.write("anilist", id, 1, "magnet:x", None, 0, "x.mkv").expect("write");
        }

        assert_eq!(store.read_all().len(), MAX_ENTRIES);
        assert_eq!(store.read("anilist", 0, 1), None);
        assert!(store.read("anilist", MAX_ENTRIES as i64 + 4, 1).is_some());
    }

    #[test]
    fn a_corrupt_file_reads_as_nothing() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), "{ not json").unwrap();

        assert_eq!(store.read("anilist", 21, 1), None);
    }
}