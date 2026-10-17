//! A TTL cache in front of an [`AnimeProvider`].
//!
//! Every metadata call funnels through one provider, so caching here covers
//! every caller without each needing to know about it. AniList rate-limits
//! hard and the UI is chatty -- the home page alone fires several requests at
//! once, and navigating back to a title it already loaded would ask again --
//! so the cache is the first line of defence against "too many requests".
//!
//! Mirrors the Svelte app's `src/lib/api/cache.ts`: same TTLs, same keys, same
//! two behaviours beyond a plain TTL --
//!   - in-flight de-duplication: two callers asking for the same key at once
//!     share a single request rather than racing two;
//!   - eviction at a fixed cap, so unbounded browse keys cannot leak.
//!
//! A rejection is deliberately NOT cached: a rate limit or a transient network
//! failure must be retried on the next call, not stick for the whole TTL.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::OnceCell;

use super::anilist::{AniListProvider, UserListEntry};
use super::traits::{AnimeProvider, ProviderError};
use crate::types::{
    Anime, AnimePage, BrowseQuery, HomeShelves, ListFilter, MediaTag, ProviderId,
    RecommendationsPage, ScheduledEpisode,
};

/// How long each kind of result stays fresh, matching `API_TTL` in the Svelte
/// app so the two frontends age data identically.
pub mod ttl {
    use std::time::Duration;

    /// The genre and tag catalogue is effectively static.
    pub const CATALOGUE: Duration = Duration::from_secs(60 * 60);
    /// Metadata for a single title drifts slowly.
    pub const DETAIL: Duration = Duration::from_secs(30 * 60);
    /// Rankings and browse results shift hour to hour.
    pub const LIST: Duration = Duration::from_secs(5 * 60);
    /// Airing times move, so the schedule is the least safe to hold.
    pub const SCHEDULE: Duration = Duration::from_secs(60);
}

/// Upper bound on stored entries per cache.
///
/// A browse key encodes the whole query, so an unbounded map would grow with
/// every filter combination the reader tries. The cap keeps memory flat.
const MAX_ENTRIES: usize = 200;

/// A `(value, expires-at)` pair held inside a [`OnceCell`].
type Entry<V> = (V, Instant);

/// The per-instance state, behind one mutex so the map and the eviction order
/// are always updated together. The lock is never held across an await.
struct Inner<V> {
    /// Key -> the cell holding its value once loaded. The [`OnceCell`] is what
    /// makes de-duplication work: the first caller runs the load and any caller
    /// arriving while it is in flight awaits the SAME cell.
    map: HashMap<String, std::sync::Arc<OnceCell<Entry<V>>>>,
    /// Keys in insertion order, so the oldest can be evicted first.
    order: VecDeque<String>,
}

/// A small TTL cache with in-flight de-duplication and a bounded size.
///
/// One instance serves one return shape; a provider holds several.
struct TtlCache<V> {
    inner: Mutex<Inner<V>>,
}

