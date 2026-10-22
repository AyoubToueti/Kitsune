//! The reader's media list, kept locally.
//!
//! Every read of "my list" -- the My List page, Continue Watching, the status
//! badge on a detail page -- is answered from this file rather than from
//! AniList. Three reasons:
//!
//! - **It works while AniList is down.** AniList rate-limits and has outages;
//!   a local list keeps the app usable through both.
//! - **It works signed out.** A reader who has never connected an account still
//!   gets a list.
//! - **It is provider-agnostic.** An entry records the source that issued the
//!   work, so a Kitsu-sourced title can be listed without pretending its id
//!   means anything to AniList.
//!
//! The price is synchronisation, which lives in [`super::sync`]. This module is
//! only the durable record and the rules for changing it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::types::{Anime, ListStatus, ProviderId};

/// One work on the reader's list.
///
/// `dirty` is the load-bearing field. It means "changed locally, not yet
/// pushed", and it decides who wins when the local copy and the server
/// disagree: a dirty entry is authoritative until the push succeeds, while a
/// clean one takes whatever the server says. Without it, a naive last-write
/// wins on timestamps would silently discard an offline edit -- AniList does
/// not even bump `updatedAt` for a same-progress re-watch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaListRecord {
    /// Which source issued `media_id`. Ids are per-source, so the pair is the
    /// identity: Kitsu's `12` and AniList's `12` are different works.
    pub provider: ProviderId,
    pub media_id: i64,
    /// A snapshot of the work as it looked when last seen.
    ///
    /// The list pages render a cover, a title and a score, so a record holding
    /// only ids would need one request PER ROW to draw the page -- exactly the
    /// chatter the local list exists to avoid. Storing the work means My List
    /// paints from disk with no network at all.
    ///
    /// It goes stale, which is accepted: the snapshot is refreshed on every
    /// pull and on every local add.
    pub anime: Anime,
    /// The work's AniList id, when it is known.
    ///
    /// Resolved lazily on the first push: an entry added from a Kitsu card has
    /// no AniList id until Kitsu's mapping table supplies one. `None` means
    /// "not resolvable yet", which keeps the entry local-only rather than
    /// guessing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anilist_id: Option<i64>,
    /// The id of the entry on AniList's list, needed to delete it there.
    ///
    /// Distinct from `anilist_id`, which identifies the WORK: deleting uses the
    /// list-entry id. Absent until a push or pull supplies it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anilist_entry_id: Option<i64>,
    pub status: ListStatus,
    /// Episodes watched so far.
    pub progress: u32,
    /// Unix seconds when the reader added it. Used for "recently added".
    pub added_at: i64,
    /// Unix seconds of the last LOCAL change. Drives the display order.
    pub updated_at: i64,
    /// Changed locally, not yet accepted by the server.
    #[serde(default)]
    pub dirty: bool,
    /// Removed locally, waiting for the delete to reach AniList.
    ///
    /// A TOMBSTONE, not a hard delete. Dropping the row outright would lose
    /// `anilist_entry_id`, and the next pull would then resurrect the work from
    /// the server -- an entry the reader deleted would silently come back.
    /// Keeping the record with this flag set means the delete can be retried
    /// and, until it succeeds, `reconcile` knows to ignore the server's copy.
    ///
    /// Not rendered: [`MediaListStore::visible`] filters these out, so the UI
    /// behaves as if the row were gone immediately.
    #[serde(default)]
    pub deleted: bool,
}

impl MediaListRecord {
    /// A brand-new local entry: dirty, because nothing has been pushed.
    ///
    /// `anime` is the card the reader clicked, snapshotted so the list page can
    /// render the row without a request.
    pub fn new(
        anime: Anime,
        status: ListStatus,
        now: i64,
    ) -> Self {
        Self {
            provider: anime.provider,
            media_id: anime.id,
            anime,
            anilist_id: None,
            anilist_entry_id: None,
            status,
            progress: 0,
            added_at: now,
            updated_at: now,
            dirty: true,
            deleted: false,
        }
    }

    /// The identity key: provider plus that provider's id.
    ///
    /// Deliberately not the bare id, for the same reason `get_anime` takes a
    /// namespace -- two sources can use the same number.
    pub fn key(&self) -> (ProviderId, i64) {
        (self.provider, self.media_id)
    }

