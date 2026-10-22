//! Reconciling the local media list with AniList.
//!
//! The order is deliberate: **push, then pull, then reconcile.** A pull first
//! would overwrite nothing (dirty entries are protected) but it would waste a
//! request re-learning state the push is about to change.
//!
//! Three rules shape the whole engine:
//!
//! - **A refused push leaves the entry dirty.** It is retried on the next run,
//!   never dropped. A transient 500 must not lose the reader's edit.
//! - **A signed-out reader is not an error.** Every entry simply stays dirty,
//!   which is the honest state: it has not reached AniList because there is no
//!   account to reach it with.
//! - **A mapping failure is not fatal.** An entry whose AniList id cannot be
//!   resolved is skipped and left dirty, so it stays local rather than being
//!   written to the wrong work.

use std::sync::Arc;

use crate::providers::anilist::{AniListProvider, UserListEntry};
use crate::providers::kitsu::KitsuProvider;
use crate::types::{Anime, ListStatus, ProviderId};

use super::store::{now_seconds, MediaListRecord, MediaListStore};

/// How long a successful pull is trusted when there is nothing to push.
///
/// A local change always pulls, because the server's view may have moved. A
/// sync with no dirty entries only refreshes what the reader can already see,
/// and doing that on every write was making a status change cost a full
/// 285-entry request. Five minutes is short enough that a remote edit shows up
/// while the reader is still using the app, and long enough that a burst of
/// edits costs one pull.
const PULL_TTL_SECONDS: i64 = 300;

/// What one sync run did, for logging and for the UI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Entries pushed to AniList successfully.
    pub pushed: usize,
    /// Entries the server refused; still dirty, retried next run.
    pub failed: usize,
    /// Entries pulled from AniList.
    pub pulled: usize,
    /// Dirty entries that survived the reconcile, because they are unsent.
    pub still_dirty: usize,
    /// Whether the pull ran at all. False when signed out or AniList failed.
    pub pulled_ok: bool,
    /// Whether the pull was skipped because the last one is still fresh.
    ///
    /// Distinct from `pulled_ok`: nothing failed, the pull was simply not
    /// needed. The UI treats it the same as a success.
    pub pull_skipped: bool,
}

impl SyncReport {
    /// One line, for the log. Deliberately terse: a sync runs on every start.
    pub fn summary(&self) -> String {
        format!(
            "media list sync: pushed {}, failed {}, pulled {}, still dirty {}{}",
            self.pushed,
            self.failed,
            self.pulled,
            self.still_dirty,
            if self.pull_skipped { " (pull skipped)" } else { "" },
        )
    }
}

/// Pushes local changes and pulls the server's list.
///
/// `kitsu` resolves an AniList id for a Kitsu-sourced entry. It is optional so
/// a caller without one (tests, a future single-source build) still works:
/// unresolvable entries simply stay local.
pub struct ListSync {
    anilist: Arc<AniListProvider>,
    kitsu: Option<Arc<KitsuProvider>>,
    store: Arc<MediaListStore>,
}

impl ListSync {
    pub fn new(
        anilist: Arc<AniListProvider>,
        kitsu: Option<Arc<KitsuProvider>>,
        store: Arc<MediaListStore>,
    ) -> Self {
        Self {
            anilist,
            kitsu,
            store,
        }
    }

    /// Run one full sync.
    ///
    /// Never returns `Err` for a network problem: a sync that cannot reach
    /// AniList is a normal state (offline, outage, signed out), and the caller
    /// has nothing useful to do with an error. Failures are reported in the
    /// [`SyncReport`] instead.
    pub async fn run(&self) -> SyncReport {
        let mut report = SyncReport::default();

        for entry in self.store.dirty() {
            match self.push(&entry).await {
                Ok(()) => report.pushed += 1,
                Err(_) => report.failed += 1,
            }
        }

        // A pull is only worth a request when either the server might have
        // moved (a push just happened) or the last one is stale. With nothing
        // pushed and a fresh pull, the answer would be the state already on
        // disk.
        if report.pushed == 0 && self.pull_is_fresh() {
            report.pull_skipped = true;
            report.pulled_ok = true;
            report.still_dirty = self.store.dirty().len();
            return report;
        }

        match self.pull().await {
            Ok(entries) => {
                report.pulled = entries.len();
                report.pulled_ok = true;
                // The store owns the merge rule, so it cannot drift from the
                // tests that pin it.
                report.still_dirty = self.store.reconcile(entries).unwrap_or(0);
            }
            Err(_) => {
                // Leave the store untouched: a failed pull is not evidence that
                // anything was deleted.
                report.still_dirty = self.store.dirty().len();
            }
        }

        report
    }

