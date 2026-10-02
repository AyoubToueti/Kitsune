//! IPC surface exposed to the frontend.
//!
//! Each command is a thin wrapper. The actual work lives in a free function
//! taking `&dyn AnimeProvider`, which keeps the logic testable without
//! standing up a Tauri app (a `State` cannot easily be constructed in a
//! unit test).
//!
//! Errors cross the boundary as `String`. `ProviderError` already renders
//! distinctly per cause, so the frontend gets something actionable rather
//! than a generic failure.

use std::sync::Arc;

use tauri::State;

use crate::providers::{AnimeProvider, JikanProvider, ProviderError};
use crate::types::{
    Anime, AnimePage, BrowseQuery, EpisodeInfo, ListFilter, MediaTag, RecommendationsPage,
    ScheduledEpisode,
};

/// How many results to ask for when the caller does not say.
pub const DEFAULT_LIMIT: u32 = 20;

/// The provider handle managed as Tauri state.
///
/// A trait object so a second provider (TMDB, or a legal streaming catalog)
/// can be added without touching these commands.
pub type SharedProvider = Arc<dyn AnimeProvider>;

/// The Jikan handle managed as Tauri state.
///
/// Separate from [`SharedProvider`]: Jikan is not an [`AnimeProvider`], it only
/// fills in episode lists for works AniList already described.
pub type SharedEpisodeProvider = Arc<JikanProvider>;
/// Resolve a caller-supplied limit.
///
/// Treats `None` and `0` alike as "unset": asking for zero results is
/// almost always a frontend bug rather than an intent, and silently
/// returning an empty list would hide it.
pub fn resolve_limit(requested: Option<u32>) -> u32 {
    match requested {
        Some(0) | None => DEFAULT_LIMIT,
        Some(n) => n,
    }
}

/// Render a provider failure for the frontend.
pub fn to_message(err: ProviderError) -> String {
    err.to_string()
}

/// Trending titles, for the home screen.
pub async fn trending_from(
    provider: &dyn AnimeProvider,
    limit: Option<u32>,
) -> Result<Vec<Anime>, ProviderError> {
    provider.trending(resolve_limit(limit)).await
}

/// A curated list, chosen by intent. Backs the home-screen shelves.
pub async fn list_from(
    provider: &dyn AnimeProvider,
    filter: ListFilter,
    limit: Option<u32>,
) -> Result<Vec<Anime>, ProviderError> {
    provider.list(filter, resolve_limit(limit)).await
}

