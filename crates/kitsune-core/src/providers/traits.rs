//! The provider abstraction.
//!
//! Every metadata source (AniList now; TMDB later) implements
//! [`AnimeProvider`]. Callers depend on this trait rather than a concrete
//! client, so adding a source never touches the UI or the command layer.

use async_trait::async_trait;

use crate::types::{
    Anime, AnimePage, BrowseQuery, HomeShelves, ListFilter, MediaTag, ProviderId,
    RecommendationsPage, ScheduledEpisode,
};

/// How many trending titles the home page requests (hero + rail).
pub const HOME_TRENDING_LIMIT: u32 = 20;
/// How many rows each curated block shows.
pub const HOME_BLOCK_LIMIT: u32 = 5;
/// How many rows the Top 100 preview shows.
pub const HOME_TOP_100_LIMIT: u32 = 10;

/// Why a provider call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// The request never completed (DNS, connect, timeout).
    Transport(String),
    /// The server answered with a non-success status.
    Status { status: u16, body: String },
    /// The response was not the JSON shape we expect.
    Decode(String),
    /// The provider reported errors in its own envelope (e.g. GraphQL).
    Remote(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(msg) => write!(f, "provider request failed: {msg}"),
            Self::Status { status, body } => {
                write!(f, "provider returned HTTP {status}: {body}")
            }
            Self::Decode(msg) => write!(f, "could not decode provider response: {msg}"),
            Self::Remote(msg) => write!(f, "provider reported an error: {msg}"),
        }
    }
}

impl std::error::Error for ProviderError {}

/// A source of anime metadata.
///
/// Async and object-safe so the app can hold a `Vec<Box<dyn AnimeProvider>>`
/// and treat sources interchangeably.
#[async_trait]
pub trait AnimeProvider: Send + Sync {
    /// Which source this is, matching the ids it returns.
    fn id(&self) -> ProviderId;

    /// Currently popular titles, for the home screen.
    async fn trending(&self, limit: u32) -> Result<Vec<Anime>, ProviderError>;

    /// A curated list, chosen by intent rather than by provider-specific
    /// sort arguments.
    ///
    /// One method rather than one per shelf: the shelves differ only in
    /// ordering and status, and collapsing them keeps provider-specific
    /// vocabulary out of this trait.
    async fn list(&self, filter: ListFilter, limit: u32) -> Result<Vec<Anime>, ProviderError>;

    /// One page of a curated list, with the paging metadata needed to ask for
    /// another.
    ///
    /// Backs the scrolling "Top 100" view: [`AnimeProvider::list`] returns a
    /// bare `Vec` with no way to say whether more exists, so it cannot drive
    /// infinite scroll. The default wraps `list()` and synthesises the page
    /// info, which is enough for a provider that only ever serves one page.
    /// A provider whose list query already carries `Page.pageInfo` (AniList)
    /// overrides this to return the real totals.
    async fn list_page(
        &self,
        filter: ListFilter,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        let items = self.list(filter, per_page).await?;
        let count = items.len() as u32;
        // The default can only ever serve the first page: it has no way to ask
        // `list()` for a later one. `has_next_page` is a best-effort guess --
        // a full page suggests more may follow.
        let current_page = page.max(1);
        Ok(AnimePage {
            page_info: crate::types::PageInfo {
                total: count,
                current_page,
                last_page: current_page,
                has_next_page: current_page <= 1 && count >= per_page,
            },
            items,
        })
    }

    /// Look up a single title. `Ok(None)` means "not found", which is not
    /// an error — callers may legitimately probe for ids that do not exist.
    async fn by_id(&self, id: i64) -> Result<Option<Anime>, ProviderError>;

