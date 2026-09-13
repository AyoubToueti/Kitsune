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
use crate::types::{Anime, ListFilter, ProviderId, ScheduledEpisode, StreamingEpisode, Title};

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
    ///
    /// `extra_vars` declares variables the filter clause references (e.g.
    /// `$search: String`); an empty string is fine when none are needed.
    fn list_query(extra_vars: &str, filter: &str) -> String {
        let all_vars = if extra_vars.is_empty() {
            "$page: Int, $perPage: Int".to_string()
        } else {
            format!("{extra_vars}, $page: Int, $perPage: Int")
        };
        format!(
            r#"
            query ({all_vars}) {{
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

    async fn by_genre(&self, genre: &str, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        // `$genre` must be declared in the operation header or AniList
        // rejects the whole query with "Variable \"$genre\" is not defined."
        let query = Self::list_query(
            "$genre: String",
            "type: ANIME, genre: $genre, sort: POPULARITY_DESC",
        );
        let data: PageData = self
            .query(
                &query,
                serde_json::json!({
                    "genre": genre,
                    "page": 1,
                    "perPage": clamp_limit(limit),
                }),
            )
            .await?;

        Ok(data.page.media.into_iter().map(map_media).collect())
    }

    async fn genres(&self) -> Result<Vec<String>, ProviderError> {
        let data: GenreData = self
            .query("{ GenreCollection }", serde_json::json!({}))
            .await?;

        // AniList reports the adult category alongside the rest. It is not
        // offered for browsing, so drop it here rather than in the UI where
        // every caller would have to remember.
        Ok(data
            .genres
            .into_iter()
            .filter(|genre| !genre.eq_ignore_ascii_case("Hentai"))
            .collect())
    }

    async fn schedule(
        &self,
        from: i64,
        to: i64,
        limit: u32,
    ) -> Result<Vec<ScheduledEpisode>, ProviderError> {
        let query = schedule_query();
        let data: ScheduleData = self
            .query(
                &query,
                serde_json::json!({
                    "from": from,
                    "to": to,
                    "page": 1,
                    "perPage": clamp_limit(limit),
                }),
            )
            .await?;

        // A schedule entry with no media cannot be rendered as a card, and
        // AniList occasionally omits it, so those are dropped rather than
        // surfaced as a broken row.
        Ok(data
            .page
            .airing_schedules
            .into_iter()
            .filter_map(map_scheduled_episode)
            .collect())
    }

    async fn trending(&self, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let query = Self::list_query("", "type: ANIME, sort: TRENDING_DESC");
        let data: PageData = self
            .query(
                &query,
                serde_json::json!({ "page": 1, "perPage": clamp_limit(limit) }),
            )
            .await?;

        Ok(data.page.media.into_iter().map(map_media).collect())
    }
    async fn list(&self, filter: ListFilter, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        // Each shelf is the same Page query with a different filter, so
        // only this mapping is provider-specific.
        let filter_arg = match filter {
            ListFilter::Trending => "type: ANIME, sort: TRENDING_DESC",
            ListFilter::TopAiring => "type: ANIME, status: RELEASING, sort: POPULARITY_DESC",
            ListFilter::MostPopular => "type: ANIME, sort: POPULARITY_DESC",
            ListFilter::TopRated => "type: ANIME, sort: SCORE_DESC",
            ListFilter::LatestCompleted => "type: ANIME, status: FINISHED, sort: START_DATE_DESC",
            ListFilter::Upcoming => "type: ANIME, status: NOT_YET_RELEASED, sort: POPULARITY_DESC",
        };

        let query = Self::list_query("", filter_arg);
        let data: PageData = self
            .query(
                &query,
                serde_json::json!({ "page": 1, "perPage": clamp_limit(limit) }),
            )
            .await?;

        Ok(data.page.media.into_iter().map(map_media).collect())
    }
    async fn search(&self, query: &str, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        let gql = Self::list_query(
            "$search: String",
            "type: ANIME, sort: SEARCH_MATCH, search: $search",
        );
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

/// The window query for broadcasts.
///
/// Written by hand rather than through `list_query`, because
/// `airingSchedules` is a sibling of `media` on `Page`, not a media filter.
fn schedule_query() -> String {
    format!(
        r#"
        query ($from: Int, $to: Int, $page: Int, $perPage: Int) {{
          Page(page: $page, perPage: $perPage) {{
            airingSchedules(
              airingAt_greater: $from
              airingAt_lesser: $to
              sort: TIME
            ) {{
              airingAt
              episode
              media {{ {MEDIA_FIELDS} }}
            }}
          }}
        }}
        "#
    )
}

/// AniList rejects `perPage` above 50.
fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, 50)
}

// --- wire types -----------------------------------------------------------
//
// Kept private: the rest of the app only ever sees `crate::types::Anime`.

/// One `Page.airingSchedules` entry. `media` is the same shape the media
/// queries return, so `MEDIA_FIELDS` and `map_media` are reused as-is.
#[derive(Deserialize)]
struct AiringScheduleWire {
    #[serde(rename = "airingAt")]
    airing_at: i64,
    #[serde(default)]
    episode: Option<u32>,
    #[serde(default)]
    media: Option<Media>,
}

/// A page of schedules, which hangs off `Page` beside `media`.
#[derive(Deserialize)]
struct ScheduleData {
    #[serde(rename = "Page")]
    page: SchedulePage,
}

#[derive(Deserialize)]
struct SchedulePage {
    #[serde(rename = "airingSchedules", default)]
    airing_schedules: Vec<AiringScheduleWire>,
}

/// Map one broadcast, dropping entries with no media to show.
fn map_scheduled_episode(wire: AiringScheduleWire) -> Option<ScheduledEpisode> {
    let anime = wire.media.map(map_media)?;
    Some(ScheduledEpisode {
        anime,
        airing_at: wire.airing_at,
        episode: wire.episode,
    })
}

/// The root `GenreCollection` field, which returns a bare array of names.
#[derive(Deserialize)]
struct GenreData {
    #[serde(rename = "GenreCollection")]
    genres: Vec<String>,
}

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
    /// The root `GenreCollection` payload shape.
    fn genre_response(genres: Vec<&str>) -> serde_json::Value {
        serde_json::json!({ "data": { "GenreCollection": genres } })
    }

    /// One `airingSchedules` entry, reusing the standard media fixture so the
    /// nested shape stays in step with the media queries.
    fn schedule_entry(airing_at: i64, episode: u32) -> serde_json::Value {
        serde_json::json!({
            "airingAt": airing_at,
            "episode": episode,
            "media": media_json(),
        })
    }

    fn schedule_response(entries: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "data": { "Page": { "airingSchedules": entries } }
        })
    }

    /// The window bounds must reach AniList as declared variables; a missing
    /// declaration is the failure mode that broke search and the genre query.
    #[tokio::test]
    async fn schedule_query_declares_its_window_variables() {
        let (_server, provider) =
            provider_expecting("query ($from: Int, $to: Int, $page: Int, $perPage: Int)").await;

        assert!(provider
            .schedule(1_789_000_000, 1_789_600_000, 5)
            .await
            .is_ok());
    }

    /// Ordering must be chronological, otherwise a "next up" list is nonsense.
    #[tokio::test]
    async fn schedule_sorts_by_time() {
        let (_server, provider) = provider_expecting("sort: TIME").await;

        assert!(provider
            .schedule(1_789_000_000, 1_789_600_000, 5)
            .await
            .is_ok());
    }

    /// The window travels as variables rather than being interpolated into the
    /// filter, so the values cannot be mangled by string formatting.
    #[tokio::test]
    async fn schedule_passes_the_window_as_variables() {
        let (_server, provider) = provider_expecting("\"from\":1789000000").await;

        assert!(provider
            .schedule(1_789_000_000, 1_789_600_000, 5)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn schedule_maps_each_entry_to_a_scheduled_episode() {
        let (_server, provider) = provider_with(
            schedule_response(vec![schedule_entry(1_789_032_600, 25)]),
            200,
        )
        .await;

        let entries = provider
            .schedule(1_789_000_000, 1_789_600_000, 5)
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].airing_at, 1_789_032_600);
        assert_eq!(entries[0].episode, Some(25));
        // The nested media is mapped through the same path as a media query.
        assert_eq!(entries[0].anime.id, 21);
        assert_eq!(entries[0].anime.average_score, Some(88));
    }

    /// An entry with no media cannot be rendered as a card, so it is dropped
    /// rather than surfacing as a blank row.
    #[tokio::test]
    async fn schedule_drops_entries_without_media() {
        let mut orphan = schedule_entry(1_789_032_600, 1);
        orphan["media"] = serde_json::Value::Null;

        let (_server, provider) = provider_with(
            schedule_response(vec![orphan, schedule_entry(1_789_036_200, 2)]),
            200,
        )
        .await;

        let entries = provider
            .schedule(1_789_000_000, 1_789_600_000, 5)
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].episode, Some(2));
    }

    /// An entry that states no episode number is still useful; it just cannot
    /// say which episode is airing.
    #[tokio::test]
    async fn schedule_tolerates_a_missing_episode_number() {
        let mut entry = schedule_entry(1_789_032_600, 1);
        entry["episode"] = serde_json::Value::Null;

        let (_server, provider) = provider_with(schedule_response(vec![entry]), 200).await;

        let entries = provider
            .schedule(1_789_000_000, 1_789_600_000, 5)
            .await
            .unwrap();

        assert_eq!(entries[0].episode, None);
        assert_eq!(entries[0].anime.id, 21);
    }

    /// A genre query must declare `$genre` in the operation header or AniList
    /// rejects it with "Variable \"$genre\" is not defined." — the same failure
    /// mode that broke the search query.
    #[tokio::test]
    async fn genre_query_declares_the_genre_variable() {
        let (_server, provider) =
            provider_expecting("query ($genre: String, $page: Int, $perPage: Int)").await;

        assert!(provider.by_genre("Mecha", 3).await.is_ok());
    }

    /// The genre travels as a GraphQL variable rather than being interpolated
    /// into the filter, so a name containing spaces cannot break the query.
    #[tokio::test]
    async fn by_genre_passes_the_genre_as_a_variable() {
        let (_server, provider) = provider_expecting("\"genre\":\"Slice of Life\"").await;

        let anime = provider.by_genre("Slice of Life", 1).await.unwrap();
        assert_eq!(anime.len(), 1);
    }

    #[tokio::test]
    async fn by_genre_maps_anilist_fields_like_trending() {
        let (_server, provider) = provider_expecting("genre: $genre").await;

        let anime = provider.by_genre("Action", 1).await.unwrap();
        assert_eq!(anime[0].id, 21);
        assert_eq!(anime[0].provider, ProviderId::AniList);
        assert_eq!(anime[0].average_score, Some(88));
    }

    #[tokio::test]
    async fn genres_returns_the_collection() {
        let (_server, provider) =
            provider_with(genre_response(vec!["Action", "Mecha", "Romance"]), 200).await;

        let genres = provider.genres().await.expect("genres should succeed");
        assert_eq!(genres, vec!["Action", "Mecha", "Romance"]);
    }

    /// AniList reports the adult category in the same collection as the rest.
    /// It must not reach a browse grid, so it is dropped at the provider
    /// boundary rather than left for every caller to remember.
    #[tokio::test]
    async fn genres_drops_the_adult_category() {
        let (_server, provider) =
            provider_with(genre_response(vec!["Action", "Hentai", "Mecha"]), 200).await;

        let genres = provider.genres().await.unwrap();
        assert_eq!(genres, vec!["Action", "Mecha"]);
        assert!(!genres.iter().any(|g| g.eq_ignore_ascii_case("Hentai")));
    }

    /// Mount a mock that only answers when the request body contains
    /// `expected`, so the assertion is about the query actually sent.
    async fn provider_expecting(expected: &str) -> (MockServer, AniListProvider) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains(expected))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_response(vec![media_json()])),
            )
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        (server, provider)
    }

    /// Each shelf is the same query with a different filter. If a variant were
    /// wired to the wrong AniList argument, the body matcher would not fire and
    /// the request would 404, so this catches a copy-paste slip.
    #[tokio::test]
    async fn each_list_filter_sends_its_own_query() {
        let cases = [
            (ListFilter::Trending, "sort: TRENDING_DESC"),
            (
                ListFilter::TopAiring,
                "status: RELEASING, sort: POPULARITY_DESC",
            ),
            (ListFilter::MostPopular, "sort: POPULARITY_DESC"),
            (ListFilter::TopRated, "sort: SCORE_DESC"),
            (
                ListFilter::LatestCompleted,
                "status: FINISHED, sort: START_DATE_DESC",
            ),
            (
                ListFilter::Upcoming,
                "status: NOT_YET_RELEASED, sort: POPULARITY_DESC",
            ),
        ];

        for (filter, expected) in cases {
            let (_server, provider) = provider_expecting(expected).await;
            let anime = provider
                .list(filter, 5)
                .await
                .unwrap_or_else(|e| panic!("{filter:?} should succeed, got {e:?}"));

            assert_eq!(anime.len(), 1, "{filter:?} should map one entry");
        }
    }

    /// Shelf queries must declare no variable beyond paging. If one did, the
    /// operation header would have to carry it or AniList would 400.
    #[tokio::test]
    async fn list_queries_declare_only_paging_variables() {
        let (_server, provider) = provider_expecting("query ($page: Int, $perPage: Int)").await;

        assert!(provider.list(ListFilter::MostPopular, 3).await.is_ok());
    }

    #[tokio::test]
    async fn list_maps_anilist_fields_like_trending() {
        let (_server, provider) = provider_expecting("sort: SCORE_DESC").await;

        let anime = provider.list(ListFilter::TopRated, 1).await.unwrap();
        let first = &anime[0];

        assert_eq!(first.id, 21);
        assert_eq!(first.provider, ProviderId::AniList);
        assert_eq!(first.title.romaji.as_deref(), Some("One Piece"));
        assert_eq!(first.average_score, Some(88));
    }

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
    use wiremock::matchers::{body_string_contains, method, path};
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

        let anime = provider
            .trending(10)
            .await
            .expect("trending should succeed");
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
        let cover = anime[0]
            .cover_image
            .as_deref()
            .expect("cover should be set");

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

        assert!(
            anime[0].description.is_none(),
            "blank description should drop"
        );
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
            matches!(
                provider.trending(1).await.unwrap_err(),
                ProviderError::Decode(_)
            ),
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

    /// The search query must declare the `$search` variable in the GraphQL
    /// operation header, otherwise AniList rejects it with HTTP 400
    /// "Variable "$search" is not defined."
    #[tokio::test]
    async fn search_query_declares_the_search_variable() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("$search"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_response(vec![media_json()])),
            )
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        let anime = provider
            .search("one piece", 1)
            .await
            .expect("search should succeed");
        assert_eq!(anime.len(), 1);
    }
}
