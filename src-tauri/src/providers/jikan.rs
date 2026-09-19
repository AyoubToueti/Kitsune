//! Jikan episode lists.
//!
//! AniList is the primary metadata source, but its `streamingEpisodes` is a list
//! of licensed links, not an episode list: it is empty for many works and capped
//! by the provider for long runners. Jikan (an unofficial MyAnimeList API) has
//! the full per-episode catalogue, and the two sources are bridged by AniList's
//! own `idMal`.
//!
//! Deliberately NOT an [`crate::providers::AnimeProvider`]. It does not describe
//! works -- only fills in the episode list for a work AniList already described.
//! Implementing the whole trait would duplicate AniList for every field and
//! inherit MAL's weaker anime data for the trouble.
//!
//! Two Jikan facts shape this module:
//!
//! - The list is paginated, 100 entries per page, with `has_next_page` on the
//!   envelope. A long runner therefore takes several requests.
//! - It rate-limits (3/second, 60/minute) and answers 429 when exceeded. Pages
//!   are paced and a 429 is retried rather than surfaced, because a truncated
//!   episode list is worse than a slightly slower one.

use std::time::Duration;

use serde::Deserialize;

use super::traits::ProviderError;
use crate::types::EpisodeInfo;

/// Jikan's public REST endpoint.
pub const JIKAN_ENDPOINT: &str = "https://api.jikan.moe/v4";

/// Pause between page requests.
///
/// Jikan allows 3 requests/second, so 400ms keeps a long runner safely under the
/// limit without hand-rolling a token bucket. The cost is real but small: a
/// 1000-episode series takes 10 pages, so about four seconds.
const PAGE_DELAY: Duration = Duration::from_millis(400);

/// How long to wait before retrying a rate-limited page.
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// How many times a single page is retried on a 429 before giving up.
const MAX_RETRIES: u32 = 3;

/// Stop paging after this many pages.
///
/// A guard against a malformed `has_next_page` looping forever. At 100 entries a
/// page this is 5000 episodes, an order of magnitude beyond the longest runner,
/// so it can only ever fire on bad data.
const MAX_PAGES: u32 = 50;

/// A Jikan REST client, used only for episode lists.
pub struct JikanProvider {
    http: reqwest::Client,
    endpoint: String,
}

impl JikanProvider {
    /// Build a client against the real Jikan endpoint.
    pub fn new() -> Self {
        Self::with_endpoint(JIKAN_ENDPOINT)
    }

    /// Build a client against an arbitrary endpoint, so tests can point at a
    /// local `wiremock` server.
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            endpoint: endpoint.into(),
        }
    }

    /// Every episode of a work, in the provider's own order.
    ///
    /// Walks the pagination until `has_next_page` is false, or the page guard
    /// trips. A work with no episode data yields an empty list, which the caller
    /// renders the same way as "unknown".
    pub async fn episodes(&self, mal_id: i64) -> Result<Vec<EpisodeInfo>, ProviderError> {
        let mut all = Vec::new();
        let mut page = 1;

        loop {
            let envelope = self.episodes_page(mal_id, page).await?;
            all.extend(envelope.data.into_iter().filter_map(map_episode));

            let more = envelope.pagination.has_next_page && page < MAX_PAGES;
            if !more {
                break;
            }

            page += 1;
            tokio::time::sleep(PAGE_DELAY).await;
        }

        Ok(all)
    }

    /// Fetch one page, retrying a rate-limit rejection.
    ///
    /// Split from [`JikanProvider::episodes`] so the retry lives in exactly one
    /// place and the paging loop stays readable.
    async fn episodes_page(
        &self,
        mal_id: i64,
        page: u32,
    ) -> Result<EpisodeEnvelope, ProviderError> {
        let url = format!("{}/anime/{mal_id}/episodes?page={page}", self.endpoint);
        let mut attempt = 0;

        loop {
            let response = self
                .http
                .get(&url)
                .send()
                .await
                .map_err(|e| ProviderError::Transport(e.to_string()))?;

            let status = response.status();

            // 429 is the one status worth retrying: it means "later", not "no".
            // Every other failure is reported as-is so the caller is not left
            // waiting on an error that will never clear.
            if status.as_u16() == 429 && attempt < MAX_RETRIES {
                attempt += 1;
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }

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

            return serde_json::from_str(&text).map_err(|e| {
                ProviderError::Decode(format!("{e}; body was: {text}"))
            });
        }
    }
}

impl Default for JikanProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// The paging half of an episodes response.
#[derive(Deserialize)]
struct EpisodeEnvelope {
    pagination: Pagination,
    #[serde(default)]
    data: Vec<EpisodeWire>,
}

#[derive(Deserialize)]
struct Pagination {
    #[serde(rename = "has_next_page", default)]
    has_next_page: bool,
}