    /// Whether this entry can be pushed to AniList.
    ///
    /// Needs a resolved AniList id AND a token. An entry that cannot be pushed
    /// stays local, which the My List page marks as unsynced rather than
    /// hiding.
    pub fn pushable(&self) -> bool {
        self.anilist_id.is_some()
    }
}

/// The on-disk shape.
///
/// A wrapper rather than a bare array so later versions can add fields without
/// a migration: an unknown key is ignored by serde, and a missing one defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    #[serde(default)]
    entries: Vec<MediaListRecord>,
    /// Set once a pull has succeeded.
    ///
    /// Distinguishes "never synced" from "synced, and the list is genuinely
    /// empty". Without it the first run would show "no entries" for a moment
    /// before the pull filled it.
    #[serde(default)]
    synced: bool,
    /// Unix seconds at which the last pull completed, if any.
    ///
    /// Lets a sync with nothing to push skip the network for a few minutes.
    /// Without it, every write (a status change, a progress tick, a delete)
    /// was followed by a full pull of the whole list -- so changing five
    /// statuses in a row made five 285-entry pulls.
    #[serde(default)]
    last_pull_at: Option<i64>,
}

/// Reads and writes the local media list.
pub struct MediaListStore {
    path: PathBuf,
}

impl MediaListStore {
    /// The store at the default location for this platform.
    ///
    /// Falls back to the temp directory when the OS reports no data directory,
    /// matching `LastPlayedStore`: the app keeps working rather than panicking,
    /// and the list simply does not survive a reboot.
    pub fn new() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("kitsune")
            .join("media-list.json");

