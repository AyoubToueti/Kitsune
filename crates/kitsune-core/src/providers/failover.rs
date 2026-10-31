//! A provider decorator that falls back to a second source.
//!
//! AniList rate-limits hard and has downtime spikes. When it answers 429 or
//! 5xx, or the request never completes, the reader sees an error page for
//! something that is not their fault. This decorator answers the same read
//! from a second provider instead.
//!
//! Three rules shape it:
//!
//! - **Only availability failures fail over.** A `429`, a `5xx` and a
//!   `Transport` error all mean "this source cannot answer right now". A
//!   `Decode` or `Remote` error means OUR request was wrong, and serving a
//!   different source's data would hide a real bug behind plausible results.
//! - **A breaker, not a per-request wait.** Without one, every request during
//!   an outage pays AniList's timeout before reaching the fallback. After
//!   [`FAILURE_THRESHOLD`] consecutive failures the primary is skipped for
//!   [`COOLDOWN`], then probed once more.
//! - **Capabilities do not fail over silently.** `schedule` always asks the
//!   primary: the fallback has no airing schedule, so failing over would
//!   return an empty list that looks like "nothing airs" rather than "this
//!   source cannot say".
//!
//! Writes are not part of [`AnimeProvider`], so nothing here can send a
//! reader's list to the wrong service.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use super::anilist::ListEntry;
use super::traits::AnimeProvider;
use super::traits::ProviderError;
use crate::types::{
    Anime, AnimePage, BrowseQuery, HomeShelves, ListFilter, MediaTag, ProviderId,
    RecommendationsPage, ScheduledEpisode,
};

/// Consecutive failures before the primary is skipped without being asked.
///
/// Three rather than one: a single blip should not divert a whole session to
/// the fallback, but three in a row is a pattern rather than bad luck.
const FAILURE_THRESHOLD: u32 = 3;

/// How long the primary is skipped after tripping the breaker.
///
/// Long enough that an outage stops costing a timeout per request, short
/// enough that a recovered AniList is picked up without a restart.
const COOLDOWN: Duration = Duration::from_secs(60);

/// Whether an error means "this source is unavailable" rather than "our
/// request was wrong".
///
/// `Decode` and `Remote` are deliberately excluded: both mean the primary
/// answered and we could not use the answer, which is a bug here. Falling over
/// would serve different data and hide it.
///
/// **403 counts as unavailable for AniList**, which is not the usual reading of
/// that status. Observed live: when AniList disables its API it answers 403
/// with a GraphQL body saying "temporarily disabled due to severe stability
/// issues". A refused *token*, by contrast, comes back as HTTP 200 with an
/// `errors` array and maps to `Remote`, so treating 403 as availability here
/// cannot mask an authentication problem.
fn is_availability_failure(error: &ProviderError) -> bool {
    match error {
        ProviderError::Transport(_) => true,
        ProviderError::Status { status, .. } => {
            // 5xx is the server's problem; 429 is its explicit "later"; 403 is
            // AniList's "temporarily disabled" (see above).
            *status == 429 || *status == 403 || *status >= 500
        }
        ProviderError::Decode(_) | ProviderError::Remote(_) => false,
    }
}

/// The breaker's mutable state.
///
/// One mutex over both fields so the streak and the cooldown cannot disagree.
struct Breaker {
    /// Consecutive availability failures. Reset by any success.
    failures: AtomicU32,
    /// When the cooldown ends. `None` means the primary is allowed.
    ///
    /// Behind a `std` mutex, never held across an await.
    open_until: Mutex<Option<Instant>>,
}

impl Breaker {
    fn new() -> Self {
        Self {
            failures: AtomicU32::new(0),
            open_until: Mutex::new(None),
        }
    }

    /// Whether the primary may be attempted now.
    ///
    /// A cooldown that has lapsed is cleared here, so the next call probes the
    /// primary -- the "half-open" state, with no separate flag for it.
    fn primary_allowed(&self) -> bool {
        let mut open_until = self.open_until.lock().expect("breaker poisoned");
        match *open_until {
            None => true,
            Some(deadline) if Instant::now() >= deadline => {
                *open_until = None;
                true
            }
            Some(_) => false,
        }
    }

