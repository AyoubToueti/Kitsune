//! A TTL cache in front of the indexer fan-out and the probe pass.
//!
//! Every release search funnels through [`search_all`], so caching here covers
//! both frontends without each needing to know about it. The reader reopens the
//! same episode constantly -- close the modal, click the same episode again --
//! and each open was a fresh round trip to every indexer.
//!
//! Two behaviours beyond a plain TTL, both inherited from
//! [`crate::providers::cache::TtlCache`]:
//!   - in-flight de-duplication: two opens of the same episode at once share
//!     one search rather than racing two;
//!   - eviction at a fixed cap, so unbounded browse keys cannot leak.
//!
//! An EMPTY result is deliberately NOT cached: the reader must be able to hit
//! Retry and do real work. The empty list is removed right after a successful
//! load, so the next open searches again.

use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use super::probe_rank::ProbeOutcome;
use super::traits::IndexerError;
use super::{search_all, Indexer, ReleaseRequest};
use crate::providers::cache::TtlCache;
use crate::types::{Release, ReleasePreference};

/// One search's identity, as the cache sees it.
///
/// Titles are joined with a separator that cannot appear in a title, so
/// `["A B", "C"]` and `["A", "B C"]` cannot collide. The preference is part of
/// the key because it steers the ranking inside `search_all`; without it a
/// settings change would be masked for the whole TTL.
fn release_key(request: &ReleaseRequest, preference: &ReleasePreference) -> String {
    let titles = request.titles.join("\u{1f}");
    let mode = serde_json::to_string(&request.mode).unwrap_or_default();
    let preference = serde_json::to_string(preference).unwrap_or_default();
    format!(
        "{titles}\u{1f}{:?}\u{1f}{:?}\u{1f}{mode}\u{1f}{preference}",
        request.episode, request.absolute_episode
    )
}

/// A stable key for one release list, for the probe cache.
///
/// Probes are index-aligned with the list that produced them, so the identity
/// of a batch is the identity of its members. Hashing the magnet and the title
/// covers both "same torrent" and "same order", which is all the index
/// alignment needs.
fn probe_key(releases: &[Release]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    releases.len().hash(&mut hasher);
    for release in releases {
        release.magnet_uri.hash(&mut hasher);
        release.title.hash(&mut hasher);
    }
    format!("probes:{:x}", hasher.finish())
}

/// The release search cache, plus the probe verdicts for each list.
///
/// One instance serves the whole process; both frontends hold it in state.
/// The two caches share a key scheme for searches but the probe cache is keyed
/// by the list itself, since a probe call never sees the request that built it.
pub struct ReleaseCache {
    releases: TtlCache<Vec<Release>>,
    probes: TtlCache<Vec<ProbeOutcome>>,
}