        Self { path }
    }

    /// A store at an explicit path, for tests.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Path the store reads and writes, for tests and diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every entry, in the order stored.
    ///
    /// A missing or corrupt file reads as an empty list rather than an error:
    /// the list is recoverable by syncing, and refusing to start over a bad
    /// cache would be worse than starting empty.
    pub fn read_all(&self) -> Vec<MediaListRecord> {
        self.load().entries
    }

    /// The entries the UI should draw: everything except tombstones.
    ///
    /// Every read path uses this rather than `read_all`, so a removal is
    /// immediate on screen while the delete is still queued for AniList.
    pub fn visible(&self) -> Vec<MediaListRecord> {
        self.load()
            .entries
            .into_iter()
            .filter(|entry| !entry.deleted)
            .collect()
    }

    /// Whether a successful pull has ever completed.
    pub fn has_synced(&self) -> bool {
        self.load().synced
    }

    /// Unix seconds at which the last pull completed, if any.
    pub fn last_pull_at(&self) -> Option<i64> {
        self.load().last_pull_at
    }

    /// Mark a pull as completed.
    pub fn mark_synced(&self) -> Result<()> {
        let mut stored = self.load();
        stored.synced = true;
        self.save(&stored)
    }

    /// One entry, by its identity.
    pub fn get(&self, provider: ProviderId, media_id: i64) -> Option<MediaListRecord> {
        self.load()
            .entries
            .into_iter()
            .find(|entry| entry.key() == (provider, media_id))
    }

    /// Insert or replace one entry, keyed on `(provider, media_id)`.
    ///
    /// Replacement, not merge: the caller has already decided the new value.
    pub fn upsert(&self, record: MediaListRecord) -> Result<()> {
        let mut stored = self.load();
        match stored
            .entries
            .iter_mut()
            .find(|entry| entry.key() == record.key())
        {
            Some(existing) => *existing = record,
            None => stored.entries.push(record),
        }
        self.save(&stored)
    }

    /// Insert or replace many entries in ONE write.
    ///
    /// A pull touches the whole list, and writing per entry would be one file
    /// rename per work.
    pub fn upsert_many(&self, records: Vec<MediaListRecord>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut stored = self.load();
        for record in records {
            match stored
                .entries
                .iter_mut()
                .find(|entry| entry.key() == record.key())
            {
                Some(existing) => *existing = record,
                None => stored.entries.push(record),
            }
        }
        self.save(&stored)
    }

    /// Remove one entry.
    ///
    /// A missing entry is success: removing twice is harmless, and the caller
    /// only cares that it is gone.
    pub fn remove(&self, provider: ProviderId, media_id: i64) -> Result<()> {
        let mut stored = self.load();
        let before = stored.entries.len();
        stored
            .entries
            .retain(|entry| entry.key() != (provider, media_id));
        if stored.entries.len() == before {
            return Ok(());
        }
        self.save(&stored)
    }

    /// Replace the whole set in one write, for a pull.
    ///
    /// `incoming` is what the server reports. The merge rule is here rather
    /// than in the caller so it cannot drift:
    ///
    /// - a **dirty** local entry is kept as-is, because the reader's unsent
    ///   change outranks the server's view;
    /// - a **clean** local entry takes the server's value;
    /// - a local entry with no server counterpart was deleted remotely, so it
    ///   is dropped -- unless it is dirty, which means the reader re-added it
    ///   locally and it is waiting to be pushed.
    ///
    /// Returns how many entries were kept dirty, so the caller can report it.
    pub fn reconcile(&self, incoming: Vec<MediaListRecord>) -> Result<usize> {
        let stored = self.load();
        let mut kept_dirty = 0usize;
        let mut merged: Vec<MediaListRecord> = Vec::with_capacity(incoming.len());

        for server_entry in incoming {
            match stored
                .entries
                .iter()
                .find(|local| local.key() == server_entry.key())
            {
                // A tombstone means the reader deleted this and the delete has
                // not reached AniList yet. The server still lists it, so taking
                // the server's copy would RESURRECT a work the reader removed.
                // The tombstone is kept and the server entry dropped.
                Some(local) if local.deleted => {
                    merged.push(local.clone());
                }
                // The reader has unsent work: theirs wins, untouched.
                Some(local) if local.dirty => {
                    kept_dirty += 1;
                    merged.push(local.clone());
                }
                // No local change: the server is authoritative, timestamps
                // included.
                //
                // Deliberately NOT merged with `max`. A clean local entry's
                // stamp is either the server's own or an artefact of an older
                // build that re-stamped every row with `now()` on each pull --
                // and that artefact is NEWER than every real `updatedAt`, so a
                // max would keep the garbage forever. The dirty arm above is
                // what protects an unpushed local edit; once a change has been
                // pushed, AniList's own `updatedAt` is the truth.
                _ => merged.push(server_entry),
            }
        }

        // Local entries the server did not mention. A dirty one is a local
        // addition still waiting to be pushed, so it survives; a clean one was
        // deleted on AniList and goes.
        for local in stored.entries {
            if local.dirty && !merged.iter().any(|entry| entry.key() == local.key()) {
                kept_dirty += 1;
                merged.push(local);
            }
        }

        // Reconcile IS the completion of a pull -- the caller only calls it
        // with data the server just sent -- so the timestamp is stamped here
        // rather than in the engine. That keeps the two from disagreeing about
        // whether the pull that produced this data counts.
        self.save(&Stored {
            entries: merged,
            synced: true,
            last_pull_at: Some(now_seconds()),
        })?;

        Ok(kept_dirty)
    }

    /// Mark an entry removed locally, keeping it as a tombstone.
    ///
    /// The record stays on disk (so `anilist_entry_id` survives for the delete
    /// request) but is hidden from every read. It is dirty, so the next sync
    /// pushes the deletion.
    ///
    /// A missing entry is success: removing something already gone is not an
    /// error, and the reader has no way to act on the difference.
    pub fn mark_deleted(&self, provider: ProviderId, media_id: i64) -> Result<()> {
        let mut stored = self.load();
        let Some(entry) = stored
            .entries
            .iter_mut()
            .find(|entry| entry.key() == (provider, media_id))
        else {
            return Ok(());
        };
        entry.deleted = true;
        entry.dirty = true;
        entry.updated_at = now_seconds();
        self.save(&stored)
    }

    /// Drop tombstones whose delete has reached the server.
    ///
    /// Called after a successful delete push. Until then the tombstone must
    /// stay, because a pull would otherwise re-add the work from AniList's
    /// still-unchanged list.
    pub fn purge_deleted(&self, provider: ProviderId, media_id: i64) -> Result<()> {
        let mut stored = self.load();
        let before = stored.entries.len();
        stored
            .entries
            .retain(|entry| !(entry.key() == (provider, media_id) && entry.deleted));
        if stored.entries.len() == before {
            return Ok(());
        }
        self.save(&stored)
    }

    /// Entries waiting to be pushed, oldest change first.
    ///
    /// Oldest first so a push that is interrupted part-way leaves the newest
    /// changes unsent rather than the oldest -- the reader is more likely to
    /// notice a missing recent edit.
    pub fn dirty(&self) -> Vec<MediaListRecord> {
        let mut entries: Vec<MediaListRecord> = self
            .load()
            .entries
            .into_iter()
            .filter(|entry| entry.dirty)
            .collect();
        entries.sort_by_key(|entry| entry.updated_at);
        entries
    }

    /// Clear the dirty flag after a successful push, recording the server ids.
    ///
    /// Only `dirty`, `anilist_id` and `anilist_entry_id` are touched. The
    /// status and progress are NOT re-read from the server: the push is what
    /// made them true, and a stale read could undo a change made while the
    /// request was in flight.
    pub fn mark_clean(
        &self,
        provider: ProviderId,
        media_id: i64,
        anilist_id: Option<i64>,
        anilist_entry_id: Option<i64>,
    ) -> Result<()> {
        let mut stored = self.load();
        let Some(entry) = stored
            .entries
            .iter_mut()
            .find(|entry| entry.key() == (provider, media_id))
        else {
            return Ok(());
        };
        entry.dirty = false;
        if anilist_id.is_some() {
            entry.anilist_id = anilist_id;
        }
        if anilist_entry_id.is_some() {
            entry.anilist_entry_id = anilist_entry_id;
        }
        self.save(&stored)
    }

    /// Read the stored shape, tolerating anything unreadable.
    fn load(&self) -> Stored {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Stored::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    /// Serialise and write, creating the directory if needed.
    ///
    /// Written to a sibling temp file and renamed into place, so a crash
    /// mid-write cannot leave a half-written list behind -- the same reasoning
    /// as `TokenStore` and `LastPlayedStore`.
    fn save(&self, stored: &Stored) -> Result<()> {
        let dir = self
            .path
            .parent()
            .context("media-list path has no parent directory")?;

        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let json =
            serde_json::to_string_pretty(stored).context("failed to encode the media list")?;

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

impl Default for MediaListStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Unix seconds now, or `0` if the clock is before the epoch.
pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, MediaListStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = MediaListStore::at(dir.path().join("nested").join("media-list.json"));
        (dir, store)
    }

    fn record(provider: ProviderId, id: i64, status: ListStatus) -> MediaListRecord {
        MediaListRecord::new(super::super::test_anime(provider, id), status, 1_000)
    }

    #[test]
    fn a_fresh_store_is_empty_and_never_synced() {
        let (_dir, store) = store();

        assert!(store.read_all().is_empty());
        assert!(
            !store.has_synced(),
            "never synced is not the same as an empty list"
        );
    }

    #[test]
    fn a_record_round_trips() {
        let (_dir, store) = store();
        let mut entry = record(ProviderId::AniList, 21, ListStatus::Current);
        entry.progress = 5;

        store.upsert(entry.clone()).expect("upsert");

        assert_eq!(store.read_all(), vec![entry]);
    }

    /// The identity is `(provider, id)`, so the same NUMBER from two sources is
    /// two different entries -- the whole point of storing the provider.
    #[test]
    fn two_providers_with_the_same_id_are_separate_entries() {
        let (_dir, store) = store();

        store
            .upsert(record(ProviderId::AniList, 12, ListStatus::Current))
            .expect("anilist");
        store
            .upsert(record(ProviderId::Kitsu, 12, ListStatus::Completed))
            .expect("kitsu");

        assert_eq!(store.read_all().len(), 2);
        assert_eq!(
            store.get(ProviderId::Kitsu, 12).unwrap().status,
            ListStatus::Completed
        );
        assert_eq!(
            store.get(ProviderId::AniList, 12).unwrap().status,
            ListStatus::Current
        );
    }

    #[test]
    fn upsert_replaces_rather_than_duplicating() {
        let (_dir, store) = store();
        let mut entry = record(ProviderId::AniList, 21, ListStatus::Current);
        store.upsert(entry.clone()).expect("first");

        entry.status = ListStatus::Completed;
        entry.progress = 12;
        store.upsert(entry.clone()).expect("second");

        assert_eq!(store.read_all(), vec![entry]);
    }

    #[test]
    fn a_new_entry_is_dirty() {
        let entry = record(ProviderId::AniList, 21, ListStatus::Current);

        assert!(entry.dirty, "nothing has been pushed yet");
        assert!(!entry.pushable(), "no AniList id resolved yet");
    }

    #[test]
    fn removing_an_entry_drops_it() {
        let (_dir, store) = store();
        store
            .upsert(record(ProviderId::AniList, 21, ListStatus::Current))
            .expect("upsert");

        store.remove(ProviderId::AniList, 21).expect("remove");

        assert!(store.read_all().is_empty());
    }

    #[test]
    fn removing_a_missing_entry_is_success() {
        let (_dir, store) = store();

        assert!(store.remove(ProviderId::AniList, 999).is_ok());
    }

    #[test]
    fn dirty_returns_only_unpushed_entries_oldest_first() {
        let (_dir, store) = store();
        let mut newer = record(ProviderId::AniList, 1, ListStatus::Current);
        newer.updated_at = 2_000;
        let mut older = record(ProviderId::AniList, 2, ListStatus::Current);
        older.updated_at = 1_000;

        store.upsert(newer).expect("newer");
        store.upsert(older).expect("older");

        let dirty = store.dirty();
        assert_eq!(dirty.len(), 2);
        assert_eq!(dirty[0].media_id, 2, "oldest change first");
        assert_eq!(dirty[1].media_id, 1);
    }

    #[test]
    fn mark_clean_clears_the_flag_and_records_server_ids() {
        let (_dir, store) = store();
        store
            .upsert(record(ProviderId::AniList, 21, ListStatus::Current))
            .expect("upsert");

        store
            .mark_clean(ProviderId::AniList, 21, Some(21), Some(555))
            .expect("mark clean");

        let entry = store.get(ProviderId::AniList, 21).unwrap();
        assert!(!entry.dirty);
        assert_eq!(entry.anilist_id, Some(21));
        assert_eq!(entry.anilist_entry_id, Some(555));
        assert!(store.dirty().is_empty());
    }

    /// A pushed entry is NOT re-read from the server: the push is what made its
    /// values true, and a stale read could undo a change made in flight.
    #[test]
    fn mark_clean_leaves_status_and_progress_alone() {
        let (_dir, store) = store();
        let mut entry = record(ProviderId::AniList, 21, ListStatus::Current);
        entry.progress = 7;
        store.upsert(entry).expect("upsert");

        store
            .mark_clean(ProviderId::AniList, 21, Some(21), Some(1))
            .expect("mark clean");

        let entry = store.get(ProviderId::AniList, 21).unwrap();
        assert_eq!(entry.status, ListStatus::Current);
        assert_eq!(entry.progress, 7);
    }

    #[test]
    fn a_corrupt_file_reads_as_an_empty_list() {
        let (_dir, store) = store();
        std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        std::fs::write(store.path(), "{ not json").unwrap();

        assert!(store.read_all().is_empty());
    }

    // --- reconcile -------------------------------------------------------

    #[test]
    fn reconcile_takes_the_server_value_for_a_clean_entry() {
        let (_dir, store) = store();
        let mut local = record(ProviderId::AniList, 21, ListStatus::Current);
        local.dirty = false;
        store.upsert(local).expect("local");

        let mut server = record(ProviderId::AniList, 21, ListStatus::Completed);
        server.progress = 12;
        server.dirty = false;
        store.reconcile(vec![server]).expect("reconcile");

        let entry = store.get(ProviderId::AniList, 21).unwrap();
        assert_eq!(entry.status, ListStatus::Completed);
        assert_eq!(entry.progress, 12);
    }

    /// The rule the user chose: a local edit that has not been pushed outranks
    /// whatever the server reports.
    #[test]
    fn reconcile_keeps_a_dirty_entry_over_the_server() {
        let (_dir, store) = store();
        let mut local = record(ProviderId::AniList, 21, ListStatus::Current);
        local.dirty = true;
        store.upsert(local.clone()).expect("local");

        let mut server = record(ProviderId::AniList, 21, ListStatus::Completed);
        server.dirty = false;
        let kept = store.reconcile(vec![server]).expect("reconcile");

        assert_eq!(kept, 1);
        let entry = store.get(ProviderId::AniList, 21).unwrap();
        assert_eq!(entry.status, ListStatus::Current, "local edit survives");
        assert!(entry.dirty);
    }

    /// A CLEAN entry takes the server's `updated_at` outright.
    ///
    /// This is the fix for the collapsed-timestamp bug: an older build stamped
    /// every row with `now()` on each pull, so all 273 entries shared one
    /// value -- newer than any real `updatedAt`. Merging with `max` preserved
    /// that garbage, and Continue Watching (which sorts and truncates on this
    /// field) could not show what the reader had just watched. The dirty arm is
    /// what protects an unpushed edit; a clean entry has nothing to protect.
    #[test]
    fn reconcile_takes_the_servers_updated_at_for_a_clean_entry() {
        let (_dir, store) = store();
        let mut local = record(ProviderId::AniList, 21, ListStatus::Current);
        local.updated_at = 9_000;
        local.dirty = false;
        store.upsert(local).expect("local");

        let mut server = record(ProviderId::AniList, 21, ListStatus::Completed);
        server.updated_at = 1_000;
        server.dirty = false;
        store.reconcile(vec![server]).expect("reconcile");

        let entry = store.get(ProviderId::AniList, 21).unwrap();
        assert_eq!(entry.status, ListStatus::Completed, "server owns status");
        assert_eq!(
            entry.updated_at, 1_000,
            "a clean entry takes the server's stamp, not a stale local one"
        );
    }

    #[test]
    fn reconcile_keeps_a_dirty_entry_the_server_omitted() {
        let (_dir, store) = store();
        let mut local = record(ProviderId::AniList, 21, ListStatus::Current);
        local.dirty = true;
        store.upsert(local).expect("local");

        // The server says nothing about it -- normally "deleted remotely", but
        // a dirty entry is a local re-add waiting to be pushed.
        let kept = store.reconcile(Vec::new()).expect("reconcile");

        assert_eq!(kept, 1);
        assert_eq!(store.read_all().len(), 1);
    }

    #[test]
    fn reconcile_drops_a_clean_entry_the_server_omitted() {
        let (_dir, store) = store();
        let mut local = record(ProviderId::AniList, 21, ListStatus::Current);
        local.dirty = false;
        store.upsert(local).expect("local");

        store.reconcile(Vec::new()).expect("reconcile");

        assert!(
            store.read_all().is_empty(),
            "a clean entry missing remotely was deleted there"
        );
    }

    #[test]
    fn reconcile_adds_entries_only_the_server_knows() {
        let (_dir, store) = store();

        let server = record(ProviderId::AniList, 7, ListStatus::Planning);
        store.reconcile(vec![server]).expect("reconcile");

        assert_eq!(store.read_all().len(), 1);
        assert_eq!(store.get(ProviderId::AniList, 7).unwrap().status, ListStatus::Planning);
    }

    #[test]
    fn reconcile_marks_the_store_synced() {
        let (_dir, store) = store();
        assert!(!store.has_synced());

        store.reconcile(Vec::new()).expect("reconcile");

        assert!(store.has_synced());
    }

    #[test]
    fn reconcile_stamps_the_pull_time() {
        let (_dir, store) = store();
        assert_eq!(store.last_pull_at(), None, "no pull has run yet");

        let before = now_seconds();
        store.reconcile(Vec::new()).expect("reconcile");

        let stamped = store.last_pull_at().expect("a completed pull is stamped");
        assert!(
            stamped >= before,
            "the stamp is the pull's own clock, not a stale one"
        );
    }

    #[test]
    fn writing_creates_missing_directories() {
        let (_dir, store) = store();

        store
            .upsert(record(ProviderId::AniList, 21, ListStatus::Current))
            .expect("upsert should create parents");

        assert!(store.path().exists());
    }
}
#[cfg(test)]
mod tombstone_tests {
    use super::*;