/// Browse with filters and paging.
///
/// A blank search is dropped rather than sent: the provider would treat it as
/// a filter matching nothing, but the caller means "no text filter".
pub async fn browse_from(
    provider: &dyn AnimeProvider,
    query: BrowseQuery,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<AnimePage, ProviderError> {
    provider
        .browse(query, page.unwrap_or(1).max(1), resolve_limit(per_page))
        .await
}

/// Broadcasts within a time window, soonest first.
pub async fn schedule_from(
    provider: &dyn AnimeProvider,
    from: i64,
    to: i64,
    limit: Option<u32>,
) -> Result<Vec<ScheduledEpisode>, ProviderError> {
    provider.schedule(from, to, resolve_limit(limit)).await
}

/// The genres available for browsing.
pub async fn genres_from(provider: &dyn AnimeProvider) -> Result<Vec<String>, ProviderError> {
    provider.genres().await
}

/// The tags available for filtering, each with its grouping category.
pub async fn tags_from(provider: &dyn AnimeProvider) -> Result<Vec<MediaTag>, ProviderError> {
    provider.tags().await
}

/// Look up a single title. `Ok(None)` means "no such id".
pub async fn anime_from(
    provider: &dyn AnimeProvider,
    id: i64,
) -> Result<Option<Anime>, ProviderError> {
    provider.by_id(id).await
}

/// A page of recommendations for one work, highest-rated first.
pub async fn recommendations_from(
    provider: &dyn AnimeProvider,
    id: i64,
    page: u32,
    per_page: u32,
) -> Result<RecommendationsPage, ProviderError> {
    provider.recommendations(id, page, per_page).await
}

// --- Tauri command wrappers ----------------------------------------------

#[tauri::command]
pub async fn get_trending(
    provider: State<'_, SharedProvider>,
    limit: Option<u32>,
) -> Result<Vec<Anime>, String> {
    trending_from(provider.inner().as_ref(), limit)
        .await
        .map_err(to_message)
}

#[tauri::command]
pub async fn get_browse(
    provider: State<'_, SharedProvider>,
    query: BrowseQuery,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<AnimePage, String> {
    browse_from(provider.inner().as_ref(), query, page, per_page)
        .await
        .map_err(to_message)
}

#[tauri::command]
pub async fn get_list(
    provider: State<'_, SharedProvider>,
    filter: ListFilter,
    limit: Option<u32>,
) -> Result<Vec<Anime>, String> {
    list_from(provider.inner().as_ref(), filter, limit)
        .await
        .map_err(to_message)
}

#[tauri::command]
pub async fn get_genres(provider: State<'_, SharedProvider>) -> Result<Vec<String>, String> {
    genres_from(provider.inner().as_ref())
        .await
        .map_err(to_message)
}

    #[tauri::command]
pub async fn get_tags(provider: State<'_, SharedProvider>) -> Result<Vec<MediaTag>, String> {
    tags_from(provider.inner().as_ref())
        .await
        .map_err(to_message)
}

#[tauri::command]
pub async fn get_schedule(
    provider: State<'_, SharedProvider>,
    from: i64,
    to: i64,
    limit: Option<u32>,
) -> Result<Vec<ScheduledEpisode>, String> {
    schedule_from(provider.inner().as_ref(), from, to, limit)
        .await
        .map_err(to_message)
}

#[tauri::command]
pub async fn get_anime(
    provider: State<'_, SharedProvider>,
    id: i64,
) -> Result<Option<Anime>, String> {
    anime_from(provider.inner().as_ref(), id)
        .await
        .map_err(to_message)
}

#[tauri::command]
pub async fn get_recommendations(
    provider: State<'_, SharedProvider>,
    id: i64,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<RecommendationsPage, String> {
    // `page` defaults to 1 (AniList rejects 0); `per_page` goes through the
    // same resolver as every other list, so an unset value gets the default.
    recommendations_from(
        provider.inner().as_ref(),
        id,
        page.unwrap_or(1),
        resolve_limit(per_page),
    )
    .await
    .map_err(to_message)
}
/// Full episode metadata for a work, via its MyAnimeList id.
///
/// A free function over the provider, mirroring the `*_from` helpers above, so
/// the logic is testable without standing up a Tauri `State`.
pub async fn episodes_from(
    provider: &JikanProvider,
    mal_id: i64,
) -> Result<Vec<EpisodeInfo>, ProviderError> {
    provider.episodes(mal_id).await
}

/// Every episode of a work, by its MyAnimeList id.
///
/// The id comes from AniList's `idMal`. A work AniList has no MAL link for is
/// reported as an error rather than an empty list, so the frontend can tell
/// "no data available" from "this work has no episodes".
#[tauri::command]
pub async fn get_episodes(
    provider: State<'_, SharedEpisodeProvider>,
    mal_id: i64,
) -> Result<Vec<EpisodeInfo>, String> {
    episodes_from(provider.inner().as_ref(), mal_id)
        .await
        .map_err(to_message)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::AniListProvider;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// The command-layer helper forwards to Jikan and maps the payload.
    #[tokio::test]
    async fn episodes_from_maps_a_jikan_page() {
        use crate::providers::JikanProvider;
        use wiremock::matchers::{method, path};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/21/episodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "pagination": { "has_next_page": false },
                "data": [
                    { "mal_id": 1, "title": "I'm Luffy!", "aired": "1999-10-20T00:00:00+00:00",
                      "filler": false, "recap": false }
                ]
            })))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = episodes_from(&provider, 21).await.expect("should succeed");

        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].number, 1);
        assert_eq!(episodes[0].title.as_deref(), Some("I'm Luffy!"));
    }

    /// A paged response, for the browse wrapper.
    fn browse_response() -> serde_json::Value {
        serde_json::json!({
            "data": { "Page": {
                "pageInfo": {
                    "total": 120,
                    "currentPage": 2,
                    "lastPage": 5,
                    "hasNextPage": true,
                },
                "media": [media_json()],
            } }
        })
    }

    ///
    /// Mount a mock that captures the request body.
    ///
    /// The plain `provider_with` ignores the body, so a test asserting what was
    /// sent needs the bytes. Returns the shared cell for inspection.
    async fn provider_capturing(
        status: u16,
    ) -> (
        MockServer,
        AniListProvider,
        std::sync::Arc<std::sync::Mutex<String>>,
    ) {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(status).set_body_json(browse_response())
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        (server, provider, captured)
    }

    /// An omitted page must reach the provider as page 1, not be dropped.
    #[tokio::test]
    async fn browse_from_defaults_to_the_first_page() {
        let (_server, provider, captured) = provider_capturing(200).await;

        browse_from(&provider, BrowseQuery::default(), None, None)
            .await
            .expect("should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(body.contains("\"page\":1"), "body was {body}");
    }

    /// A caller that sends page 0 means the first page, not an error. Asserted
    /// on the wire, because `is_ok()` alone would pass without the clamp.
    #[tokio::test]
    async fn browse_from_clamps_page_zero() {
        let (_server, provider, captured) = provider_capturing(200).await;

        browse_from(&provider, BrowseQuery::default(), Some(0), None)
            .await
            .expect("should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(body.contains("\"page\":1"), "body was {body}");
        assert!(!body.contains("\"page\":0"), "body was {body}");
    }

    /// The wrapper passes the caller's page and size through untouched.
    #[tokio::test]
    async fn browse_from_forwards_an_explicit_page() {
        let (_server, provider, captured) = provider_capturing(200).await;

        browse_from(&provider, BrowseQuery::default(), Some(4), Some(50))
            .await
            .expect("should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(body.contains("\"page\":4"), "body was {body}");
        assert!(body.contains("\"perPage\":50"), "body was {body}");
    }

    /// The wrapper reports the provider's page info rather than inventing it.
    #[tokio::test]
    async fn browse_from_returns_the_provider_page_info() {
        let (_server, provider) = provider_with(browse_response()).await;

        let page = browse_from(&provider, BrowseQuery::default(), Some(2), None)
            .await
            .expect("should succeed");

        assert_eq!(page.page_info.current_page, 2);
        assert_eq!(page.page_info.last_page, 5);
        assert!(page.page_info.has_next_page);
        assert_eq!(page.items.len(), 1);
    }

    /// A provider failure surfaces as an error rather than an empty page, so
    /// the UI can tell "no matches" from "the request failed".
    #[tokio::test]
    async fn browse_from_propagates_errors() {
        let (_server, provider, _captured) = provider_capturing(429).await;

        assert!(
            browse_from(&provider, BrowseQuery::default(), Some(1), None)
                .await
                .is_err()
        );
    }

    /// `schedule_from` forwards the window and applies the default limit.
    #[tokio::test]
    async fn schedule_from_uses_the_default_limit_when_unset() {
        let response = serde_json::json!({
            "data": { "Page": { "airingSchedules": [
                { "airingAt": 1_789_032_600, "episode": 1, "media": media_json() }
            ] } }
        });
        let (_server, provider) = provider_with(response).await;

        let entries = schedule_from(&provider, 1_789_000_000, 1_789_600_000, None)
            .await
            .expect("should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].episode, Some(1));
    }

    /// `list_from` forwards the filter and applies the default limit when the
    /// caller leaves it unset.
    #[tokio::test]
    async fn list_from_uses_the_default_limit_when_unset() {
        let response = serde_json::json!({
            "data": { "Page": { "media": [media_json()] } }
        });
        let (_server, provider) = provider_with(response).await;

        let anime = list_from(&provider, ListFilter::TopRated, None)
            .await
            .expect("should succeed");

        assert_eq!(anime.len(), 1);
        assert_eq!(anime[0].id, 21);
    }

    /// A caller-supplied limit must reach the provider rather than being
    /// silently replaced by the default.
    #[tokio::test]
    async fn list_from_honours_an_explicit_limit() {
        let response = serde_json::json!({
            "data": { "Page": { "media": [media_json()] } }
        });
        let (_server, provider) = provider_with(response).await;

        assert!(list_from(&provider, ListFilter::Upcoming, Some(5))
            .await
            .is_ok());
    }

    /// A minimal valid AniList media payload.
    fn media_json() -> serde_json::Value {
        serde_json::json!({
            "id": 21,
            "title": { "romaji": "One Piece" },
            "episodes": 1100
        })
    }

    async fn provider_with(response: serde_json::Value) -> (MockServer, AniListProvider) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        let provider = AniListProvider::with_endpoint(server.uri());
        (server, provider)
    }

    // --- resolve_limit ---------------------------------------------------

    #[test]
    fn resolve_limit_defaults_when_unset() {
        assert_eq!(resolve_limit(None), DEFAULT_LIMIT);
    }

    #[test]
    fn resolve_limit_treats_zero_as_unset() {
        // Zero would otherwise return an empty list and look like "no
        // results" rather than "bad input".
        assert_eq!(resolve_limit(Some(0)), DEFAULT_LIMIT);
    }

    #[test]
    fn resolve_limit_passes_through_a_real_value() {
        assert_eq!(resolve_limit(Some(5)), 5);
    }

    // --- to_message ------------------------------------------------------

    #[test]
    fn to_message_distinguishes_causes() {
        let transport = to_message(ProviderError::Transport("timeout".into()));
        let status = to_message(ProviderError::Status {
            status: 429,
            body: String::new(),
        });

        assert!(transport.contains("timeout"));
        assert!(status.contains("429"));
        assert_ne!(transport, status);
    }

    // --- trending_from ---------------------------------------------------

    #[tokio::test]
    async fn trending_from_returns_mapped_anime() {
        let response = serde_json::json!({
            "data": { "Page": { "media": [media_json()] } }
        });
        let (_server, provider) = provider_with(response).await;

        let anime = trending_from(&provider, Some(10))
            .await
            .expect("should succeed");

        assert_eq!(anime.len(), 1);
        assert_eq!(anime[0].id, 21);
        assert_eq!(anime[0].episode_count, Some(1100));
    }

    // --- anime_from ------------------------------------------------------

    #[tokio::test]
    async fn anime_from_returns_the_title() {
        let response = serde_json::json!({ "data": { "Media": media_json() } });
        let (_server, provider) = provider_with(response).await;

        let anime = anime_from(&provider, 21).await.expect("should succeed");
        assert_eq!(anime.expect("should be found").id, 21);
    }

    #[tokio::test]
    async fn anime_from_returns_none_for_a_missing_id() {
        let response = serde_json::json!({ "data": { "Media": null } });
        let (_server, provider) = provider_with(response).await;

        let anime = anime_from(&provider, 999).await.expect("should not error");
        assert!(anime.is_none(), "a missing id is not an error");
    }

    // --- tags_from -------------------------------------------------------

    #[tokio::test]
    async fn tags_from_returns_name_and_category() {
        let response = serde_json::json!({
            "data": { "MediaTagCollection": [
                { "name": "Isekai", "category": "Theme-Fantasy", "isAdult": false },
            ] }
        });
        let (_server, provider) = provider_with(response).await;

        let tags = tags_from(&provider).await.expect("should succeed");

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Isekai");
        assert_eq!(tags[0].category, "Theme-Fantasy");
    }

    // --- provider failure surfaces ---------------------------------------

    #[tokio::test]
    async fn provider_failure_becomes_an_error_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        let err = trending_from(&provider, None)
            .await
            .expect_err("should fail");

        let message = to_message(err);
        assert!(
            message.contains("429"),
            "message should name the status: {message}"
        );
    }
}
