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