impl<V: Clone> TtlCache<V> {
    fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                map: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    /// Resolve `key` from the cache, or run `load` and remember the result.
    ///
    /// The `load` closure only runs when the key is absent or stale; a live
    /// value is cloned back. Concurrent callers for one key share a single
    /// `load` through the [`OnceCell`].
    async fn get_or_load<F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        load: F,
    ) -> Result<V, ProviderError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<V, ProviderError>>,
    {
        // Look up (or create) the cell under the lock, dropping a stale entry
        // first so the load below re-runs. Never await while holding the lock.
        let cell = {
            let mut inner = self.inner.lock().expect("cache poisoned");
            // A value past its expiry is evicted, so a fresh cell is made and
            // the load runs again. (An initialized-but-stale cell is already
            // complete, so no in-flight caller can still be awaiting it.)
            let stale = inner
                .map
                .get(key)
                .and_then(|cell| cell.get())
                .is_some_and(|(_, expires)| *expires <= Instant::now());
            if stale {
                inner.map.remove(key);
                inner.order.retain(|k| k != key);
            }
            inner
                .map
                .entry(key.to_string())
                .or_insert_with(|| std::sync::Arc::new(OnceCell::new()))
                .clone()
        };

        // First caller runs `load`; others await the same cell.
        let entry = cell
            .get_or_try_init(|| async move {
                let value = load().await?;
                Ok::<Entry<V>, ProviderError>((value, Instant::now() + ttl))
            })
            .await?;

        // Record insertion order and evict the oldest beyond the cap, now that
        // a real value exists (a failed load never occupies a slot).
        self.touch(key);
        Ok(entry.0.clone())
    }

    /// Note `key` as most-recently inserted and evict the oldest if over cap.
    fn touch(&self, key: &str) {
        let mut inner = self.inner.lock().expect("cache poisoned");
        if !inner.order.iter().any(|k| k == key) {
            inner.order.push_back(key.to_string());
        }
        while inner.order.len() > MAX_ENTRIES {
            if let Some(oldest) = inner.order.pop_front() {
                inner.map.remove(&oldest);
            }
        }
    }
}

/// A provider that caches another provider's results.
///
/// Wrap the concrete provider once at startup; every trait method then serves
/// repeated reads from memory instead of hitting AniList again.
pub struct CachedProvider<P: AnimeProvider> {
    /// The wrapped provider, held behind an `Arc` so the backend can keep its
    /// own concrete handle (for account writes) while the metadata path goes
    /// through the cache.
    inner: Arc<P>,
    trending: TtlCache<Vec<Anime>>,
    list: TtlCache<Vec<Anime>>,
    list_page: TtlCache<AnimePage>,
    browse: TtlCache<AnimePage>,
    by_id: TtlCache<Option<Anime>>,
    recommendations: TtlCache<RecommendationsPage>,
    genres: TtlCache<Vec<String>>,
    tags: TtlCache<Vec<MediaTag>>,
    schedule: TtlCache<Vec<ScheduledEpisode>>,
    home: TtlCache<HomeShelves>,
}

impl<P: AnimeProvider> CachedProvider<P> {
    /// Wrap `inner`, starting with empty caches.
    pub fn new(inner: Arc<P>) -> Self {
        Self {
            inner,
            trending: TtlCache::new(),
            list: TtlCache::new(),
            list_page: TtlCache::new(),
            browse: TtlCache::new(),
            by_id: TtlCache::new(),
            recommendations: TtlCache::new(),
            genres: TtlCache::new(),
            tags: TtlCache::new(),
            schedule: TtlCache::new(),
            home: TtlCache::new(),
        }
    }
}

/// A cache for the reader's own list (`user_list`).
///
/// Kept apart from [`CachedProvider`] because `user_list` is an
/// [`AniListProvider`] capability, not part of the [`AnimeProvider`] trait --
/// Jikan has no notion of a reader's list. Wrapping the concrete provider here
/// keeps that AniList-only surface out of the shared trait while still letting
/// the list be cached.
///
/// Unlike every other cached read, the reader's list changes by the reader's
/// OWN action rather than by time. A plain TTL would keep serving the list as
/// it looked before the last add/remove until the TTL expired. Instead the
/// cache key carries [`AniListProvider::list_revision`], which the provider
/// bumps on every successful write (and on a token change, since that is a
/// different account's list). A write therefore changes the key, so the next
/// read misses and re-fetches -- no explicit invalidation call, and no need for
/// the cache to know which work changed. Old revisions age out under the same
/// LRU cap as every other entry.
pub struct CachedAccount {
    inner: Arc<AniListProvider>,
    user_list: TtlCache<Vec<UserListEntry>>,
}

impl CachedAccount {
    /// Wrap `inner`, starting with an empty list cache.
    pub fn new(inner: Arc<AniListProvider>) -> Self {
        Self {
            inner,
            user_list: TtlCache::new(),
        }
    }

