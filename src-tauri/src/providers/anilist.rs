//! AniList metadata provider.
//!
//! Talks to AniList's GraphQL endpoint. Two details are worth knowing:
//!
//! - GraphQL returns HTTP 200 even for query errors, putting them in an
//!   `errors` array. That array is checked, otherwise a bad query would
//!   look like an empty result set.
//! - `coverImage` offers both `large` and `extraLarge`. Both are requested
//!   and `large` is kept, since that is the right size for a card thumbnail.

use async_trait::async_trait;
use serde::Deserialize;

use super::traits::{AnimeProvider, ProviderError};
use crate::types::{Anime, ProviderId, StreamingEpisode, Title};

/// AniList's public GraphQL endpoint.
pub const ANILIST_ENDPOINT: &str = "https://graphql.anilist.co";

/// Fields shared by every media query, so the mapping code is written once.
const MEDIA_FIELDS: &str = r#"
    bannerImage
    duration
    format
    popularity
    streamingEpisodes { title url site thumbnail }
    id
    title { romaji english native }
    coverImage { large extraLarge }
    description
    episodes
    genres
    averageScore
    status
    seasonYear
"#;

/// An AniList GraphQL client.
pub struct AniListProvider {
    http: reqwest::Client,
    endpoint: String,
}

impl AniListProvider {
    /// Build a client against the real AniList endpoint.
    pub fn new() -> Self {
        Self::with_endpoint(ANILIST_ENDPOINT)
    }

    /// Build a client against an arbitrary endpoint.
    ///
    /// Exists so tests can point at a local `wiremock` server instead of
    /// the internet.
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            endpoint: endpoint.into(),
        }
    }

    /// Execute a GraphQL query and return the `data` payload.
    async fn query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, ProviderError> {
        let body = serde_json::json!({ "query": query, "variables": variables });

        let response = self
            .http
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

        if !status.is_success() {
            return Err(ProviderError::Status {
                status: status.as_u16(),
                body: text,
            });
        }

        // Parse into an envelope first: GraphQL reports query problems with
        // a 200, so the status check above is not enough on its own.
        let envelope: Envelope<T> = serde_json::from_str(&text)
            .map_err(|e| ProviderError::Decode(format!("{e}; body was: {text}")))?;

        if let Some(errors) = envelope.errors {
            if !errors.is_empty() {
                let joined = errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(ProviderError::Remote(joined));
            }
        }

        envelope
            .data
            .ok_or_else(|| ProviderError::Remote("response contained no data".into()))
    }

    /// Build the paged-media query for a filter clause.
    fn list_query(filter: &str) -> String {
        format!(
            r#"
            query ($page: Int, $perPage: Int) {{
              Page(page: $page, perPage: $perPage) {{
                media({filter}) {{
                  {MEDIA_FIELDS}
                }}
              }}
            }}
            "#
        )
    }
}

impl Default for AniListProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AnimeProvider for AniListProvider {
    fn id(&self) -> ProviderId {
        ProviderId::AniList
    }

    async fn trending(&self, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let query = Self::list_query("type: ANIME, sort: TRENDING_DESC");
        let data: PageData = self
            .query(
                &query,
                serde_json::json!({ "page": 1, "perPage": clamp_limit(limit) }),
            )
            .await?;

        Ok(data.page.media.into_iter().map(map_media).collect())
    }

    async fn search(&self, query: &str, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let gql = Self::list_query("type: ANIME, sort: SEARCH_MATCH, search: $search");
        let data: PageData = self
            .query(
                &gql,
                serde_json::json!({
                    "search": query,
                    "page": 1,
                    "perPage": clamp_limit(limit),
                }),
            )
            .await?;

        Ok(data.page.media.into_iter().map(map_media).collect())
    }

    async fn by_id(&self, id: i64) -> Result<Option<Anime>, ProviderError> {
        let query = format!(
            r#"
            query ($id: Int) {{
              Media(id: $id, type: ANIME) {{
                {MEDIA_FIELDS}
              }}
            }}
            "#
        );

        let data: MediaData = self.query(&query, serde_json::json!({ "id": id })).await?;
        Ok(data.media.map(map_media))
    }
}

/// AniList rejects `perPage` above 50.
fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, 50)
}