    /// Look up a title by an id whose NAMESPACE the caller already knows.
    ///
    /// Ids are per-provider, so a bare `i64` is ambiguous once more than one
    /// source is in play: Attack on Titan is `16498` on AniList and `7442` on
    /// Kitsu. A caller that just rendered a card knows which namespace it came
    /// from, and passing that through is the difference between opening the
    /// right work and opening whatever happens to share the number.
    ///
    /// Defaults to [`Self::by_id`], which is correct for a single-source
    /// provider: it has only one namespace, so the hint cannot disagree with
    /// it.
    async fn by_id_in(
        &self,
        _provider: ProviderId,
        id: i64,
    ) -> Result<Option<Anime>, ProviderError> {
        self.by_id(id).await
    }

    /// Look up a title by its **AniList** id, from a provider that speaks a
    /// different namespace.
    ///
    /// This is what lets the fallback answer a detail request during an
    /// AniList outage: it resolves the AniList id to its own id through its
    /// own mapping table, then fetches. No AniList request is involved, so it
    /// works while AniList is down.
    ///
    /// Defaults to `Ok(None)` — "I cannot address AniList ids" — which is the
    /// truth for a provider with no mapping table.
    async fn by_anilist_id(&self, _anilist_id: i64) -> Result<Option<Anime>, ProviderError> {
        Ok(None)
    }

    /// Look up a single title AND the reader's own list entry for it.
    ///
    /// The entry is a field ON the media node, so a provider that supports it
    /// can answer both in ONE request. That matters because the rate limit is
    /// counted per request, not per field: fetching the entry separately
    /// doubled the cost of opening a detail page while signed in.
    ///
    /// Defaults to `by_id` plus "no entry", which is correct for a provider
    /// with no notion of a reader's list, and keeps existing callers working.
    async fn by_id_with_entry(
        &self,
        id: i64,
    ) -> Result<(Option<Anime>, Option<crate::providers::anilist::ListEntry>), ProviderError>
    {
        Ok((self.by_id(id).await?, None))
    }

    /// The reader's own list entry for a work, when one was already fetched.
    ///
    /// [`AnimeProvider::by_id_with_entry`] returns the entry alongside the
    /// work, so the detail page does not need a second request for it. A
    /// caller that asks again -- `DetailPage` does, from its own flow -- can
    /// answer from here instead of paying for the round trip.
    ///
    /// A counter that moves whenever the reader's own list changes.
    ///
    /// A cache holding list entries keys them on this value, so an add or a
    /// remove makes the next read miss instead of serving the pre-write
    /// status for the whole TTL. `None` for a provider with no reader's list.
    fn list_revision(&self) -> Option<u64> {
        None
    }

    /// Three states, which is why the return is doubly optional:
    ///
    /// - `None` -- nothing known; the caller must do a real lookup.
    /// - `Some(None)` -- known, and the work is NOT on the reader's list.
    /// - `Some(Some(entry))` -- known, with the entry.
    ///
    /// The middle case matters: AniList answers `mediaListEntry: null` both
    /// for "not on the list" and for "signed out", so caching only the
    /// presence of an entry would send every signed-in reader back for a
    /// second request on every work they have not added.
    ///
    /// Defaulted to `None` for providers that keep no such cache.
    fn cached_entry(&self, _id: i64) -> Option<Option<super::anilist::ListEntry>> {
        None
    }

    /// A work this provider has already listed, without any request.
    ///
    /// A card on the home screen, a browse grid or a rail already carries
    /// nearly everything the detail page renders; only `relations` and
    /// `recommendations` are held back (see `MEDIA_DETAIL_FIELDS`). Handing
    /// that copy back lets the UI paint the page immediately instead of
    /// showing a skeleton until `by_id` answers.
    ///
    /// Deliberately SYNCHRONOUS: the point is to avoid a request and a wait,
    /// so it must be answerable on the UI thread. It returns `None` when the
    /// work has not been seen, which is the signal to keep the skeleton.
    ///
    /// Defaulted to `None` so a provider that does not keep such a map (and
    /// every test stub) needs no change. The returned value is provisional and
    /// always superseded by [`AnimeProvider::by_id`].
    fn cached_partial(&self, _id: i64) -> Option<Anime> {
        None
    }