    fn store() -> (tempfile::TempDir, MediaListStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = MediaListStore::at(dir.path().join("nested").join("media-list.json"));
        (dir, store)
    }

    fn record(id: i64) -> MediaListRecord {
        MediaListRecord::new(
            super::super::test_anime(ProviderId::AniList, id),
            ListStatus::Current,
            1_000,
        )
    }

    /// A removal must hide the row immediately, without waiting for the push.
    #[test]
    fn a_tombstone_disappears_from_visible() {
        let (_dir, store) = store();
        store.upsert(record(21)).expect("upsert");

        store
            .mark_deleted(ProviderId::AniList, 21)
            .expect("mark deleted");

        assert_eq!(store.visible().len(), 0, "the reader sees it gone at once");
        // Still on disk, because the delete request needs `anilist_entry_id`.
        assert_eq!(store.read_all().len(), 1);
        assert!(store.read_all()[0].dirty);
    }

    /// THE RESURRECTION BUG: a pull must not re-add a work the reader deleted
    /// while the delete was still queued for AniList.
    #[test]
    fn reconcile_does_not_resurrect_a_tombstone() {
        let (_dir, store) = store();
        let mut entry = record(21);
        entry.anilist_id = Some(21);
        entry.anilist_entry_id = Some(99);
        store.upsert(entry).expect("upsert");
        store
            .mark_deleted(ProviderId::AniList, 21)
            .expect("mark deleted");

        // The server still lists it: the delete has not reached AniList yet.
        let mut server = record(21);
        server.dirty = false;

        store.reconcile(vec![server]).expect("reconcile");

        assert_eq!(store.visible().len(), 0, "must stay deleted");
        assert_eq!(
            store.read_all().len(),
            1,
            "the tombstone survives so the delete can be retried"
        );
        assert!(store.read_all()[0].deleted);
    }