// --- wire types -----------------------------------------------------------
//
// Kept private: the rest of the app only ever sees `crate::types::Anime`.

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Deserialize)]
struct PageData {
    #[serde(rename = "Page")]
    page: Page,
}

#[derive(Deserialize)]
struct Page {
    media: Vec<Media>,
}

#[derive(Deserialize)]
struct MediaData {
    #[serde(rename = "Media")]
    media: Option<Media>,
}

#[derive(Deserialize)]
struct Media {
    #[serde(rename = "bannerImage", default)]
    banner_image: Option<String>,
    #[serde(default)]
    duration: Option<u32>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    popularity: Option<u32>,
    #[serde(rename = "streamingEpisodes", default)]
    streaming_episodes: Option<Vec<StreamingEpisodeWire>>,
    id: i64,
    #[serde(default)]
    title: Option<MediaTitle>,
    #[serde(rename = "coverImage", default)]
    cover_image: Option<CoverImage>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    episodes: Option<u32>,
    #[serde(default)]
    genres: Option<Vec<String>>,
    #[serde(rename = "averageScore", default)]
    average_score: Option<u8>,
    #[serde(default)]
    status: Option<String>,
    #[serde(rename = "seasonYear", default)]
    season_year: Option<u32>,
}

#[derive(Deserialize, Default)]
struct MediaTitle {
    #[serde(default)]
    romaji: Option<String>,
    #[serde(default)]
    english: Option<String>,
    #[serde(default)]
    native: Option<String>,
}

#[derive(Deserialize)]
struct StreamingEpisodeWire {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    site: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>,
}

#[derive(Deserialize)]
struct CoverImage {
    #[serde(default)]
    large: Option<String>,
    #[serde(rename = "extraLarge", default)]
    extra_large: Option<String>,
}

/// Map AniList's shape onto our domain type.
fn map_media(media: Media) -> Anime {
    let title = media.title.unwrap_or_default();

    Anime {
        id: media.id,
        provider: ProviderId::AniList,
        title: Title {
            romaji: non_empty(title.romaji),
            english: non_empty(title.english),
            native: non_empty(title.native),
            user_preferred: None,
        },
        cover_image: media.cover_image.and_then(pick_cover),
        banner_image: non_empty(media.banner_image),
        description: non_empty(media.description),
        episode_count: media.episodes,
        duration_minutes: media.duration,
        format: non_empty(media.format),
        genres: media.genres.unwrap_or_default(),
        average_score: media.average_score,
        popularity: media.popularity,
        status: non_empty(media.status),
        season_year: media.season_year,
        streaming_episodes: media
            .streaming_episodes
            .unwrap_or_default()
            .into_iter()
            .filter_map(map_streaming_episode)
            .collect(),
    }
}

/// Map one streaming link, dropping entries that have no URL.
///
/// AniList occasionally returns an entry without one; a link to nowhere is
/// worse than no link, so it is skipped rather than surfaced.
fn map_streaming_episode(episode: StreamingEpisodeWire) -> Option<StreamingEpisode> {
    let url = non_empty(episode.url)?;
    Some(StreamingEpisode {
        title: non_empty(episode.title),
        url,
        site: non_empty(episode.site),
        thumbnail: non_empty(episode.thumbnail),
    })
}

/// Choose the thumbnail-sized cover.
///
/// Prefers `large` over `extraLarge`: at card size the extra pixels are not
/// visible, and they cost several times the bandwidth and memory. Falls back
/// to `extraLarge` for the rare entry that has no `large`.
fn pick_cover(cover: CoverImage) -> Option<String> {
    cover.large.or(cover.extra_large)
}