    /// The reader's whole list, served from cache until the TTL lapses or the
    /// reader writes to the list (which bumps the revision the key carries).
    ///
    /// A rejection is not cached, exactly as elsewhere: a transient failure or
    /// a rate limit must be retried on the next call.
    pub async fn user_list(&self) -> Result<Vec<UserListEntry>, ProviderError> {
        let key = format!("user_list:{}", self.inner.list_revision());
        self.user_list
            .get_or_load(&key, ttl::LIST, || self.inner.user_list())
            .await
    }
}

/// The cache key for a `ListFilter`, via its `Debug` spelling.
///
/// The enum derives `Debug`, which is stable enough for a cache key and avoids
/// pulling serialization into the hot path.
fn filter_key(filter: ListFilter) -> String {
    format!("{filter:?}")
}

#[async_trait]
impl<P: AnimeProvider> AnimeProvider for CachedProvider<P> {
    fn id(&self) -> ProviderId {
        self.inner.id()
    }

    async fn trending(&self, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let key = format!("trending:{limit}");
        self.trending
            .get_or_load(&key, ttl::LIST, || self.inner.trending(limit))
            .await
    }

    async fn home_shelves(&self) -> Result<HomeShelves, ProviderError> {
        // A single key: the whole home screen is one request with fixed
        // limits, so there is nothing to parameterise. Caching it means
        // returning to the home tab re-uses the one reply instead of asking
        // again (and the inner provider's own `home_shelves` is the single
        // combined query, not this trait's six-call default).
        self.home
            .get_or_load("home", ttl::LIST, || self.inner.home_shelves())
            .await
    }

    async fn list(&self, filter: ListFilter, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let key = format!("list:{}:{limit}", filter_key(filter));
        self.list
            .get_or_load(&key, ttl::LIST, || self.inner.list(filter, limit))
            .await
    }

    async fn list_page(
        &self,
        filter: ListFilter,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        let key = format!("list_page:{}:{page}:{per_page}", filter_key(filter));
        self.list_page
            .get_or_load(&key, ttl::LIST, || {
                self.inner.list_page(filter, page, per_page)
            })
            .await
    }

    async fn by_id(&self, id: i64) -> Result<Option<Anime>, ProviderError> {
        let key = format!("anime:{id}");
        self.by_id
            .get_or_load(&key, ttl::DETAIL, || self.inner.by_id(id))
            .await
    }

    async fn recommendations(
        &self,
        id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<RecommendationsPage, ProviderError> {
        let key = format!("recs:{id}:{page}:{per_page}");
        self.recommendations
            .get_or_load(&key, ttl::DETAIL, || {
                self.inner.recommendations(id, page, per_page)
            })
            .await
    }

    async fn genres(&self) -> Result<Vec<String>, ProviderError> {
        self.genres
            .get_or_load("genres", ttl::CATALOGUE, || self.inner.genres())
            .await
    }

    async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError> {
        self.tags
            .get_or_load("tags", ttl::CATALOGUE, || self.inner.tags())
            .await
    }

    async fn schedule(
        &self,
        from: i64,
        to: i64,
        limit: u32,
    ) -> Result<Vec<ScheduledEpisode>, ProviderError> {
        let key = format!("schedule:{from}:{to}:{limit}");
        self.schedule
            .get_or_load(&key, ttl::SCHEDULE, || {
                self.inner.schedule(from, to, limit)
            })
            .await
    }

    async fn browse(
        &self,
        query: BrowseQuery,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        // The whole query is part of the key, so two different filter sets
        // never collide. `serde_json` gives a stable spelling for the struct.
        let query_json = serde_json::to_string(&query).unwrap_or_default();
        let key = format!("browse:{query_json}:{page}:{per_page}");
        self.browse
            .get_or_load(&key, ttl::LIST, || {
                self.inner.browse(query.clone(), page, per_page)
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A provider that counts its calls and returns a fixed list, so a test can
    /// tell a cache hit (no new call) from a miss.
    struct CountingProvider {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }

    impl CountingProvider {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    calls: Arc::clone(&calls),
                    fail: false,
                },
                calls,
            )
        }

        fn failing() -> (Self, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    calls: Arc::clone(&calls),
                    fail: true,
                },
                calls,
            )
        }