/// One episode as Jikan reports it.
#[derive(Deserialize)]
struct EpisodeWire {
    /// Jikan numbers episodes 1-based in this field, which is the number the
    /// rest of the app keys on.
    #[serde(rename = "mal_id")]
    number: Option<u32>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    aired: Option<String>,
    #[serde(default)]
    filler: bool,
    #[serde(default)]
    recap: bool,
}

/// Map one entry, dropping anything without a number.
///
/// An entry with no number cannot be placed in the list or matched against a
/// release, so it is skipped rather than guessed at.
fn map_episode(wire: EpisodeWire) -> Option<EpisodeInfo> {
    let number = wire.number?;

    Some(EpisodeInfo {
        number,
        title: non_empty(wire.title),
        aired: non_empty(wire.aired),
        filler: wire.filler,
        recap: wire.recap,
    })
}

/// Treat blank strings as absent, matching how the AniList provider handles
/// them: Jikan returns `""` where it has nothing to say.
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
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// One page of episodes, with the paging flags the caller asked for.
    fn page(data: serde_json::Value, has_next: bool) -> serde_json::Value {
        serde_json::json!({
            "pagination": { "last_visible_page": 2, "has_next_page": has_next },
            "data": data,
        })
    }

    fn episode_json(number: u32) -> serde_json::Value {
        serde_json::json!({
            "mal_id": number,
            "url": format!("https://myanimelist.net/anime/20/episode/{number}"),
            "title": format!("Episode {number}"),
            "aired": "2002-10-03T00:00:00+00:00",
            "filler": false,
            "recap": false,
        })
    }

    #[tokio::test]
    async fn episodes_are_mapped() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([episode_json(1), episode_json(2)]),
                false,
            )))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = provider.episodes(20).await.unwrap();

        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0].number, 1);
        assert_eq!(episodes[1].title.as_deref(), Some("Episode 2"));
        assert_eq!(episodes[0].aired.as_deref(), Some("2002-10-03T00:00:00+00:00"));
        assert!(!episodes[0].filler);
    }

    #[tokio::test]
    async fn filler_and_recap_are_carried() {
        let server = MockServer::start().await;
        let mut ep = episode_json(97);
        ep["filler"] = serde_json::json!(true);
        ep["recap"] = serde_json::json!(true);
        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([ep]),
                false,
            )))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = provider.episodes(20).await.unwrap();

        assert!(episodes[0].filler);
        assert!(episodes[0].recap);
    }

    #[tokio::test]
    async fn paging_follows_has_next_page() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([episode_json(1)]),
                true,
            )))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([episode_json(2)]),
                false,
            )))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = provider.episodes(20).await.unwrap();

        // Both pages contributed, and the loop stopped at the end rather than
        // walking off into repeated requests.
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[1].number, 2);
    }

    #[tokio::test]
    async fn an_entry_without_a_number_is_dropped() {
        let server = MockServer::start().await;
        let mut orphan = episode_json(9);
        orphan["mal_id"] = serde_json::Value::Null;
        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([episode_json(1), orphan]),
                false,
            )))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = provider.episodes(20).await.unwrap();

        assert_eq!(episodes.len(), 1, "the numberless entry should be skipped");
    }

    #[tokio::test]
    async fn a_blank_title_is_treated_as_absent() {
        let server = MockServer::start().await;
        let mut ep = episode_json(1);
        ep["title"] = serde_json::json!("   ");
        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([ep]),
                false,
            )))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = provider.episodes(20).await.unwrap();

        assert_eq!(episodes[0].title, None);
    }

    #[tokio::test]
    async fn a_rate_limit_is_retried_then_succeeds() {
        let server = MockServer::start().await;

        // First answer is a 429, second is the real page. Wiremock serves
        // mounted mocks in order, and an `up_to_n_times` mock lets the first
        // one be consumed once.
        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([episode_json(1)]),
                false,
            )))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = provider.episodes(20).await.unwrap();

        assert_eq!(episodes.len(), 1, "the retry should have succeeded");
    }

    #[tokio::test]
    async fn a_persistent_rate_limit_is_reported() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let err = provider.episodes(20).await.unwrap_err();

        // After the retries are exhausted the 429 is surfaced rather than
        // swallowed, so the frontend can say "try again" instead of showing an
        // empty episode list.
        match err {
            ProviderError::Status { status, .. } => assert_eq!(status, 429),
            other => panic!("expected a status error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_not_found_work_is_reported() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/999999/episodes"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let err = provider.episodes(999_999).await.unwrap_err();

        match err {
            ProviderError::Status { status, .. } => assert_eq!(status, 404),
            other => panic!("expected a status error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_work_with_no_episodes_yields_an_empty_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/20/episodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([]),
                false,
            )))
            .mount(&server)
            .await;

        let provider = JikanProvider::with_endpoint(server.uri());
        let episodes = provider.episodes(20).await.unwrap();

        assert!(episodes.is_empty());
    }
}