/// Treat blank strings as absent, since AniList sometimes returns "".
fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    /// The base fixture plus the newer fields: banner, format, duration,
    /// popularity, and one streaming link.
    ///
    /// Built by augmenting `media_json` rather than duplicating it, so the
    /// two cannot drift apart.
    fn media_json_with_extras() -> serde_json::Value {
        let mut media = media_json();
        media["bannerImage"] = serde_json::json!("https://example.test/banner.jpg");
        media["duration"] = serde_json::json!(24);
        media["format"] = serde_json::json!("TV");
        media["popularity"] = serde_json::json!(250000);
        media["streamingEpisodes"] = serde_json::json!([
            {
                "title": "Episode 1",
                "url": "https://crunchyroll.test/one-piece/1",
                "site": "Crunchyroll",
                "thumbnail": "https://example.test/thumb.jpg"
            }
        ]);
        media
    }

    #[tokio::test]
    async fn new_metadata_fields_are_mapped() {
        let (_server, provider) =
            provider_with(page_response(vec![media_json_with_extras()]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        let first = &anime[0];

        assert_eq!(
            first.banner_image.as_deref(),
            Some("https://example.test/banner.jpg")
        );
        assert_eq!(first.duration_minutes, Some(24));
        assert_eq!(first.format.as_deref(), Some("TV"));
        assert_eq!(first.popularity, Some(250_000));
    }

    #[tokio::test]
    async fn streaming_episodes_are_mapped() {
        let (_server, provider) =
            provider_with(page_response(vec![media_json_with_extras()]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        let links = &anime[0].streaming_episodes;

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].site.as_deref(), Some("Crunchyroll"));
        assert_eq!(links[0].url, "https://crunchyroll.test/one-piece/1");
        assert_eq!(links[0].title.as_deref(), Some("Episode 1"));
    }

    /// An entry with no URL is useless, so it is dropped rather than shown.
    #[tokio::test]
    async fn streaming_episode_without_url_is_dropped() {
        let mut media = media_json();
        media["streamingEpisodes"] = serde_json::json!([
            { "title": "No URL", "site": "Crunchyroll" },
            { "title": "Good", "url": "https://crunchyroll.test/ok", "site": "Crunchyroll" }
        ]);

        let (_server, provider) = provider_with(page_response(vec![media]), 200).await;
        let anime = provider.trending(1).await.unwrap();

        assert_eq!(anime[0].streaming_episodes.len(), 1);
        assert_eq!(
            anime[0].streaming_episodes[0].url,
            "https://crunchyroll.test/ok"
        );
    }

    /// A provider that omits every newer field must still map cleanly.
    #[tokio::test]
    async fn absent_extras_default_cleanly() {
        let (_server, provider) = provider_with(page_response(vec![media_json()]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        let first = &anime[0];

        assert!(first.banner_image.is_none());
        assert!(first.format.is_none());
        assert!(first.duration_minutes.is_none());
        assert!(first.popularity.is_none());
        assert!(first.streaming_episodes.is_empty());
    }
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A payload shaped like AniList's, trimmed to the fields we request.
    fn media_json() -> serde_json::Value {
        serde_json::json!({
            "id": 21,
            "title": {
                "romaji": "One Piece",
                "english": "One Piece",
                "native": "One Piece (JP)"
            },
            "coverImage": {
                "large": "https://example.test/cover/large/bx21.jpg",
                "extraLarge": "https://example.test/cover/large/bx21-xl.jpg"
            },
            "description": "A pirate adventure.",
            "episodes": 1100,
            "genres": ["Action", "Adventure"],
            "averageScore": 88,
            "status": "RELEASING",
            "seasonYear": 1999
        })
    }

    fn page_response(media: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({ "data": { "Page": { "media": media } } })
    }

    /// Mount a mock AniList and point a provider at it.
    async fn provider_with(
        response: serde_json::Value,
        status: u16,
    ) -> (MockServer, AniListProvider) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(status).set_body_json(response))
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        (server, provider)
    }

    #[tokio::test]
    async fn trending_maps_anilist_fields_to_anime() {
        let (_server, provider) = provider_with(page_response(vec![media_json()]), 200).await;

        let anime = provider.trending(10).await.expect("trending should succeed");
        assert_eq!(anime.len(), 1);

        let first = &anime[0];
        assert_eq!(first.id, 21);
        assert_eq!(first.provider, ProviderId::AniList);
        assert_eq!(first.title.romaji.as_deref(), Some("One Piece"));
        assert_eq!(first.title.english.as_deref(), Some("One Piece"));
        assert_eq!(first.episode_count, Some(1100));
        assert_eq!(
            first.genres,
            vec!["Action".to_string(), "Adventure".to_string()]
        );
        assert_eq!(first.average_score, Some(88));
        assert_eq!(first.status.as_deref(), Some("RELEASING"));
        assert_eq!(first.season_year, Some(1999));
    }

    #[tokio::test]
    async fn cover_prefers_the_card_sized_variant() {
        let (_server, provider) = provider_with(page_response(vec![media_json()]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        let cover = anime[0].cover_image.as_deref().expect("cover should be set");

        assert!(
            cover.ends_with("bx21.jpg"),
            "should keep the `large` variant, got {cover}"
        );
        assert!(!cover.contains("xl"), "should not pick extraLarge");
    }

    #[tokio::test]
    async fn cover_falls_back_to_extra_large() {
        let mut media = media_json();
        media["coverImage"] = serde_json::json!({ "extraLarge": "https://example.test/xl.jpg" });

        let (_server, provider) = provider_with(page_response(vec![media]), 200).await;
        let anime = provider.trending(1).await.unwrap();

        assert_eq!(
            anime[0].cover_image.as_deref(),
            Some("https://example.test/xl.jpg")
        );
    }

    #[tokio::test]
    async fn blank_strings_are_treated_as_absent() {
        let mut media = media_json();
        media["description"] = serde_json::json!("   ");
        media["status"] = serde_json::json!("");

        let (_server, provider) = provider_with(page_response(vec![media]), 200).await;
        let anime = provider.trending(1).await.unwrap();

        assert!(anime[0].description.is_none(), "blank description should drop");
        assert!(anime[0].status.is_none(), "blank status should drop");
    }

    #[tokio::test]
    async fn sparse_media_still_maps() {
        // Only the required `id` is present; everything else is omitted.
        let media = serde_json::json!({ "id": 5 });
        let (_server, provider) = provider_with(page_response(vec![media]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        assert_eq!(anime[0].id, 5);
        assert!(anime[0].cover_image.is_none());
        assert!(anime[0].genres.is_empty());
        assert_eq!(anime[0].episode_count, None);
    }

    #[tokio::test]
    async fn by_id_returns_the_anime() {
        let response = serde_json::json!({ "data": { "Media": media_json() } });
        let (_server, provider) = provider_with(response, 200).await;

        let anime = provider.by_id(21).await.unwrap().expect("should be found");
        assert_eq!(anime.id, 21);
    }

    /// A missing title is not an error, so `by_id` must report `None`
    /// rather than failing the call.
    #[tokio::test]
    async fn by_id_returns_none_when_media_is_null() {
        let response = serde_json::json!({ "data": { "Media": null } });
        let (_server, provider) = provider_with(response, 200).await;

        let result = provider.by_id(999).await.expect("should not error");
        assert!(result.is_none());
    }

    /// GraphQL reports query errors with HTTP 200. Without checking the
    /// `errors` array, a broken query would look like an empty result set.
    #[tokio::test]
    async fn graphql_errors_become_remote_errors() {
        let response = serde_json::json!({
            "data": null,
            "errors": [{ "message": "Invalid query" }]
        });
        let (_server, provider) = provider_with(response, 200).await;

        match provider.trending(1).await.unwrap_err() {
            ProviderError::Remote(msg) => assert!(msg.contains("Invalid query")),
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn http_error_status_becomes_status_error() {
        let response = serde_json::json!({ "message": "too many requests" });
        let (_server, provider) = provider_with(response, 429).await;

        match provider.trending(1).await.unwrap_err() {
            ProviderError::Status { status, .. } => assert_eq!(status, 429),
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn non_json_body_becomes_decode_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html>nope</html>"))
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        assert!(
            matches!(provider.trending(1).await.unwrap_err(), ProviderError::Decode(_)),
            "expected Decode error"
        );
    }

    #[test]
    fn clamp_limit_keeps_requests_within_anilist_bounds() {
        assert_eq!(clamp_limit(0), 1, "zero is not a valid perPage");
        assert_eq!(clamp_limit(10), 10);
        assert_eq!(clamp_limit(50), 50);
        assert_eq!(clamp_limit(500), 50, "AniList rejects perPage above 50");
    }

    #[test]
    fn provider_reports_its_own_id() {
        assert_eq!(AniListProvider::new().id(), ProviderId::AniList);
    }
}