    /// Record a successful primary call: the source is healthy again.
    fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
    }

    /// Record an availability failure, tripping the breaker at the threshold.
    fn record_failure(&self) {
        let streak = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if streak >= FAILURE_THRESHOLD {
            let mut open_until = self.open_until.lock().expect("breaker poisoned");
            *open_until = Some(Instant::now() + COOLDOWN);
        }
    }
}

/// A provider that prefers `primary` and falls back to `fallback`.
///
/// Both are trait objects so the decorator does not care which concrete
/// sources it wraps, and so a third source can be added later without changing
/// this file.
pub struct FailoverProvider {
    primary: Arc<dyn AnimeProvider>,
    fallback: Arc<dyn AnimeProvider>,
    breaker: Breaker,
}

impl FailoverProvider {
    /// Wrap two providers.
    pub fn new(primary: Arc<dyn AnimeProvider>, fallback: Arc<dyn AnimeProvider>) -> Self {
        Self {
            primary,
            fallback,
            breaker: Breaker::new(),
        }
    }

    /// Run `primary`, falling back to `fallback` when the first is unavailable.
    ///
    /// Both futures are passed in READY-BUILT and already owning their
    /// provider (each call site clones the `Arc` into an `async move` block).
    /// That is deliberate: a closure taking `&dyn AnimeProvider` would have to
    /// return a future borrowing its argument, which needs a higher-ranked
    /// bound and boxing to express. Cloning an `Arc` is free by comparison.
    ///
    /// Building the fallback future eagerly is harmless -- a future does no
    /// work until polled -- and it keeps this helper free of any knowledge of
    /// which method is being called.
    async fn with_fallback<T, PF, FF>(
        &self,
        primary: PF,
        fallback: FF,
    ) -> Result<T, ProviderError>
    where
        PF: std::future::Future<Output = Result<T, ProviderError>>,
        FF: std::future::Future<Output = Result<T, ProviderError>>,
    {
        if self.breaker.primary_allowed() {
            match primary.await {
                Ok(value) => {
                    self.breaker.record_success();
                    return Ok(value);
                }
                Err(error) if is_availability_failure(&error) => {
                    // Deliberately not logged as an error: a tripped breaker
                    // during an outage is expected, and a log line per request
                    // would bury the one that matters.
                    self.breaker.record_failure();
                }
                Err(error) => {
                    // Not an availability problem, so the primary is healthy;
                    // the breaker must not punish it for our bad request.
                    self.breaker.record_success();
                    return Err(error);
                }
            }
        }

        fallback.await
    }

    /// Run a read against the PRIMARY only.
    ///
    /// Used for capabilities the fallback does not have. Returning the
    /// primary's error is better than an empty list from a source that was
    /// never able to answer: the caller can tell "unavailable" from "none".
    async fn primary_only<T, F>(&self, primary: F) -> Result<T, ProviderError>
    where
        F: std::future::Future<Output = Result<T, ProviderError>>,
    {
        match primary.await {
            Ok(value) => {
                self.breaker.record_success();
                Ok(value)
            }
            Err(error) => {
                if is_availability_failure(&error) {
                    self.breaker.record_failure();
                }
                Err(error)
            }
        }
    }

    /// The primary, cloned so a future can own it.
    fn primary_handle(&self) -> Arc<dyn AnimeProvider> {
        Arc::clone(&self.primary)
    }

    /// The fallback, cloned so a future can own it.
    fn fallback_handle(&self) -> Arc<dyn AnimeProvider> {
        Arc::clone(&self.fallback)
    }
}

// Every method is overridden explicitly. A method left to the trait default
// would answer from NEITHER provider -- the default bodies are written for a
// provider that can serve them, not for a decorator -- and the failure is
// silent, because a default always compiles. This is the trap that made
// `CachedProvider` render an empty home page.
#[async_trait]
impl AnimeProvider for FailoverProvider {
    fn id(&self) -> ProviderId {
        self.primary.id()
    }

