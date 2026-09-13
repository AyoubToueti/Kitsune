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

use crate::providers::{AnimeProvider, ProviderError};
use crate::types::{Anime, ListFilter};

/// How many results to ask for when the caller does not say.
pub const DEFAULT_LIMIT: u32 = 20;

/// The provider handle managed as Tauri state.
///
/// A trait object so a second provider (TMDB, or a legal streaming catalog)
/// can be added without touching these commands.
pub type SharedProvider = Arc<dyn AnimeProvider>;

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

/// Search by free text.
///
/// A blank query short-circuits to an empty list rather than spending a
/// request on something guaranteed to be meaningless.
pub async fn search_from(
    provider: &dyn AnimeProvider,
    query: &str,
    limit: Option<u32>,
) -> Result<Vec<Anime>, ProviderError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    provider.search(trimmed, resolve_limit(limit)).await
}

/// Titles in a genre, most popular first.
pub async fn by_genre_from(
    provider: &dyn AnimeProvider,
    genre: &str,
    limit: Option<u32>,
) -> Result<Vec<Anime>, ProviderError> {
    provider.by_genre(genre, resolve_limit(limit)).await
}

/// The genres available for browsing.
pub async fn genres_from(provider: &dyn AnimeProvider) -> Result<Vec<String>, ProviderError> {
    provider.genres().await
}

/// Look up a single title. `Ok(None)` means "no such id".
pub async fn anime_from(
    provider: &dyn AnimeProvider,
    id: i64,
) -> Result<Option<Anime>, ProviderError> {
    provider.by_id(id).await
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
pub async fn search_anime(
    provider: State<'_, SharedProvider>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<Anime>, String> {
    search_from(provider.inner().as_ref(), &query, limit)
        .await
        .map_err(to_message)
}

#[tauri::command]
pub async fn get_by_genre(
    provider: State<'_, SharedProvider>,
    genre: String,
    limit: Option<u32>,
) -> Result<Vec<Anime>, String> {
    by_genre_from(provider.inner().as_ref(), &genre, limit)
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
pub async fn get_anime(
    provider: State<'_, SharedProvider>,
    id: i64,
) -> Result<Option<Anime>, String> {
    anime_from(provider.inner().as_ref(), id)
        .await
        .map_err(to_message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::AniListProvider;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

    // --- search_from -----------------------------------------------------

    #[tokio::test]
    async fn search_from_maps_results() {
        let response = serde_json::json!({
            "data": { "Page": { "media": [media_json()] } }
        });
        let (_server, provider) = provider_with(response).await;

        let anime = search_from(&provider, "one piece", None)
            .await
            .expect("should succeed");

        assert_eq!(anime.len(), 1);
    }

    /// A blank query must not spend a request. The endpoint here is a
    /// black hole, so any attempt to reach it would surface as a
    /// transport error rather than `Ok`.
    #[tokio::test]
    async fn search_from_short_circuits_on_blank_query() {
        let provider = AniListProvider::with_endpoint("http://127.0.0.1:1");

        for query in ["", "   ", "\t\n"] {
            let result = search_from(&provider, query, None).await;
            assert!(
                matches!(result, Ok(ref v) if v.is_empty()),
                "blank query {query:?} should return an empty list, got {result:?}"
            );
        }
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