    /// After the delete reaches the server the record is dropped for good.
    #[test]
    fn purging_drops_the_tombstone() {
        let (_dir, store) = store();
        store.upsert(record(21)).expect("upsert");
        store
            .mark_deleted(ProviderId::AniList, 21)
            .expect("mark deleted");

        store
            .purge_deleted(ProviderId::AniList, 21)
            .expect("purge");

        assert!(store.read_all().is_empty(), "gone from disk entirely");
    }

    /// A tombstone must not be re-added by a pull even when the server entry is
    /// clean -- the exact case above, asserted through the public read path too.
    #[test]
    fn a_deleted_entry_stays_out_of_visible_after_upsert_of_others() {
        let (_dir, store) = store();
        store.upsert(record(21)).expect("upsert");
        store
            .mark_deleted(ProviderId::AniList, 21)
            .expect("mark deleted");
        store.upsert(record(22)).expect("upsert another");

        let visible = store.visible();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].media_id, 22);
    }
}

#[cfg(test)]
mod tombstone_branch_tests {
    use super::*;

    fn store() -> (tempfile::TempDir, MediaListStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = MediaListStore::at(dir.path().join("nested").join("media-list.json"));
        (dir, store)
    }

    /// The `deleted` arm in `reconcile` is NOT the same guard as `dirty`.
    ///
    /// `mark_deleted` sets both flags, so a tombstone made the normal way is
    /// preserved by the dirty arm even if the deleted arm is removed -- which
    /// makes a naive test vacuous (verified by sabotage). This writes a
    /// tombstone with `dirty: false` directly, so ONLY the deleted arm can
    /// save it.
    ///
    /// That state is unreachable through the public API today, which is exactly
    /// why it is worth pinning: the two flags mean different things, and a
    /// future change to `mark_deleted` must not quietly un-guard the merge.
    #[test]
    fn a_clean_tombstone_is_still_not_resurrected() {
        let (_dir, store) = store();

        let mut tombstone = MediaListRecord::new(
            super::super::test_anime(ProviderId::AniList, 21),
            ListStatus::Current,
            1_000,
        );
        tombstone.deleted = true;
        tombstone.dirty = false;
        store.upsert(tombstone).expect("upsert");

        let mut server = MediaListRecord::new(
            super::super::test_anime(ProviderId::AniList, 21),
            ListStatus::Current,
            1_000,
        );
        server.dirty = false;

        store.reconcile(vec![server]).expect("reconcile");

        assert_eq!(
            store.visible().len(),
            0,
            "a deleted entry must never come back from a pull"
        );
        assert_eq!(store.read_all().len(), 1);
        assert!(store.read_all()[0].deleted);
    }
}