    async fn trending(&self, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        self.with_fallback(
            async move { primary.trending(limit).await },
            async move { fallback.trending(limit).await },
        )
        .await
    }

    async fn list(&self, filter: ListFilter, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        self.with_fallback(
            async move { primary.list(filter, limit).await },
            async move { fallback.list(filter, limit).await },
        )
        .await
    }

    async fn list_page(
        &self,
        filter: ListFilter,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        self.with_fallback(
            async move { primary.list_page(filter, page, per_page).await },
            async move { fallback.list_page(filter, page, per_page).await },
        )
        .await
    }

    async fn by_id(&self, id: i64) -> Result<Option<Anime>, ProviderError> {
        // A bare id carries no namespace, so the only safe reading is "it is
        // the primary's", which keeps this primary-only.
        let primary = self.primary_handle();
        self.primary_only(async move { primary.by_id(id).await }).await
    }

    async fn by_id_in(
        &self,
        provider: ProviderId,
        id: i64,
    ) -> Result<Option<Anime>, ProviderError> {
        // The caller told us WHICH namespace the id belongs to, which removes
        // the ambiguity that made `by_id` primary-only.
        //
        // - The id is the primary's: primary first, fallback second. Both can
        //   address it (`by_anilist_id` resolves through the mapping table).
        // - The id belongs to some other source: only that source can answer.
        if provider == self.primary.id() {
            let primary = self.primary_handle();
            let fallback = self.fallback_handle();
            self.with_fallback(
                async move { primary.by_id_in(provider, id).await },
                async move { fallback.by_id_in(provider, id).await },
            )
            .await
        } else if provider == self.fallback.id() {
            // A card the FALLBACK rendered. The primary's namespace is a
            // different set of numbers, so asking it would open the wrong
            // work; go straight to the source that issued the id.
            let fallback = self.fallback_handle();
            self.primary_only(async move { fallback.by_id_in(provider, id).await })
                .await
        } else {
            // Neither source issued this id (TMDB, an indexer). Nothing here
            // can resolve it.
            Ok(None)
        }
    }

    async fn by_anilist_id(&self, anilist_id: i64) -> Result<Option<Anime>, ProviderError> {
        // Genuinely resolvable by BOTH: the primary speaks AniList natively,
        // and the fallback maps it through its own table. This is what makes a
        // detail page work during an AniList outage.
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        self.with_fallback(
            async move { primary.by_anilist_id(anilist_id).await },
            async move { fallback.by_anilist_id(anilist_id).await },
        )
        .await
    }

    async fn by_id_with_entry(
        &self,
        id: i64,
    ) -> Result<(Option<Anime>, Option<ListEntry>), ProviderError> {
        // The entry is an AniList concept and the fallback has no token, so
        // this is primary-only for the same reason as `by_id`.
        let primary = self.primary_handle();
        self.primary_only(async move { primary.by_id_with_entry(id).await })
            .await
    }

    fn list_revision(&self) -> Option<u64> {
        // The revision is bumped by writes, which only the primary accepts.
        self.primary.list_revision()
    }

    fn cached_entry(&self, id: i64) -> Option<Option<ListEntry>> {
        // A synchronous cache read: asking the fallback cannot help, since it
        // keeps no such cache.
        self.primary.cached_entry(id)
    }

    fn cached_partial(&self, id: i64) -> Option<Anime> {
        // Deliberately BOTH sources: whichever answered `by_id` last is the one
        // holding a partial, and the caller only wants "anything to paint now".
        self.primary
            .cached_partial(id)
            .or_else(|| self.fallback.cached_partial(id))
    }