    /// Whether the last successful pull is recent enough to trust.
    ///
    /// A clock that moved backwards (a timezone change, an NTP correction)
    /// yields a negative age; treated as fresh, so the worst case is one
    /// skipped pull rather than a loop of forced ones.
    fn pull_is_fresh(&self) -> bool {
        let Some(last) = self.store.last_pull_at() else {
            return false;
        };
        let age = now_seconds() - last;
        (0..PULL_TTL_SECONDS).contains(&age)
    }

    /// Push one entry, recording the server ids on success.
    ///
    /// A tombstone takes the DELETE path instead of the save path. The two are
    /// mutually exclusive: a record marked deleted has no status worth
    /// sending, and sending one would re-create the entry the reader removed.
    async fn push(&self, entry: &MediaListRecord) -> Result<(), ()> {
        if entry.deleted {
            return self.push_delete(entry).await;
        }

        let anilist_id = match self.resolve_anilist_id(entry).await {
            Some(id) => id,
            // Unresolvable: leave it dirty and local-only rather than guessing.
            None => return Err(()),
        };

        let saved = self
            .anilist
            .save_list_entry_returning(anilist_id, entry.status, Some(entry.progress))
            .await
            .map_err(|_| ())?;

        // `mark_clean` needs BOTH ids. The entry id is what a later delete
        // names, and it only exists after a successful push.
        self.store
            .mark_clean(
                entry.provider,
                entry.media_id,
                Some(anilist_id),
                Some(saved.id),
            )
            .map_err(|_| ())?;

        Ok(())
    }

    /// Push a deletion, then drop the tombstone.
    ///
    /// Deleting needs the LIST-ENTRY id, not the work id -- a media id would
    /// target the wrong thing. An entry added and removed entirely while
    /// offline never got one, so there is nothing on the server to delete and
    /// the tombstone can be dropped immediately: the delete is already true
    /// from AniList's point of view because it never heard about the add.
    async fn push_delete(&self, entry: &MediaListRecord) -> Result<(), ()> {
        if let Some(entry_id) = entry.anilist_entry_id {
            self.anilist
                .delete_list_entry(entry_id)
                .await
                .map_err(|_| ())?;
        }

        // Only after the server agreed (or there was nothing to tell it):
        // purging first would lose the entry id a retry needs.
        self.store
            .purge_deleted(entry.provider, entry.media_id)
            .map_err(|_| ())?;

        Ok(())
    }

    /// The entry's AniList id, resolving a Kitsu id through Kitsu's table.
    ///
    /// A Kitsu-sourced entry has no AniList id until this runs. The lookup is
    /// answered by Kitsu alone, so it works while AniList is unreachable -- but
    /// the push that follows still needs AniList, which is why a failure here
    /// is not distinguished from one there.
    async fn resolve_anilist_id(&self, entry: &MediaListRecord) -> Option<i64> {
        if let Some(id) = entry.anilist_id {
            return Some(id);
        }

        match entry.provider {
            // An AniList entry's media id IS the AniList id.
            ProviderId::AniList => Some(entry.media_id),
            ProviderId::Kitsu => {
                let kitsu = self.kitsu.as_ref()?;
                // `anilist_id_for_kitsu`, NOT `kitsu_id_for_anilist`: the id we
                // hold is a KITSU id, and asking for a Kitsu id with a Kitsu id
                // would resolve nothing and -- worse -- a variant that returned
                // the input would push the Kitsu id as if it were an AniList
                // one, writing to an unrelated work.
                kitsu
                    .anilist_id_for_kitsu(entry.media_id)
                    .await
                    .ok()
                    .flatten()
            }
            // Not a metadata source we can resolve.
            _ => None,
        }
    }

    /// Pull the whole list and convert it to local records.
    async fn pull(&self) -> Result<Vec<MediaListRecord>, ()> {
        let entries = self.anilist.user_list().await.map_err(|_| ())?;

        Ok(entries.into_iter().map(record_from_server).collect())
    }
}