        fn anime() -> Anime {
            // `Anime` has no struct-level `default`, so the required `id`,
            // `provider` and `title` must all be present for a decode.
            serde_json::from_value(serde_json::json!({
                "id": 1,
                "provider": "anilist",
                "title": {},
            }))
            .expect("valid anime")
        }
    }

    #[async_trait]
    impl AnimeProvider for CountingProvider {
        fn id(&self) -> ProviderId {
            ProviderId::AniList
        }

        async fn trending(&self, _limit: u32) -> Result<Vec<Anime>, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(ProviderError::Remote("nope".into()));
            }
            Ok(vec![Self::anime()])
        }

        async fn home_shelves(&self) -> Result<HomeShelves, ProviderError> {
            // Counted once: the trait's default would call `trending` and
            // `list` several times, so an override proves the cache forwards
            // to the provider's own single-request version.
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(HomeShelves {
                trending: vec![Self::anime()],
                ..HomeShelves::default()
            })
        }

        async fn list(
            &self,
            _filter: ListFilter,
            _limit: u32,
        ) -> Result<Vec<Anime>, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![Self::anime()])
        }

        async fn by_id(&self, _id: i64) -> Result<Option<Anime>, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Some(Self::anime()))
        }

        async fn recommendations(
            &self,
            _id: i64,
            _page: u32,
            _per_page: u32,
        ) -> Result<RecommendationsPage, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            unreachable!("not exercised")
        }

        async fn genres(&self) -> Result<Vec<String>, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec!["Action".into()])
        }

        async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }

        async fn schedule(
            &self,
            _from: i64,
            _to: i64,
            _limit: u32,
        ) -> Result<Vec<ScheduledEpisode>, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }

        async fn browse(
            &self,
            _query: BrowseQuery,
            _page: u32,
            _per_page: u32,
        ) -> Result<AnimePage, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            unreachable!("not exercised")
        }
    }

    #[tokio::test]
    async fn a_second_read_is_served_from_cache() {
        let (inner, calls) = CountingProvider::new();
        let cached = CachedProvider::new(Arc::new(inner));

        cached.trending(10).await.unwrap();
        cached.trending(10).await.unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1, "only one upstream call");
    }

    #[tokio::test]
    async fn different_limits_do_not_share_an_entry() {
        let (inner, calls) = CountingProvider::new();
        let cached = CachedProvider::new(Arc::new(inner));

        cached.trending(10).await.unwrap();
        cached.trending(20).await.unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn home_shelves_is_cached_and_forwarded() {
        let (inner, calls) = CountingProvider::new();
        let cached = CachedProvider::new(Arc::new(inner));

        let first = cached.home_shelves().await.unwrap();
        let second = cached.home_shelves().await.unwrap();

        assert_eq!(first.trending.len(), 1);
        assert_eq!(second.trending.len(), 1);
        // One upstream call, not the trait default's six, and the second read
        // is served from the cache.
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the combined home query runs once and is cached"
        );
    }

    #[tokio::test]
    async fn a_rejection_is_not_cached() {
        let (inner, calls) = CountingProvider::failing();
        let cached = CachedProvider::new(Arc::new(inner));

        assert!(cached.trending(10).await.is_err());
        assert!(cached.trending(10).await.is_err());

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "a failure must be retried, not remembered"
        );
    }

    #[tokio::test]
    async fn a_stale_entry_is_reloaded() {
        let (inner, calls) = CountingProvider::new();
        let cached = CachedProvider::new(Arc::new(inner));

        // Prime with a zero TTL so the value is instantly stale.
        cached
            .genres
            .get_or_load("genres", Duration::from_secs(0), || cached.inner.genres())
            .await
            .unwrap();
        cached.genres().await.unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 2, "expired entry reloads");
    }

    #[tokio::test]
    async fn concurrent_reads_share_one_load() {
        let (inner, calls) = CountingProvider::new();
        let cached = Arc::new(CachedProvider::new(Arc::new(inner)));

        // Fire several calls for the same key at once.
        let mut handles = Vec::new();
        for _ in 0..8 {
            let c = Arc::clone(&cached);
            handles.push(tokio::spawn(async move { c.trending(5).await.unwrap() }));
        }
        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1, "one load, shared");
    }

    // --- CachedAccount -----------------------------------------------------

    /// One `MediaListCollection` entry, matching what the list reader maps.
    fn list_entry(id: i64, status: &str, progress: u32) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "status": status,
            "progress": progress,
            "media": { "id": 21, "title": { "romaji": "One Piece" } },
        })
    }

    /// Mount a server answering the two requests `user_list` makes -- the
    /// `Viewer` lookup, then the collection -- and count the COLLECTION
    /// requests, which is the one the cache is meant to save.
    async fn account_with(entries: Vec<serde_json::Value>) -> (MockServer, CachedAccount) {
        let server = MockServer::start().await;

        // The Viewer lookup answers once per uncached read; it is small and is
        // not what the cache is about, so it is not counted here.
        Mock::given(method("POST"))
            .and(path("/"))
            .and(wiremock::matchers::body_string_contains("Viewer"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "data": { "Viewer": { "id": 7 } } }),
            ))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/"))
            .and(wiremock::matchers::body_string_contains("MediaListCollection"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "MediaListCollection": { "lists": [{ "entries": entries }] }
                }
            })))
            .mount(&server)
            .await;

        let provider = Arc::new(AniListProvider::with_endpoint(server.uri()));
        let account = CachedAccount::new(provider);
        (server, account)
    }

    /// How many collection requests the server received.
    async fn collection_calls(server: &MockServer) -> usize {
        server
            .received_requests()
            .await
            .expect("requests recorded")
            .iter()
            .filter(|request| {
                let body = String::from_utf8_lossy(&request.body);
                body.contains("MediaListCollection")
            })
            .count()
    }

    #[tokio::test]
    async fn a_second_list_read_is_served_from_cache() {
        let (server, account) = account_with(vec![list_entry(4, "CURRENT", 3)]).await;

        let first = account.user_list().await.unwrap();
        let second = account.user_list().await.unwrap();

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].entry_id, 4);
        assert_eq!(
            collection_calls(&server).await,
            1,
            "the second read must not hit AniList again"
        );
    }

    /// A write bumps the provider's list revision, so the cached list is no
    /// longer served: the next read re-fetches, and shows the new state.
    #[tokio::test]
    async fn a_list_write_invalidates_the_cached_list() {
        let (server, account) = account_with(vec![list_entry(4, "CURRENT", 3)]).await;

        // Prime the cache.
        account.user_list().await.unwrap();

        // A save that the server accepts. This bumps the revision.
        Mock::given(method("POST"))
            .and(path("/"))
            .and(wiremock::matchers::body_string_contains("SaveMediaListEntry"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "SaveMediaListEntry": { "id": 4, "status": "COMPLETED", "progress": 3 }
                }
            })))
            .mount(&server)
            .await;
        account
            .inner
            .save_list_entry(21, crate::types::ListStatus::Completed, None)
            .await
            .unwrap();

        // The read after the write must go back to AniList.
        account.user_list().await.unwrap();

        assert_eq!(
            collection_calls(&server).await,
            2,
            "a write must make the next read miss the cache"
        );
    }

    /// Signing in or out is a different list, so it invalidates just like a
    /// write -- the previous account's list must never be served.
    #[tokio::test]
    async fn a_token_change_invalidates_the_cached_list() {
        let (server, account) = account_with(vec![list_entry(4, "CURRENT", 3)]).await;

        account.user_list().await.unwrap();

        // Setting a token changes which list is being read.
        account.inner.set_token(Some("secret".into()));
        account.user_list().await.unwrap();

        assert_eq!(
            collection_calls(&server).await,
            2,
            "a token change must make the next read miss the cache"
        );
    }
}