    async fn recommendations(
        &self,
        id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<RecommendationsPage, ProviderError> {
        // The fallback has no recommendations at all, so failing over would
        // present an empty row as though the work had none. The UI shows a
        // "source unavailable" state from the error instead.
        let primary = self.primary_handle();
        self.primary_only(async move { primary.recommendations(id, page, per_page).await })
            .await
    }

    async fn genres(&self) -> Result<Vec<String>, ProviderError> {
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        self.with_fallback(
            async move { primary.genres().await },
            async move { fallback.genres().await },
        )
        .await
    }

    async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError> {
        // A genuine failover case: the fallback returns an EMPTY catalogue
        // rather than an error, which is the right degraded answer -- the tag
        // filter simply disappears.
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        self.with_fallback(
            async move { primary.tags().await },
            async move { fallback.tags().await },
        )
        .await
    }

    async fn schedule(
        &self,
        from: i64,
        to: i64,
        limit: u32,
    ) -> Result<Vec<ScheduledEpisode>, ProviderError> {
        // Primary-only: the fallback publishes no airing schedule, so an empty
        // list from it would read as "nothing is airing" rather than "this
        // source cannot answer".
        let primary = self.primary_handle();
        self.primary_only(async move { primary.schedule(from, to, limit).await })
            .await
    }

    async fn browse(
        &self,
        query: BrowseQuery,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        // Two owned copies of the query: both futures are built up front, and
        // each needs its own.
        let fallback_query = query.clone();
        self.with_fallback(
            async move { primary.browse(query, page, per_page).await },
            async move { fallback.browse(fallback_query, page, per_page).await },
        )
        .await
    }

    async fn home_shelves(&self) -> Result<HomeShelves, ProviderError> {
        // Delegated rather than rebuilt from `list()` calls: the primary's own
        // override batches six queries into one request, and the trait default
        // would undo that -- the exact regression `CachedProvider` once caused.
        let primary = self.primary_handle();
        let fallback = self.fallback_handle();
        self.with_fallback(
            async move { primary.home_shelves().await },
            async move { fallback.home_shelves().await },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::AtomicUsize;

    /// A provider whose every read is scripted, and which counts its calls.
    ///
    /// Defined per-test rather than once, because the interesting cases differ
    /// in WHICH method they exercise.
    struct Scripted {
        id: ProviderId,
        /// The result every read returns, cloned per call.
        outcome: Result<(), StatusCode>,
        calls: AtomicUsize,
    }

    /// What a scripted call returns: success, or a 429/500/transport error.
    #[derive(Clone, Copy)]
    enum StatusCode {
        RateLimited,
        ServerError,
        Transport,
    }

    impl Scripted {
        fn new(id: ProviderId, outcome: Result<(), StatusCode>) -> Arc<Self> {
            Arc::new(Self {
                id,
                outcome,
                calls: AtomicUsize::new(0),
            })
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }

        fn record(&self) -> Result<(), ProviderError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            match self.outcome {
                Ok(()) => Ok(()),
                Err(StatusCode::RateLimited) => Err(ProviderError::Status {
                    status: 429,
                    body: "slow down".into(),
                }),
                Err(StatusCode::ServerError) => Err(ProviderError::Status {
                    status: 500,
                    body: "boom".into(),
                }),
                Err(StatusCode::Transport) => Err(ProviderError::Transport("refused".into())),
            }
        }
    }

    #[async_trait]
    impl AnimeProvider for Scripted {
        fn id(&self) -> ProviderId {
            self.id
        }

        async fn trending(&self, _limit: u32) -> Result<Vec<Anime>, ProviderError> {
            self.record()?;
            Ok(Vec::new())
        }

        async fn list(&self, _filter: ListFilter, _limit: u32) -> Result<Vec<Anime>, ProviderError> {
            self.record()?;
            Ok(Vec::new())
        }

        async fn list_page(
            &self,
            _filter: ListFilter,
            page: u32,
            _per_page: u32,
        ) -> Result<AnimePage, ProviderError> {
            self.record()?;
            Ok(AnimePage {
                items: Vec::new(),
                page_info: crate::types::PageInfo {
                    total: 0,
                    current_page: page,
                    last_page: page,
                    has_next_page: false,
                },
            })
        }

        async fn by_id(&self, _id: i64) -> Result<Option<Anime>, ProviderError> {
            self.record()?;
            Ok(None)
        }

        async fn by_id_in(
            &self,
            _provider: ProviderId,
            id: i64,
        ) -> Result<Option<Anime>, ProviderError> {
            // Delegates to `by_id` so the call is RECORDED. Without this the
            // trait default answers `Ok(None)` without touching `calls`, and a
            // test asserting on the count silently measures nothing.
            self.by_id(id).await
        }

        async fn by_anilist_id(&self, anilist_id: i64) -> Result<Option<Anime>, ProviderError> {
            // Recorded for the same reason as `by_id_in`: the decorator's
            // fallback-on-outage test needs to see BOTH providers attempted.
            self.by_id(anilist_id).await
        }

        async fn recommendations(
            &self,
            _id: i64,
            page: u32,
            _per_page: u32,
        ) -> Result<RecommendationsPage, ProviderError> {
            self.record()?;
            Ok(RecommendationsPage {
                items: Vec::new(),
                page_info: crate::types::PageInfo {
                    total: 0,
                    current_page: page,
                    last_page: page,
                    has_next_page: false,
                },
            })
        }

        async fn genres(&self) -> Result<Vec<String>, ProviderError> {
            self.record()?;
            Ok(Vec::new())
        }

        async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError> {
            self.record()?;
            Ok(Vec::new())
        }

        async fn schedule(
            &self,
            _from: i64,
            _to: i64,
            _limit: u32,
        ) -> Result<Vec<ScheduledEpisode>, ProviderError> {
            self.record()?;
            Ok(Vec::new())
        }

        async fn browse(
            &self,
            _query: BrowseQuery,
            page: u32,
            _per_page: u32,
        ) -> Result<AnimePage, ProviderError> {
            self.record()?;
            Ok(AnimePage {
                items: Vec::new(),
                page_info: crate::types::PageInfo {
                    total: 0,
                    current_page: page,
                    last_page: page,
                    has_next_page: false,
                },
            })
        }

        async fn home_shelves(&self) -> Result<HomeShelves, ProviderError> {
            self.record()?;
            Ok(HomeShelves::default())
        }
    }

    fn wrap(
        primary: Arc<Scripted>,
        fallback: Arc<Scripted>,
    ) -> (FailoverProvider, Arc<Scripted>, Arc<Scripted>) {
        let provider = FailoverProvider::new(primary.clone(), fallback.clone());
        (provider, primary, fallback)
    }

    #[tokio::test]
    async fn a_healthy_primary_serves_the_read() {
        let (provider, primary, fallback) =
            wrap(Scripted::new(ProviderId::AniList, Ok(())), Scripted::new(ProviderId::Kitsu, Ok(())));

        provider.trending(5).await.unwrap();

        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 0, "the fallback must not be touched");
    }

    #[tokio::test]
    async fn a_rate_limit_fails_over() {
        let (provider, primary, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::RateLimited)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        provider.trending(5).await.unwrap();

        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 1, "the fallback must answer");
    }