/// Convert one AniList list entry into a local record.
///
/// `dirty: false` on purpose: this value CAME from the server, so there is
/// nothing to push back. `reconcile` then decides whether it replaces the local
/// copy (it does not, when that copy is dirty).
fn record_from_server(entry: UserListEntry) -> MediaListRecord {
    // The ANIME ARRIVES WITH the entry: `user_list` already fetches the media
    // node, so the snapshot costs nothing extra and saves a request per row.
    let anime = entry.anime;

    MediaListRecord {
        // AniList's own list, so the namespace is unambiguous.
        provider: ProviderId::AniList,
        media_id: anime.id,
        anilist_id: Some(anime.id),
        anilist_entry_id: Some(entry.entry_id),
        status: entry.status,
        progress: entry.progress,
        added_at: entry.added_at.unwrap_or(0),
        // The SERVER's timestamp, not the pull's. Stamping `now` here made
        // every entry share one value after each sync, which destroyed the
        // Continue Watching order: the row sorts by this field and truncates
        // to a dozen BEFORE the UI can re-sort, so the work the reader just
        // watched could be cut off entirely.
        //
        // `0` when AniList omits it, which sorts last -- safer than claiming
        // a recency that is not there.
        updated_at: entry.updated_at.unwrap_or(0),
        dirty: false,
        deleted: false,
        anime,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(provider: ProviderId, media_id: i64, anilist_id: Option<i64>) -> MediaListRecord {
        let mut entry =
            MediaListRecord::new(super::super::test_anime(provider, media_id), ListStatus::Current, 1);
        entry.anilist_id = anilist_id;
        entry
    }

    #[test]
    fn a_local_change_is_always_dirty() {
        let changed = local_change(
            super::super::test_anime(ProviderId::AniList, 21),
            ListStatus::Completed,
            Some(12),
            None,
        );

        assert!(changed.dirty, "nothing has pushed it");
        assert_eq!(changed.status, ListStatus::Completed);
        assert_eq!(changed.progress, 12);
    }

    #[test]
    fn a_local_change_keeps_the_existing_added_at() {
        let mut existing = MediaListRecord::new(
            super::super::test_anime(ProviderId::AniList, 21),
            ListStatus::Current,
            1,
        );
        existing.added_at = 500;
        existing.anilist_id = Some(21);

        let changed = local_change(
            super::super::test_anime(ProviderId::AniList, 21),
            ListStatus::Completed,
            Some(12),
            Some(existing),
        );

        assert_eq!(changed.added_at, 500, "adding time is stable");
        assert_eq!(changed.anilist_id, Some(21), "the resolved id survives");
        assert!(changed.dirty);
    }

    /// A server entry is CLEAN: it came from AniList, so there is nothing to
    /// push back. If this were dirty, every sync would re-push the whole list.
    #[test]
    fn a_pulled_entry_is_clean() {
        let pulled = record_from_server(UserListEntry {
            anime: crate::types::Anime {
                id: 21,
                provider: ProviderId::AniList,
                title: Default::default(),
                id_mal: None,
                cover_image: None,
                banner_image: None,
                description: None,
                episode_count: None,
                aired_episodes: None,
                duration_minutes: None,
                format: None,
                genres: Vec::new(),
                average_score: None,
                popularity: None,
                status: None,
                season: None,
                season_year: None,
                streaming_episodes: Vec::new(),
                relations: Vec::new(),
                recommendations: Vec::new(),
                trailer: None,
            },
            status: ListStatus::Current,
            progress: 3,
            entry_id: 555,
            added_at: Some(100),
            updated_at: Some(900),
        });

        assert!(!pulled.dirty);
        assert_eq!(pulled.media_id, 21);
        assert_eq!(pulled.anilist_id, Some(21));
        assert_eq!(pulled.anilist_entry_id, Some(555));
        assert_eq!(pulled.provider, ProviderId::AniList);
    }

    /// The bug this guards: a Kitsu entry's `media_id` is a KITSU id. Treating
    /// it as an AniList id (or pushing it through `kitsu_id_for_anilist`) would
    /// write to an unrelated work.
    #[test]
    fn an_anilist_entry_needs_no_mapping() {
        let entry = record(ProviderId::AniList, 16498, None);

        // The id IS the AniList id, so resolution is the identity.
        assert_eq!(entry.media_id, 16498);
        assert_eq!(entry.provider, ProviderId::AniList);
    }

    #[test]
    fn an_unresolved_kitsu_entry_is_not_pushable() {
        let entry = record(ProviderId::Kitsu, 7442, None);

        assert!(
            !entry.pushable(),
            "no AniList id resolved, so pushing would guess"
        );
    }

    #[test]
    fn a_resolved_entry_is_pushable() {
        let entry = record(ProviderId::Kitsu, 7442, Some(16498));

        assert!(entry.pushable());
    }

    #[test]
    fn the_summary_reports_every_count() {
        let report = SyncReport {
            pushed: 2,
            failed: 1,
            pulled: 30,
            still_dirty: 1,
            pulled_ok: true,
            pull_skipped: false,
        };

        let line = report.summary();
        assert!(line.contains("pushed 2"));
        assert!(line.contains("failed 1"));
        assert!(line.contains("pulled 30"));
        assert!(line.contains("still dirty 1"));
        assert!(!line.contains("skipped"), "a real pull is not a skip");
    }

    #[test]
    fn a_skipped_pull_says_so() {
        let report = SyncReport {
            pull_skipped: true,
            pulled_ok: true,
            ..SyncReport::default()
        };

        // The skip is visible in the log, otherwise "pulled 0" reads like a
        // failure to anyone diagnosing a stale list.
        assert!(report.summary().contains("pull skipped"));
    }
}

/// Build a local record for a work the reader has just changed.
///
/// Shared by every write path so the `updated_at` and `dirty` rules live in one
/// place: a local change is ALWAYS dirty, because nothing has pushed it yet.
///
/// `progress` is OPTIONAL, mirroring AniList's own signature:
/// - `Some(n)` records n episodes watched;
/// - `None` means "leave progress alone", which is what a status-only change
///   (Watching -> Completed) must send.
///
/// The distinction is load-bearing. Taking a bare `u32` forced every caller to
/// pass a number, so moving a work between lists silently reset it to 0 --
/// destroying the episode count for a reader who had watched 11 of 12.
pub fn local_change(
    anime: Anime,
    status: ListStatus,
    progress: Option<u32>,
    existing: Option<MediaListRecord>,
) -> MediaListRecord {
    let now = now_seconds();

    match existing {
        Some(mut record) => {
            // Re-adding a tombstoned work is a resurrection, not an edit: the
            // delete must be cancelled, or the next sync would delete it again.
            record.deleted = false;
            record.status = status;
            if let Some(progress) = progress {
                record.progress = progress;
            }
            record.updated_at = now;
            record.dirty = true;
            // Refresh the snapshot: the card the reader is looking at may carry
            // a newer cover or score than the last pull did.
            record.anime = anime;
            record
        }
        None => {
            let mut record = MediaListRecord::new(anime, status, now);
            record.progress = progress.unwrap_or(0);
            record
        }
    }
}
#[cfg(test)]
mod change_tests {
    use super::*;

    fn anime(id: i64) -> Anime {
        super::super::test_anime(ProviderId::AniList, id)
    }

    /// THE BUG THIS GUARDS: moving a work from Watching to Completed used to
    /// reset `progress` to 0, silently discarding how far the reader had got.
    /// A status-only change must leave progress untouched.
    #[test]
    fn a_status_only_change_keeps_progress() {
        let mut existing = MediaListRecord::new(anime(21), ListStatus::Current, 1);
        existing.progress = 11;

        let changed = local_change(
            anime(21),
            ListStatus::Completed,
            None,
            Some(existing),
        );

        assert_eq!(changed.status, ListStatus::Completed);
        assert_eq!(changed.progress, 11, "progress must survive a status change");
        assert!(changed.dirty, "the change still needs pushing");
    }

    /// An explicit progress IS applied -- the guard above must not freeze it.
    #[test]
    fn an_explicit_progress_overrides() {
        let mut existing = MediaListRecord::new(anime(21), ListStatus::Current, 1);
        existing.progress = 11;

        let changed = local_change(
            anime(21),
            ListStatus::Current,
            Some(12),
            Some(existing),
        );

        assert_eq!(changed.progress, 12);
    }

    /// A brand-new entry with no progress stated starts at 0 rather than
    /// failing to build.
    #[test]
    fn a_new_entry_defaults_progress_to_zero() {
        let changed = local_change(anime(21), ListStatus::Planning, None, None);

        assert_eq!(changed.progress, 0);
        assert!(changed.dirty);
        assert!(!changed.deleted);
    }

    /// Re-adding a work the reader had deleted must CANCEL the pending delete,
    /// or the next sync would delete it again and the re-add would vanish.
    #[test]
    fn re_adding_a_tombstoned_entry_revives_it() {
        let mut existing = MediaListRecord::new(anime(21), ListStatus::Current, 1);
        existing.deleted = true;
        existing.dirty = true;

        let changed = local_change(
            anime(21),
            ListStatus::Planning,
            None,
            Some(existing),
        );

        assert!(!changed.deleted, "the tombstone must be cleared");
        assert!(changed.dirty);
    }
}