impl ReleaseCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self {
            releases: TtlCache::new(),
            probes: TtlCache::new(),
        }
    }

    /// Search every indexer, or serve a recent identical search.
    ///
    /// `ttl` is how long a hit stays fresh. `force` drops any entry first, so
    /// the call always reaches the indexers -- the Retry button passes it.
    ///
    /// Note that a shorter `ttl` alone does NOT invalidate: the expiry stored
    /// with the value is what governs, so a fresh entry is served whatever TTL
    /// the caller now asks for. That is what `force` is for.
    ///
    /// Never fails in practice -- `search_all` drops per-indexer errors and
    /// returns whatever worked -- but the `Result` keeps the signature honest
    /// with [`TtlCache::get_or_load`], which is fallible.
    pub async fn search(
        &self,
        indexers: &[Arc<dyn Indexer>],
        request: &ReleaseRequest,
        preference: &ReleasePreference,
        ttl: Duration,
        force: bool,
    ) -> Result<Vec<Release>, IndexerError> {
        let key = release_key(request, preference);
        if force {
            self.releases.invalidate(&key);
        }
        let found = self
            .releases
            .get_or_load(&key, ttl, || async {
                // An indexer that fails contributes nothing rather than
                // failing the search; see `search_all`.
                Ok::<Vec<Release>, IndexerError>(search_all(indexers, request, preference).await)
            })
            .await?;

        // An empty list must not stick for the TTL: Retry is the reader's only
        // way out of it, and it has to do real work.
        if found.is_empty() {
            self.releases.invalidate(&key);
        }

        Ok(found)
    }

    /// The cached verdicts for `releases`, when a recent batch matches.
    ///
    /// `None` means the caller must probe. A returned list is index-aligned
    /// with `releases`, exactly as a live batch would be.
    pub fn cached_probes(&self, releases: &[Release]) -> Option<Vec<ProbeOutcome>> {
        self.probes.get(&probe_key(releases))
    }

    /// Remember a completed probe batch for `releases`.
    ///
    /// Only ever called with the whole batch: a partial one would leave
    /// releases whose badge never arrives, since `probeDone` is what stops the
    /// UI waiting.
    pub fn store_probes(&self, releases: &[Release], outcomes: Vec<ProbeOutcome>) {
        self.probes
            .store(&probe_key(releases), outcomes, super::super::providers::cache::ttl::RELEASES);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::query::SearchMode;
    use crate::types::{ProviderId, ReleaseSource, Resolution};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// An indexer that counts how many queries reached it.
    struct CountingIndexer {
        calls: Arc<AtomicUsize>,
        releases: Vec<Release>,
    }

    #[async_trait::async_trait]
    impl Indexer for CountingIndexer {
        fn name(&self) -> &str {
            "counting"
        }

        async fn search(&self, _query: &str) -> Result<Vec<Release>, IndexerError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.releases.clone())
        }
    }

    fn release_named(title: &str) -> Release {
        Release {
            title: title.to_string(),
            indexer: ProviderId::Nyaa,
            magnet_uri: format!("magnet:?xt=urn:btih:{}", "a".repeat(40)),
            torrent_url: None,
            info_hash: None,
            size_bytes: None,
            seeders: Some(10),
            leechers: None,
            published_at: None,
            resolution: Resolution::R1080p,
            source: ReleaseSource::WebDl,
            remux: false,
            trusted: false,
            parsed: crate::indexer::parse::parse_release(title),
            score: 0,
        }
    }

    fn request(episode: u32) -> ReleaseRequest {
        ReleaseRequest {
            titles: vec!["Show".into()],
            episode: Some(episode),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        }
    }

    fn indexer(calls: &Arc<AtomicUsize>) -> CountingIndexer {
        CountingIndexer {
            calls: Arc::clone(calls),
            releases: vec![release_named("[G] Show S01E05 [1080p]")],
        }
    }

    #[tokio::test]
    async fn a_repeat_search_is_served_from_cache() {
        let calls = Arc::new(AtomicUsize::new(0));
        let indexers: Vec<Arc<dyn Indexer>> = vec![Arc::new(indexer(&calls))];
        let cache = ReleaseCache::new();
        let request = request(5);
        let preference = ReleasePreference::default();

        cache
            .search(&indexers, &request, &preference, Duration::from_secs(60), false)
            .await
            .unwrap();
        let after_first = calls.load(Ordering::SeqCst);
        assert!(after_first > 0, "the first search must reach the indexer");

        cache
            .search(&indexers, &request, &preference, Duration::from_secs(60), false)
            .await
            .unwrap();

        assert_eq!(
            calls.load(Ordering::SeqCst),
            after_first,
            "a repeated search must not reach the indexer"
        );
    }

    #[tokio::test]
    async fn a_different_episode_is_a_miss() {
        let calls = Arc::new(AtomicUsize::new(0));
        let indexers: Vec<Arc<dyn Indexer>> = vec![Arc::new(indexer(&calls))];
        let cache = ReleaseCache::new();
        let preference = ReleasePreference::default();

        cache
            .search(&indexers, &request(5), &preference, Duration::from_secs(60), false)
            .await
            .unwrap();
        let after_first = calls.load(Ordering::SeqCst);

        cache
            .search(&indexers, &request(6), &preference, Duration::from_secs(60), false)
            .await
            .unwrap();

        assert!(
            calls.load(Ordering::SeqCst) > after_first,
            "a different episode must search again"
        );
    }

    #[tokio::test]
    async fn an_empty_result_is_not_cached() {
        let calls = Arc::new(AtomicUsize::new(0));
        let indexers: Vec<Arc<dyn Indexer>> = vec![Arc::new(CountingIndexer {
            calls: Arc::clone(&calls),
            releases: Vec::new(),
        })];
        let cache = ReleaseCache::new();
        let request = request(5);
        let preference = ReleasePreference::default();

        let first = cache
            .search(&indexers, &request, &preference, Duration::from_secs(60), false)
            .await
            .unwrap();
        assert!(first.is_empty());
        let after_first = calls.load(Ordering::SeqCst);

        cache
            .search(&indexers, &request, &preference, Duration::from_secs(60), false)
            .await
            .unwrap();

        assert!(
            calls.load(Ordering::SeqCst) > after_first,
            "an empty result must be searched again"
        );
    }

    #[tokio::test]
    async fn force_forces_a_search() {
        let calls = Arc::new(AtomicUsize::new(0));
        let indexers: Vec<Arc<dyn Indexer>> = vec![Arc::new(indexer(&calls))];
        let cache = ReleaseCache::new();
        let request = request(5);
        let preference = ReleasePreference::default();
        let ttl = Duration::from_secs(60);

        cache
            .search(&indexers, &request, &preference, ttl, false)
            .await
            .unwrap();
        let after_first = calls.load(Ordering::SeqCst);
        assert!(after_first > 0, "the first search must reach the indexer");

        // A plain repeat is a hit.
        cache
            .search(&indexers, &request, &preference, ttl, false)
            .await
            .unwrap();
        assert_eq!(
            calls.load(Ordering::SeqCst),
            after_first,
            "a repeat without force must not reach the indexer"
        );

        // Forced, it searches again even though the entry is fresh.
        cache
            .search(&indexers, &request, &preference, ttl, true)
            .await
            .unwrap();
        assert!(
            calls.load(Ordering::SeqCst) > after_first,
            "force must re-run the search"
        );
    }

    #[tokio::test]
    async fn concurrent_identical_searches_share_one_load() {
        let calls = Arc::new(AtomicUsize::new(0));
        let indexers: Vec<Arc<dyn Indexer>> = vec![Arc::new(indexer(&calls))];
        // A FRESH cache: priming it first would make the concurrent calls hits,
        // which proves nothing about de-duplication.
        let cache = Arc::new(ReleaseCache::new());
        let request = request(5);
        let preference = ReleasePreference::default();
        let ttl = Duration::from_secs(60);

        let mut handles = Vec::new();
        for _ in 0..8 {
            let cache = Arc::clone(&cache);
            let indexers = indexers.clone();
            let request = request.clone();
            let preference = preference.clone();
            handles.push(tokio::spawn(async move {
                cache.search(&indexers, &request, &preference, ttl, false).await
            }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // One query set, run once: the stub answers one query per `queries()`
        // entry, so any duplication would show as a multiple.
        let shared = calls.load(Ordering::SeqCst);
        assert!(shared > 0, "someone must search");
        assert_eq!(
            shared,
            request.queries().len(),
            "concurrent callers must share one search, not one each"
        );
    }

    #[test]
    fn the_key_separates_titles_and_preferences() {
        let request = ReleaseRequest {
            titles: vec!["A B".into(), "C".into()],
            episode: Some(1),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let split = ReleaseRequest {
            titles: vec!["A".into(), "B C".into()],
            ..request.clone()
        };
        let preference = ReleasePreference::default();

        assert_ne!(
            release_key(&request, &preference),
            release_key(&split, &preference),
            "title joins must not collide"
        );

        let mut other_preference = preference.clone();
        other_preference.min_seeders = 5;
        assert_ne!(
            release_key(&request, &preference),
            release_key(&request, &other_preference),
            "a preference change is a different key"
        );
    }
}