    #[tokio::test]
    async fn a_server_error_fails_over() {
        let (provider, _, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::ServerError)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        provider.list(ListFilter::TopRated, 5).await.unwrap();

        assert_eq!(fallback.calls(), 1);
    }

    #[tokio::test]
    async fn a_transport_error_fails_over() {
        let (provider, _, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::Transport)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        provider.browse(BrowseQuery::default(), 1, 20).await.unwrap();

        assert_eq!(fallback.calls(), 1);
    }

    #[tokio::test]
    async fn a_decode_error_does_not_fail_over() {
        // A decode failure is OUR bug. Serving the fallback's data would hide
        // it behind plausible-looking results.
        struct BadDecode;
        #[async_trait]
        impl AnimeProvider for BadDecode {
            fn id(&self) -> ProviderId {
                ProviderId::AniList
            }
            async fn trending(&self, _limit: u32) -> Result<Vec<Anime>, ProviderError> {
                Err(ProviderError::Decode("unexpected shape".into()))
            }
            async fn list(
                &self,
                _filter: ListFilter,
                _limit: u32,
            ) -> Result<Vec<Anime>, ProviderError> {
                unreachable!()
            }
            async fn by_id(&self, _id: i64) -> Result<Option<Anime>, ProviderError> {
                unreachable!()
            }
            async fn recommendations(
                &self,
                _id: i64,
                _page: u32,
                _per_page: u32,
            ) -> Result<RecommendationsPage, ProviderError> {
                unreachable!()
            }
            async fn genres(&self) -> Result<Vec<String>, ProviderError> {
                unreachable!()
            }
            async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError> {
                unreachable!()
            }
            async fn schedule(
                &self,
                _from: i64,
                _to: i64,
                _limit: u32,
            ) -> Result<Vec<ScheduledEpisode>, ProviderError> {
                unreachable!()
            }
            async fn browse(
                &self,
                _query: BrowseQuery,
                _page: u32,
                _per_page: u32,
            ) -> Result<AnimePage, ProviderError> {
                unreachable!()
            }
        }

        let fallback = Scripted::new(ProviderId::Kitsu, Ok(()));
        let provider = FailoverProvider::new(Arc::new(BadDecode), fallback.clone());

        let error = provider.trending(5).await.unwrap_err();

        assert!(matches!(error, ProviderError::Decode(_)));
        assert_eq!(fallback.calls(), 0, "a decode error must not fall over");
    }