    /// A page of community recommendations for a work, highest-rated first.
    ///
    /// Separate from [`AnimeProvider::by_id`] because the detail lookup only
    /// carries the first handful inline; this is what backs the "view more"
    /// view, which pages through the whole set. `page` is 1-based.
    async fn recommendations(
        &self,
        id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<RecommendationsPage, ProviderError>;

    /// The genre names this provider recognises.
    ///
    /// Asked of the provider rather than hardcoded in the UI, so the browse
    /// grid cannot drift from what queries actually accept.
    async fn genres(&self) -> Result<Vec<String>, ProviderError>;

    /// The tags this provider recognises, each with its grouping category.
    ///
    /// Separate from [`AnimeProvider::genres`] because providers model the two
    /// differently: a genre is a flat list of broad categories, a tag is a
    /// larger set that only makes sense grouped. Asked of the provider for the
    /// same reason as genres -- a hardcoded list would drift from what queries
    /// actually accept, and an unrecognised tag filters to nothing rather than
    /// erroring, so the drift would be silent.
    async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError>;

    /// Broadcasts falling within a time window, soonest first.
    ///
    /// `from` and `to` are unix timestamps in seconds. The window is passed in
    /// rather than a "next N hours" count so the caller decides the span and
    /// the provider stays free of clock or timezone assumptions.
    async fn schedule(
        &self,
        from: i64,
        to: i64,
        limit: u32,
    ) -> Result<Vec<ScheduledEpisode>, ProviderError>;
    /// Browse with filters and paging.
    ///
    /// One parameterised method rather than one per filter combination: a
    /// filter is the same query with different arguments, so adding a filter
    /// never changes this trait.
    ///
    /// `page` is 1-based, matching the providers' own numbering. `per_page`
    /// is a request, not a guarantee: providers cap it.
    async fn browse(
        &self,
        query: BrowseQuery,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError>;

    /// Every shelf the home screen shows, in one call.
    ///
    /// The default runs one `list()` per shelf, which is correct but chatty.
    /// A provider whose API accepts several queries in one round trip
    /// (AniList: GraphQL aliases) overrides this so the whole home screen
    /// costs ONE request instead of six -- the difference between tripping the
    /// rate limit on startup and not.
    async fn home_shelves(&self) -> Result<HomeShelves, ProviderError> {
        let trending = self.trending(HOME_TRENDING_LIMIT).await?;
        let top_airing = self.list(ListFilter::TopAiring, HOME_BLOCK_LIMIT).await?;
        let most_popular = self
            .list(ListFilter::MostPopular, HOME_BLOCK_LIMIT)
            .await?;
        let top_rated = self.list(ListFilter::TopRated, HOME_BLOCK_LIMIT).await?;
        let latest_completed = self
            .list(ListFilter::LatestCompleted, HOME_BLOCK_LIMIT)
            .await?;
        let top_100 = self.list(ListFilter::TopRated, HOME_TOP_100_LIMIT).await?;

        Ok(HomeShelves {
            trending,
            top_airing,
            most_popular,
            top_rated,
            latest_completed,
            top_100,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_variant() {
        // Cheap guard that no variant renders as an empty string.
        let cases = [
            ProviderError::Transport("timeout".into()),
            ProviderError::Status {
                status: 429,
                body: "slow down".into(),
            },
            ProviderError::Decode("bad json".into()),
            ProviderError::Remote("invalid query".into()),
        ];

        for case in cases {
            let rendered = case.to_string();
            assert!(!rendered.trim().is_empty(), "empty display for {case:?}");
        }
    }

    #[test]
    fn status_includes_the_code() {
        let err = ProviderError::Status {
            status: 404,
            body: String::new(),
        };
        assert!(err.to_string().contains("404"));
    }

    /// The trait must be usable behind a trait object, since the app stores
    /// providers heterogeneously.
    #[test]
    fn provider_error_is_a_std_error() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<ProviderError>();
    }
}