    #[tokio::test]
    async fn schedule_never_asks_the_fallback() {
        // The fallback has no airing schedule. An empty list from it would read
        // as "nothing airs" rather than "this source cannot say".
        let (provider, primary, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::ServerError)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        let error = provider.schedule(0, 100, 20).await.unwrap_err();

        assert!(matches!(error, ProviderError::Status { status: 500, .. }));
        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 0);
    }

    #[tokio::test]
    async fn recommendations_never_ask_the_fallback() {
        let (provider, _, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::RateLimited)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        assert!(provider.recommendations(1, 1, 24).await.is_err());
        assert_eq!(fallback.calls(), 0);
    }

    #[tokio::test]
    async fn by_id_never_asks_the_fallback() {
        // The fallback is keyed on a different id namespace: an AniList id sent
        // there would 404, or match an unrelated work.
        let (provider, _, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::RateLimited)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        assert!(provider.by_id(16498).await.is_err());
        assert_eq!(fallback.calls(), 0);
    }

    #[tokio::test]
    async fn the_breaker_stops_asking_a_failing_primary() {
        let (provider, primary, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::RateLimited)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        // Each of the first three requests asks the primary and fails over.
        for _ in 0..FAILURE_THRESHOLD {
            provider.trending(5).await.unwrap();
        }
        assert_eq!(primary.calls(), FAILURE_THRESHOLD as usize);

        // The fourth skips the primary entirely.
        provider.trending(5).await.unwrap();
        assert_eq!(
            primary.calls(),
            FAILURE_THRESHOLD as usize,
            "the breaker must stop asking the primary"
        );
        assert_eq!(fallback.calls(), FAILURE_THRESHOLD as usize + 1);
    }

    #[tokio::test]
    async fn a_success_resets_the_failure_streak() {
        // A provider that fails twice, succeeds once, then fails twice must
        // NOT trip: the streak is consecutive failures, not a total.
        let primary = Scripted::new(ProviderId::AniList, Ok(()));
        let fallback = Scripted::new(ProviderId::Kitsu, Ok(()));
        let provider = FailoverProvider::new(primary.clone(), fallback.clone());

        provider.breaker.record_failure();
        provider.breaker.record_failure();
        provider.breaker.record_success();
        provider.breaker.record_failure();
        provider.breaker.record_failure();

        assert!(provider.breaker.primary_allowed(), "streak was broken");
    }

    #[tokio::test]
    async fn a_bad_request_does_not_trip_the_breaker() {
        // A 400 is our fault, not the server's. Punishing the primary for it
        // would divert traffic on a bug.
        let (provider, _, _) = wrap(
            Scripted::new(ProviderId::AniList, Ok(())),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        provider.breaker.record_failure();
        provider.breaker.record_failure();
        // A non-availability error records a success instead.
        let error = ProviderError::Remote("bad query".into());
        assert!(!is_availability_failure(&error));
    }

    #[test]
    fn only_availability_failures_are_classified_as_such() {
        assert!(is_availability_failure(&ProviderError::Transport("x".into())));
        assert!(is_availability_failure(&ProviderError::Status {
            status: 429,
            body: String::new()
        }));
        assert!(is_availability_failure(&ProviderError::Status {
            status: 503,
            body: String::new()
        }));
        // AniList answers 403 when it disables the API, so this MUST fail over.
        // Observed live: "The AniList API has been temporarily disabled due to
        // severe stability issues."
        assert!(
            is_availability_failure(&ProviderError::Status {
                status: 403,
                body: "The AniList API has been temporarily disabled".into()
            }),
            "a disabled API must fall over, or the home page renders empty"
        );
        assert!(!is_availability_failure(&ProviderError::Status {
            status: 400,
            body: String::new()
        }));
        assert!(!is_availability_failure(&ProviderError::Status {
            status: 404,
            body: String::new()
        }));
        assert!(!is_availability_failure(&ProviderError::Decode("x".into())));
        assert!(!is_availability_failure(&ProviderError::Remote("x".into())));
    }

    /// The reported symptom, end to end: AniList disables its API (403) and the
    /// home screen must still fill from Kitsu.
    ///
    /// This drives the REAL `KitsuProvider` over a wiremock server, so it also
    /// covers the field mapping the QML reads. A stub here would have hidden
    /// the very bug it is meant to catch.
    #[tokio::test]
    async fn a_disabled_anilist_still_fills_the_home_screen_from_kitsu() {
        use crate::providers::kitsu::KitsuProvider;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // The primary: a server that answers exactly as AniList does when it
        // disables the API.
        let anilist = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "errors": [{ "message": "The AniList API has been temporarily disabled" }],
                "data": null
            })))
            .mount(&anilist)
            .await;

        // The fallback: a Kitsu server with one anime per endpoint shape.
        let kitsu = MockServer::start().await;
        let anime = serde_json::json!({
            "id": "7442",
            "type": "anime",
            "attributes": {
                "canonicalTitle": "Attack on Titan",
                "titles": { "en": "Attack on Titan", "en_jp": "Shingeki no Kyojin" },
                "synopsis": "Titans.",
                "averageRating": "84.4",
                "userCount": 1000,
                "startDate": "2013-04-07",
                "status": "finished",
                "subtype": "TV",
                "posterImage": { "large": "https://media.kitsu.app/p.jpg" },
                "coverImage": { "large": "https://media.kitsu.app/c.jpg" },
                "episodeCount": 25,
                "episodeLength": 24,
                "youtubeVideoId": "abc123",
                "nsfw": false
            }
        });
        let collection = serde_json::json!({
            "data": [anime.clone()],
            "meta": { "count": 1 },
            "links": {}
        });

        // The trending endpoint returns a BARE array, unlike every other one.
        Mock::given(method("GET"))
            .and(path("/trending/anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [anime.clone()]
            })))
            .mount(&kitsu)
            .await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(collection))
            .mount(&kitsu)
            .await;

        let anilist_provider =
            crate::providers::anilist::AniListProvider::with_endpoint(anilist.uri());
        let kitsu_provider = KitsuProvider::with_endpoint(kitsu.uri());

        let provider = FailoverProvider::new(
            Arc::new(anilist_provider),
            Arc::new(kitsu_provider),
        );

        let shelves = provider.home_shelves().await.expect("home must not fail");

        // Every shelf the home page renders must have content, otherwise the
        // reader sees the same empty page as before the fallback existed.
        assert!(
            !shelves.trending.is_empty(),
            "hero + rail come from trending"
        );
        assert!(!shelves.top_airing.is_empty());
        assert!(!shelves.most_popular.is_empty());
        assert!(!shelves.top_rated.is_empty());
        assert!(!shelves.latest_completed.is_empty());
        assert!(!shelves.top_100.is_empty());

        // And the mapped work must carry what a CARD renders, not just an id.
        let card = &shelves.trending[0];
        assert_eq!(card.provider, ProviderId::Kitsu);
        assert_eq!(card.title.romaji.as_deref(), Some("Shingeki no Kyojin"));
        assert_eq!(card.title.english.as_deref(), Some("Attack on Titan"));
        assert!(card.cover_image.is_some(), "a card with no cover is blank");
        assert!(card.average_score.is_some());
        assert_eq!(card.episode_count, Some(25));
    }

    /// The reported bug: an anime opened from a card the FALLBACK rendered must
    /// load, even though its id belongs to the fallback's namespace.
    #[tokio::test]
    async fn by_id_in_opens_a_fallback_card_through_the_fallback() {
        let (provider, primary, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::RateLimited)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        provider
            .by_id_in(ProviderId::Kitsu, 7442)
            .await
            .unwrap();

        // The primary must NOT be asked: it would read 7442 as an AniList id
        // and return an unrelated work.
        assert_eq!(primary.calls(), 0, "a Kitsu id must not go to AniList");
        assert_eq!(fallback.calls(), 1);
    }

    /// A detail page opened during an AniList outage must still work: the
    /// fallback resolves the AniList id through its own mapping table.
    #[tokio::test]
    async fn an_anilist_id_falls_over_to_the_fallback() {
        let (provider, primary, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Err(StatusCode::ServerError)),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        provider.by_anilist_id(16498).await.unwrap();

        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 1, "the mapping table must be tried");
    }

    /// The `Scripted` stub's default `by_id_in` delegates to `by_id`, so a
    /// wrong namespace routing would show up as a call on the primary.
    #[tokio::test]
    async fn by_id_in_ignores_namespaces_neither_provider_owns() {
        let (provider, primary, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Ok(())),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        let found = provider.by_id_in(ProviderId::Tmdb, 1).await.unwrap();

        assert!(found.is_none());
        assert_eq!(primary.calls(), 0);
        assert_eq!(fallback.calls(), 0);
    }

    #[tokio::test]
    async fn id_reports_the_primary() {
        let (provider, _, _) = wrap(
            Scripted::new(ProviderId::AniList, Ok(())),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );
        assert_eq!(provider.id(), ProviderId::AniList);
    }

    /// The decorator must override EVERY method.
    ///
    /// A missed override compiles and silently answers from the trait default,
    /// which for a decorator means from neither provider. `CachedProvider` hit
    /// exactly this and rendered an empty home page.
    #[tokio::test]
    async fn every_read_reaches_one_of_the_two_providers() {
        let (provider, primary, fallback) = wrap(
            Scripted::new(ProviderId::AniList, Ok(())),
            Scripted::new(ProviderId::Kitsu, Ok(())),
        );

        provider.trending(1).await.unwrap();
        provider.list(ListFilter::TopRated, 1).await.unwrap();
        provider.list_page(ListFilter::TopRated, 1, 1).await.unwrap();
        provider.by_id(1).await.unwrap();
        provider.by_id_with_entry(1).await.unwrap();
        provider.recommendations(1, 1, 1).await.unwrap();
        provider.genres().await.unwrap();
        provider.tags().await.unwrap();
        provider.schedule(0, 1, 1).await.unwrap();
        provider.browse(BrowseQuery::default(), 1, 1).await.unwrap();
        provider.home_shelves().await.unwrap();

        // Eleven reads: `by_id_with_entry` defaults to `by_id` on Scripted, so
        // the count is one per call rather than two.
        assert_eq!(primary.calls(), 11);
        assert_eq!(fallback.calls(), 0);
    }
}