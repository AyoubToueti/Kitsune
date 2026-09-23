//! AniList metadata provider.
//!
//! Talks to AniList's GraphQL endpoint. Two details are worth knowing:
//!
//! - GraphQL returns HTTP 200 even for query errors, putting them in an
//!   `errors` array. That array is checked, otherwise a bad query would
//!   look like an empty result set.
//! - `coverImage` offers both `large` and `extraLarge`. Both are requested
//!   and `large` is kept, since that is the right size for a card thumbnail.

use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::traits::{AnimeProvider, ProviderError};
use crate::types::{
    Anime, AnimePage, BrowseQuery, FormatFilter, ListFilter, ListStatus, MediaTag, PageInfo,
    ProviderId, RecommendedAnime, RelatedAnime, ScheduledEpisode, SeasonFilter, SortOption,
    StatusFilter, StreamingEpisode, Title, Trailer,
};

/// AniList's public GraphQL endpoint.
pub const ANILIST_ENDPOINT: &str = "https://graphql.anilist.co";

/// How strongly a work must carry a tag before the tag counts as a match.
///
/// Weak taggings are noise: one user tagging an unrelated work is enough to
/// match it, which is why an unfiltered `tag_in: [Isekai]` surfaced BLEACH.
/// AniList's own UI exposes this as "minimum tag percentage", defaulting to
/// 18, but 40 is where results actually stabilise -- measured: 40 and 60
/// returned the same set while 18 still let the noise through.
///
/// Verified to govern `tag_not_in` as well as `tag_in`: excluding a tag only
/// drops works carrying it at or above this rank, so a work with a weaker
/// tagging survives the exclusion. One floor for both directions keeps the
/// two symmetric -- below-floor taggings are treated as noise either way.
const MINIMUM_TAG_RANK: u32 = 40;

/// Fields shared by every media query, so the mapping code is written once.
const MEDIA_FIELDS: &str = r#"
    bannerImage
    duration
    format
    popularity
    streamingEpisodes { title url site thumbnail }
    idMal
    id
    title { romaji english native }
    coverImage { large extraLarge }
    description
    episodes
    genres
    averageScore
    status
    season
    seasonYear
"#;

/// Detail-only fields, added on top of [`MEDIA_FIELDS`] for the single-title
/// lookup.
///
/// Kept apart because they are expensive and only the detail page can use them:
/// `relations` is a graph traversal, `recommendations` runs its own paged
/// sub-query, and a card has nowhere to render either. Sending them on every
/// list query would multiply AniList's work for data the UI throws away.
///
/// The fields inside `mediaRecommendation` are chosen to match what
/// `HoverPreview.svelte` renders, because the detail page's Recommended row
/// shows that panel on hover. They are NOT the whole of [`MEDIA_FIELDS`]:
/// that includes `streamingEpisodes`, the licensed-link list, which would
/// bloat the payload ten times over (one per recommendation) for data the
/// panel never shows. A field the panel reads but this list omits renders as a
/// blank line rather than as an error, which is how the panel came to be
/// nearly empty before.
const MEDIA_DETAIL_FIELDS: &str = r#"
    relations {
      edges {
        relationType
        node {
          id
          type
          format
          status
          episodes
          title { romaji english }
          coverImage { large }
        }
      }
    }
    recommendations(perPage: 10, sort: RATING_DESC) {
      edges {
        node {
          rating
          mediaRecommendation {
            id
            type
            title { romaji english native }
            coverImage { large }
            description
            episodes
            duration
            format
            genres
            averageScore
            status
            seasonYear
          }
        }
      }
    }
    trailer { id site thumbnail }
"#;

/// An AniList GraphQL client.
pub struct AniListProvider {
    http: reqwest::Client,
    endpoint: String,
    /// The reader's access token, when signed in.
    ///
    /// `None` means every request is anonymous, which is what the app did
    /// before sign-in existed. A `Mutex` rather than an atomic because the
    /// value is a `String`; the guard is only ever held long enough to clone
    /// the token, never across an await.
    token: Mutex<Option<String>>,
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
            token: Mutex::new(None),
        }
    }

    /// Start sending `token` as a bearer credential.
    ///
    /// A blank token is discarded rather than stored: it is not a credential,
    /// and sending `Bearer ` would turn every request into a 401 instead of
    /// leaving it anonymous.
    pub fn set_token(&self, token: Option<String>) {
        let cleaned = token
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());

        *self.token.lock().expect("token mutex poisoned") = cleaned;
    }

    /// Stop sending a token. Every later request is anonymous.
    pub fn clear_token(&self) {
        self.set_token(None);
    }

    /// Whether a token is currently held.
    pub fn has_token(&self) -> bool {
        self.token.lock().expect("token mutex poisoned").is_some()
    }

    /// The token to send with a request, cloned out of the lock.
    ///
    /// Cloned rather than borrowed so the guard is released before the caller
    /// awaits: holding a `std::sync::Mutex` across an await is how a deadlock
    /// gets built.
    fn current_token(&self) -> Option<String> {
        self.token.lock().expect("token mutex poisoned").clone()
    }

    /// Execute a GraphQL query and return the `data` payload.
    ///
    /// Anonymous unless a token is held, which is how every call worked before
    /// sign-in existed.
    async fn query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, ProviderError> {
        self.send(query, variables).await
    }

    /// Execute a GraphQL mutation and return the `data` payload.
    ///
    /// Kept as its own wrapper rather than folded into `query` so call sites
    /// read as what they are. The document still carries the `mutation`
    /// keyword; that keyword is what makes AniList treat it as a mutation, not
    /// the key it travels under.
    async fn mutate<T: for<'de> Deserialize<'de>>(
        &self,
        mutation: &str,
        variables: serde_json::Value,
    ) -> Result<T, ProviderError> {
        self.send(mutation, variables).await
    }

    /// Send one GraphQL operation and return the `data` payload.
    ///
    /// Split from [`Self::query`] so the transport -- the bearer header, the
    /// status check, the `errors` array -- lives in exactly one place. A
    /// mutation with its own copy could drift and report a rejected write as
    /// success, which is the failure this shape prevents.
    async fn send<T: for<'de> Deserialize<'de>>(
        &self,
        document: &str,
        variables: serde_json::Value,
    ) -> Result<T, ProviderError> {
        // The document ALWAYS travels under the `query` key, even for a
        // mutation. AniList reads the operation from `query` and takes the
        // kind from the keyword inside the string; sending a mutation under a
        // `mutation` key is answered with "No query or mutation provided".
        let body = serde_json::json!({
            "query": document,
            "variables": variables,
        });

        let mut request = self.http.post(&self.endpoint).json(&body);

        // Cloned out of the lock before the await: holding a `std` Mutex
        // across one is how a deadlock gets built.
        if let Some(token) = self.current_token() {
            request = request.bearer_auth(token);
        }

        let response = request
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
    async fn browse(
        &self,
        query: BrowseQuery,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        // The filter clause and the variable declarations are built together,
        // so an argument is only emitted when a variable backs it. Every value
        // travels as a GraphQL variable -- never interpolated -- which is what
        // makes the enum filters safe to drive from user input.
        let mut filter = String::from("type: ANIME");
        let mut declarations: Vec<&str> = Vec::new();
        let mut variables = serde_json::Map::new();

        if let Some(search) = non_empty(query.search) {
            filter.push_str(", search: $search");
            declarations.push("$search: String");
            variables.insert("search".into(), serde_json::json!(search));
        }

        if !query.genres.is_empty() {
            filter.push_str(", genre_in: $genres");
            declarations.push("$genres: [String]");
            variables.insert("genres".into(), serde_json::json!(query.genres));
        }

        if let Some(format) = query.format {
            filter.push_str(", format: $format");
            declarations.push("$format: MediaFormat");
            variables.insert("format".into(), serde_json::json!(format_literal(format)));
        }

        if !query.tags.is_empty() {
            filter.push_str(", tag_in: $tags");
            declarations.push("$tags: [String]");
            variables.insert("tags".into(), serde_json::json!(query.tags));
        }

        if !query.excluded_tags.is_empty() {
            filter.push_str(", tag_not_in: $excludedTags");
            declarations.push("$excludedTags: [String]");
            variables.insert(
                "excludedTags".into(),
                serde_json::json!(query.excluded_tags),
            );
        }

        // The rank floor governs BOTH tag directions (verified against the
        // live API), so it is emitted once whenever either list is present
        // rather than duplicated per direction. Sending it on its own would
        // apply a filter the caller never asked for.
        if !query.tags.is_empty() || !query.excluded_tags.is_empty() {
            filter.push_str(", minimumTagRank: $minimumTagRank");
            declarations.push("$minimumTagRank: Int");
            variables.insert("minimumTagRank".into(), serde_json::json!(MINIMUM_TAG_RANK));
        }

        if let Some(status) = query.status {
            filter.push_str(", status: $status");
            declarations.push("$status: MediaStatus");
            variables.insert("status".into(), serde_json::json!(status_literal(status)));
        }

        if let Some(season) = query.season {
            filter.push_str(", season: $season");
            declarations.push("$season: MediaSeason");
            variables.insert("season".into(), serde_json::json!(season_literal(season)));
        }

        if let Some(year) = query.season_year {
            filter.push_str(", seasonYear: $seasonYear");
            declarations.push("$seasonYear: Int");
            variables.insert("seasonYear".into(), serde_json::json!(year));
        }

        if let Some(score) = query.min_score {
            filter.push_str(", averageScore_greater: $minScore");
            declarations.push("$minScore: Int");
            variables.insert("minScore".into(), serde_json::json!(score));
        }

        // Sort is always set and its literal comes from an enum, never from
        // free text, so it is safe to inline rather than send as a variable.
        filter.push_str(", sort: ");
        filter.push_str(sort_literal(query.sort));

        // `pageInfo` is requested here rather than in `list_query` because this
        // is the only query that paginates; the rest ignore the field.
        let gql = format!(
            r#"
            query ({decls}, $page: Int, $perPage: Int) {{
              Page(page: $page, perPage: $perPage) {{
                pageInfo {{ total currentPage lastPage hasNextPage }}
                media({filter}) {{
                  {MEDIA_FIELDS}
                }}
              }}
            }}
            "#,
            decls = declarations.join(", "),
        );

        // Floored at 1: AniList treats page 0 as an error, and a caller asking
        // for it has almost certainly meant the first page.
        variables.insert("page".into(), serde_json::json!(page.max(1)));
        variables.insert("perPage".into(), serde_json::json!(clamp_limit(per_page)));

        let data: PageData = self
            .query(&gql, serde_json::Value::Object(variables))
            .await?;

        Ok(AnimePage {
            items: data.page.media.into_iter().map(map_media).collect(),
            page_info: PageInfo {
                total: data.page.page_info.total,
                current_page: data.page.page_info.current_page,
                last_page: data.page.page_info.last_page,
                has_next_page: data.page.page_info.has_next_page,
            },
        })
    }
    fn id(&self) -> ProviderId {
        ProviderId::AniList
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

    async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError> {
        let data: TagData = self
            .query(
                "{ MediaTagCollection { name category description isAdult } }",
                serde_json::json!({}),
            )
            .await?;

        // The adult tags are interleaved with the rest rather than grouped in
        // their own field. They are not offered for browsing, so they are
        // dropped here rather than in the UI where every caller would have to
        // remember -- the same reason `genres` drops Hentai.
        Ok(data
            .tags
            .into_iter()
            .filter(|tag| !tag.is_adult)
            .map(|tag| MediaTag {
                name: tag.name,
                category: tag.category,
                // Blank prose is treated as absent, matching how every other
                // optional string in this module is handled.
                description: non_empty(tag.description),
            })
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

    async fn by_id(&self, id: i64) -> Result<Option<Anime>, ProviderError> {
        let query = format!(
            r#"
            query ($id: Int) {{
              Media(id: $id, type: ANIME) {{
                {MEDIA_FIELDS}
                {MEDIA_DETAIL_FIELDS}
              }}
            }}
            "#
        );

        let data: MediaData = self.query(&query, serde_json::json!({ "id": id })).await?;
        Ok(data.media.map(map_media))
    }
}

/// The reader's own list, which is not part of the [`AnimeProvider`] trait.
///
/// Kept inherent rather than added to the trait because the trait describes
/// reading a CATALOGUE, and every provider must be able to do it. Writing to a
/// user's list is an AniList capability -- Jikan has no such thing -- so
/// widening the trait would force a second provider to implement methods it
/// cannot honour.
impl AniListProvider {
    /// Put a work on the reader's list, or move it between lists.
    ///
    /// One mutation covers both create and update: AniList decides which based
    /// on whether the entry already exists for this work, so the caller does
    /// not have to know. Passing `progress` is optional and, when omitted,
    /// leaves the stored value alone -- which is what makes this usable both
    /// for the status menu and for a progress update.
    ///
    /// Requires a token. Without one AniList answers with a GraphQL error,
    /// which surfaces here as a provider error rather than a silent success.
    pub async fn save_list_entry(
        &self,
        media_id: i64,
        status: ListStatus,
        progress: Option<u32>,
    ) -> Result<(), ProviderError> {
        let mutation = r#"
            mutation ($mediaId: Int, $status: MediaListStatus, $progress: Int) {
              SaveMediaListEntry(mediaId: $mediaId, status: $status, progress: $progress) {
                id
                status
                progress
              }
            }
        "#;

        // Built as a map rather than with `json!` so `progress` can be left OUT
        // entirely when it is `None`. Sending `"progress": null` is not the same
        // as omitting it: AniList validates a present argument and answers
        // "The progress must be an integer", which broke every status-only
        // change. An absent variable leaves the stored progress untouched.
        let mut variables = serde_json::Map::new();
        variables.insert("mediaId".into(), serde_json::json!(media_id));
        // `status` travels as AniList's own literal, not the serde form: the
        // GraphQL enum has no camelCase spelling, and the two are deliberately
        // kept apart so the frontend's vocabulary and the wire vocabulary can
        // differ.
        variables.insert("status".into(), serde_json::json!(status.literal()));
        if let Some(progress) = progress {
            variables.insert("progress".into(), serde_json::json!(progress));
        }

        let data: SaveEntryData = self
            .mutate(mutation, serde_json::Value::Object(variables))
            .await?;

        // GraphQL returns `null` for a mutation field when the write was
        // refused without an `errors` entry, so a null here is a failure the
        // status check above would not have caught.
        if data.save.is_none() {
            return Err(ProviderError::Remote("the list entry was not saved".into()));
        }

        Ok(())
    }

    /// Where a work sits on the reader's list, if it is on it at all.
    ///
    /// `None` means either that the work is not on the list or that nobody is
    /// signed in -- AniList returns a null entry in both cases and does not
    /// distinguish them. The caller already knows whether it has a token, so
    /// the ambiguity costs nothing.
    pub async fn list_entry(&self, media_id: i64) -> Result<Option<ListEntry>, ProviderError> {
        let query = r#"
            query ($mediaId: Int) {
              Media(id: $mediaId) {
                mediaListEntry {
                  status
                  progress
                }
              }
            }
        "#;

        let data: ListEntryData = self
            .query(query, serde_json::json!({ "mediaId": media_id }))
            .await?;

        Ok(data
            .media
            .and_then(|media| media.list_entry)
            .and_then(map_list_entry))
    }

    /// The works the reader is currently watching, most recently touched first.
    ///
    /// Reads the reader's own CURRENT list via `MediaListCollection`, which
    /// takes no paging arguments -- so the whole list is fetched and trimmed
    /// here. A CURRENT list is bounded by what someone is actively watching, so
    /// it is small; taking it whole keeps the ordering in one place.
    ///
    /// Requires a token. Without one AniList answers with a GraphQL error,
    /// which surfaces as a provider error rather than an empty list.
    pub async fn continue_watching(
        &self,
        limit: u32,
    ) -> Result<Vec<ContinueWatchingItem>, ProviderError> {
        // `MediaListCollection` does NOT infer the user from the token, even
        // when authenticated: without an explicit `userId` (or `userName`) it
        // answers "User ID/Name & Type arguments required". The id is fetched
        // first because GraphQL cannot feed one field's value into a sibling
        // field's argument -- there is no single-request form.
        let viewer: ViewerData = self
            .query("{ Viewer { id } }", serde_json::json!({}))
            .await?;
        let user_id = viewer
            .viewer
            .ok_or_else(|| ProviderError::Remote("the token has no viewer".into()))?
            .id;

        let query = format!(
            r#"
            query ($userId: Int) {{
              MediaListCollection(userId: $userId, type: ANIME, status: CURRENT) {{
                lists {{
                  entries {{
                    media {{ {MEDIA_FIELDS} }}
                    updatedAt
                    progress
                  }}
                }}
              }}
            }}
            "#
        );

        let data: MediaListCollectionData = self
            .query(&query, serde_json::json!({ "userId": user_id }))
            .await?;

        // `lists` is normally one group for a status query, but it is a list in
        // the schema, so flatten rather than reaching for the first.
        let mut entries: Vec<(i64, ContinueWatchingItem)> = data
            .collection
            .map(|collection| collection.lists)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|group| group.entries)
            .filter_map(|entry| {
                let media = entry.media?;
                // A missing timestamp sorts last rather than dropping the work:
                // it is still being watched, just not knowably recent.
                Some((
                    entry.updated_at.unwrap_or(0),
                    ContinueWatchingItem {
                        anime: map_media(media),
                        // The number of episodes watched, which is where a
                        // "resume" play button should pick up. Zero for an entry
                        // put on the list but never started.
                        progress: entry.progress.unwrap_or(0),
                    },
                ))
            })
            .collect();

        // Descending, so the work played last leads.
        entries.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(entries
            .into_iter()
            .take(limit as usize)
            .map(|(_, item)| item)
            .collect())
    }

    /// The reader's whole anime list, every status, for the My List page.
    ///
    /// No `status:` argument, so AniList returns every group; the caller groups
    /// by each entry's own `status` rather than by which group it arrived in.
    /// That matters because a work can be hidden from the status lists and live
    /// only in a custom list -- keying off the entry's `status` still finds it.
    ///
    /// Deduplicated by media id: a work in both a status list and a custom list
    /// would otherwise appear twice.
    ///
    /// Requires a token. Without one AniList answers with a GraphQL error,
    /// which surfaces as a provider error rather than an empty list.
    pub async fn user_list(&self) -> Result<Vec<UserListEntry>, ProviderError> {
        // Same reason as `continue_watching`: the user is not inferred from the
        // token, and the id cannot be fed into the sibling argument in one
        // request.
        let viewer: ViewerData = self
            .query("{ Viewer { id } }", serde_json::json!({}))
            .await?;
        let user_id = viewer
            .viewer
            .ok_or_else(|| ProviderError::Remote("the token has no viewer".into()))?
            .id;

        let query = format!(
            r#"
            query ($userId: Int) {{
              MediaListCollection(userId: $userId, type: ANIME) {{
                lists {{
                  entries {{
                    id
                    status
                    progress
                    media {{ {MEDIA_FIELDS} }}
                  }}
                }}
              }}
            }}
            "#
        );

        let data: MediaListCollectionData = self
            .query(&query, serde_json::json!({ "userId": user_id }))
            .await?;

        let mut seen = std::collections::HashSet::new();
        let mut entries: Vec<UserListEntry> = Vec::new();

        for group in data
            .collection
            .map(|collection| collection.lists)
            .unwrap_or_default()
        {
            for entry in group.entries {
                // No media means nothing to render; no id means it cannot be
                // deleted, so both are dropped rather than surfaced broken.
                let Some(media) = entry.media else { continue };
                let Some(entry_id) = entry.id else { continue };
                let Some(status) = entry
                    .status
                    .as_deref()
                    .and_then(ListStatus::from_literal)
                else {
                    continue;
                };

                // First sighting wins: a work in more than one group is one row.
                if !seen.insert(media.id) {
                    continue;
                }

                entries.push(UserListEntry {
                    anime: map_media(media),
                    status,
                    progress: entry.progress.unwrap_or(0),
                    entry_id,
                });
            }
        }

        Ok(entries)
    }

    /// Remove a work from the reader's list.
    ///
    /// Takes the LIST ENTRY id (from [`UserListEntry::entry_id`]), not the media
    /// id: `DeleteMediaListEntry` identifies the entry, and a media id would
    /// target the wrong thing.
    pub async fn delete_list_entry(&self, entry_id: i64) -> Result<(), ProviderError> {
        let mutation = r#"
            mutation ($id: Int) {
              DeleteMediaListEntry(id: $id) {
                deleted
              }
            }
        "#;

        let data: DeleteEntryData = self
            .mutate(mutation, serde_json::json!({ "id": entry_id }))
            .await?;

        // A refused delete answers with a present object whose `deleted` is
        // false, so the outer Option alone is not enough -- the bool is checked.
        match data.deleted {
            Some(wire) if wire.deleted => Ok(()),
            _ => Err(ProviderError::Remote(
                "the list entry was not deleted".into(),
            )),
        }
    }
}

/// One entry in the reader's own list, for the My List page.
///
/// Carries the status so the page can group by it, and the list-entry id so the
/// row's menu can remove it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserListEntry {
    pub anime: Anime,
    /// Which list the work is on.
    pub status: ListStatus,
    /// Episodes watched so far, `0` for one never started.
    pub progress: u32,
    /// The id of the list entry itself, needed to delete it.
    pub entry_id: i64,
}

/// One entry in the reader's "Continue Watching" row.
///
/// Carries the work AND how far the reader got: the row's resume button needs
/// the episode number, which is not part of the catalogue [`Anime`] shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueWatchingItem {
    pub anime: Anime,
    /// Episodes watched so far, `0` for one never started.
    pub progress: u32,
}

/// The reader's own entry for a work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEntry {
    pub status: ListStatus,
    /// Episodes watched so far.
    pub progress: u32,
}

#[derive(Deserialize, Default)]
struct SaveEntryData {
    #[serde(rename = "SaveMediaListEntry", default)]
    save: Option<serde_json::Value>,
}

#[derive(Deserialize, Default)]
struct ListEntryData {
    /// The key is capitalised, like every other root field AniList returns.
    /// Serde matches case-sensitively, so without this rename the whole
    /// response decodes to `None` and an entry that exists looks absent.
    #[serde(rename = "Media", default)]
    media: Option<ListEntryMedia>,
}

#[derive(Deserialize, Default)]
struct ListEntryMedia {
    #[serde(rename = "mediaListEntry", default)]
    list_entry: Option<ListEntryWire>,
}

#[derive(Deserialize, Default)]
struct ListEntryWire {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    progress: Option<u32>,
}

/// The root `Viewer` field: the authenticated user.
///
/// Used only to learn the reader's own id, which `MediaListCollection` needs
/// explicitly.
#[derive(Deserialize, Default)]
struct ViewerData {
    #[serde(rename = "Viewer", default)]
    viewer: Option<Viewer>,
}

#[derive(Deserialize, Default)]
struct Viewer {
    id: i64,
}

/// The envelope around `MediaListCollection`, which is a root field.
#[derive(Deserialize, Default)]
struct MediaListCollectionData {
    #[serde(rename = "MediaListCollection", default)]
    collection: Option<MediaListCollection>,
}

#[derive(Deserialize, Default)]
struct MediaListCollection {
    /// One group per status in the schema, so a status-filtered query normally
    /// fills exactly one -- but it is a list, so it is flattened rather than
    /// indexed.
    #[serde(default)]
    lists: Vec<MediaListGroup>,
}

#[derive(Deserialize, Default)]
struct MediaListGroup {
    #[serde(default)]
    entries: Vec<MediaListEntry>,
}

#[derive(Deserialize, Default)]
struct MediaListEntry {
    #[serde(default)]
    media: Option<Media>,
    /// A Unix timestamp in seconds. `0` when absent, which sorts last.
    #[serde(rename = "updatedAt", default)]
    updated_at: Option<i64>,
    /// Episodes watched so far. `0` when absent, matching the domain type.
    #[serde(default)]
    progress: Option<u32>,
    /// The list entry's own id, needed to delete it. Not the media id.
    #[serde(default)]
    id: Option<i64>,
    /// The entry's status, as AniList spells it. Read per entry so grouping
    /// does not depend on which `lists` group the entry arrived in.
    #[serde(default)]
    status: Option<String>,
}

/// The `DeleteMediaListEntry` response, which is a `Deleted` object.
#[derive(Deserialize, Default)]
struct DeleteEntryData {
    #[serde(rename = "DeleteMediaListEntry", default)]
    deleted: Option<DeletedWire>,
}

#[derive(Deserialize, Default)]
struct DeletedWire {
    #[serde(default)]
    deleted: bool,
}

/// Map the wire entry onto the domain type.
///
/// An unrecognised status drops the whole entry rather than defaulting to
/// something: AniList could add a status this app has never heard of, and
/// showing the reader's list position as "Planning" when it is not would be
/// worse than showing nothing.
fn map_list_entry(wire: ListEntryWire) -> Option<ListEntry> {
    let status = ListStatus::from_literal(wire.status.as_deref()?)?;

    Some(ListEntry {
        status,
        // A missing progress is zero rather than an error: AniList omits it for
        // an entry that has never been watched.
        progress: wire.progress.unwrap_or(0),
    })
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

/// AniList's literal for a format filter.
///
/// Kept here rather than on the domain enum so provider vocabulary stays in
/// the provider. An unknown variant cannot occur -- the match is exhaustive --
/// which is the point of using an enum over a free-text string.
fn format_literal(format: FormatFilter) -> &'static str {
    match format {
        FormatFilter::Tv => "TV",
        FormatFilter::Movie => "MOVIE",
        FormatFilter::Ova => "OVA",
        FormatFilter::Ona => "ONA",
        FormatFilter::Special => "SPECIAL",
        FormatFilter::Music => "MUSIC",
    }
}

/// AniList's literal for a status filter.
fn status_literal(status: StatusFilter) -> &'static str {
    match status {
        StatusFilter::Releasing => "RELEASING",
        StatusFilter::Finished => "FINISHED",
        StatusFilter::NotYetReleased => "NOT_YET_RELEASED",
    }
}

/// AniList's literal for a season filter.
fn season_literal(season: SeasonFilter) -> &'static str {
    match season {
        SeasonFilter::Winter => "WINTER",
        SeasonFilter::Spring => "SPRING",
        SeasonFilter::Summer => "SUMMER",
        SeasonFilter::Fall => "FALL",
    }
}

/// AniList's literal for a sort option.
fn sort_literal(sort: SortOption) -> &'static str {
    match sort {
        SortOption::Popularity => "POPULARITY_DESC",
        SortOption::Score => "SCORE_DESC",
        SortOption::Newest => "START_DATE_DESC",
        SortOption::TitleAz => "TITLE_ROMAJI",
        SortOption::Trending => "TRENDING_DESC",
        SortOption::Favorites => "FAVOURITES_DESC",
        SortOption::DateAdded => "ID_DESC",
        SortOption::SearchMatch => "SEARCH_MATCH",
    }
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

/// The root `MediaTagCollection` field, a bare array of tag descriptors.
#[derive(Deserialize)]
struct TagData {
    #[serde(rename = "MediaTagCollection", default)]
    tags: Vec<TagWire>,
}

#[derive(Deserialize)]
struct TagWire {
    name: String,
    #[serde(default)]
    category: String,
    /// Nullable on AniList, so a tag without prose still deserialises.
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "isAdult", default)]
    is_adult: bool,
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
    /// Defaulted because only `browse` requests this. The other Page queries
    /// never read it, and requiring it would break their deserialization.
    #[serde(rename = "pageInfo", default)]
    page_info: PageInfoWire,
}

/// `Page.pageInfo`, as AniList spells it.
#[derive(Deserialize, Default)]
struct PageInfoWire {
    #[serde(default)]
    total: u32,
    #[serde(rename = "currentPage", default)]
    current_page: u32,
    #[serde(rename = "lastPage", default)]
    last_page: u32,
    #[serde(rename = "hasNextPage", default)]
    has_next_page: bool,
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
    #[serde(rename = "idMal", default)]
    id_mal: Option<i64>,
    popularity: Option<u32>,
    #[serde(rename = "streamingEpisodes", default)]
    streaming_episodes: Option<Vec<StreamingEpisodeWire>>,
    id: i64,
    /// Only requested by the recommendation sub-query, where it distinguishes
    /// anime from manga. Absent everywhere else, so the check tolerates `None`.
    #[serde(rename = "type", default)]
    media_type: Option<String>,
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
    #[serde(default)]
    season: Option<String>,
    #[serde(rename = "seasonYear", default)]
    season_year: Option<u32>,
    #[serde(default)]
    relations: Option<RelationConnection>,
    #[serde(default)]
    recommendations: Option<RecommendationConnection>,
    #[serde(default)]
    trailer: Option<TrailerWire>,
}

#[derive(Deserialize, Default)]
struct RelationConnection {
    #[serde(default)]
    edges: Vec<RelationEdge>,
}

#[derive(Deserialize, Default)]
struct RelationEdge {
    #[serde(rename = "relationType", default)]
    relation_type: Option<String>,
    #[serde(default)]
    node: Option<RelationNode>,
}

#[derive(Deserialize, Default)]
struct RelationNode {
    id: i64,
    /// AniList mixes manga and anime into one relation graph, so the type is read
    /// back to filter the manga edges out.
    #[serde(rename = "type", default)]
    media_type: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    episodes: Option<u32>,
    #[serde(default)]
    title: Option<MediaTitle>,
    #[serde(rename = "coverImage", default)]
    cover_image: Option<CoverImage>,
}

#[derive(Deserialize, Default)]
struct RecommendationConnection {
    #[serde(default)]
    edges: Vec<RecommendationEdge>,
}

#[derive(Deserialize, Default)]
struct RecommendationEdge {
    #[serde(default)]
    node: Option<RecommendationNode>,
}

#[derive(Deserialize, Default)]
struct RecommendationNode {
    /// Vote tally for the suggestion. SIGNED: AniList returns -1 when users
    /// downvote a recommendation, and a u32 would fail the whole decode.
    #[serde(default)]
    rating: i32,
    #[serde(rename = "mediaRecommendation", default)]
    media_recommendation: Option<Media>,
}

#[derive(Deserialize, Default)]
struct TrailerWire {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    site: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>,
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
        id_mal: media.id_mal,
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
        season: non_empty(media.season),
        season_year: media.season_year,
        streaming_episodes: media
            .streaming_episodes
            .unwrap_or_default()
            .into_iter()
            .filter_map(map_streaming_episode)
            .collect(),
        relations: media
            .relations
            .unwrap_or_default()
            .edges
            .into_iter()
            .filter_map(map_relation_edge)
            .collect(),
        recommendations: media
            .recommendations
            .unwrap_or_default()
            .edges
            .into_iter()
            .filter_map(map_recommendation_edge)
            .collect(),
        trailer: media.trailer.and_then(map_trailer),
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

/// Map one relation edge, dropping anything that is not an anime.
///
/// AniList's relation graph mixes media types, so a work's relations can include
/// its source manga. Those are skipped: this is an anime app, and a manga node
/// carries no episode data to show.
fn map_relation_edge(edge: RelationEdge) -> Option<RelatedAnime> {
    let node = edge.node?;
    if node.media_type.as_deref() != Some("ANIME") {
        return None;
    }
    // The relation type is what the sidebar labels the row with; without it the
    // entry is unlabelled, so it is treated as absent rather than guessed.
    let relation_type = non_empty(edge.relation_type)?;

    Some(RelatedAnime {
        id: node.id,
        title: Title {
            romaji: node
                .title
                .as_ref()
                .and_then(|t| non_empty(t.romaji.clone())),
            english: node
                .title
                .as_ref()
                .and_then(|t| non_empty(t.english.clone())),
            native: node
                .title
                .as_ref()
                .and_then(|t| non_empty(t.native.clone())),
            user_preferred: None,
        },
        cover_image: node.cover_image.and_then(pick_cover),
        format: non_empty(node.format),
        status: non_empty(node.status),
        episode_count: node.episodes,
        relation_type,
    })
}

/// Map one recommendation edge, dropping any with no recommended work.
///
/// The nested work is mapped through [`map_media`], so it carries the same fields
/// a card expects. Its own relations and recommendations are not requested, so
/// they come back empty.
fn map_recommendation_edge(edge: RecommendationEdge) -> Option<RecommendedAnime> {
    let node = edge.node?;
    let media = node.media_recommendation?;
    if media.media_type.as_deref() != Some("ANIME") {
        return None;
    }

    Some(RecommendedAnime {
        anime: map_media(media),
        rating: node.rating,
    })
}

/// Map a trailer, requiring both the id and the site.
///
/// A trailer with no `id` has no video to open and one with no `site` has no URL
/// to build, so either missing drops the whole thing rather than surfacing a
/// link that cannot be followed.
fn map_trailer(trailer: TrailerWire) -> Option<Trailer> {
    Some(Trailer {
        id: non_empty(trailer.id)?,
        site: non_empty(trailer.site)?,
        thumbnail: non_empty(trailer.thumbnail),
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
    /// A page response carrying `pageInfo`, for the paginated browse query.
    fn browse_response(
        media: Vec<serde_json::Value>,
        total: u32,
        current: u32,
        last: u32,
    ) -> serde_json::Value {
        serde_json::json!({
            "data": { "Page": {
                "pageInfo": {
                    "total": total,
                    "currentPage": current,
                    "lastPage": last,
                    "hasNextPage": current < last,
                },
                "media": media,
            } }
        })
    }

    /// Paging must reach AniList as variables. Hardcoding page 1 is precisely
    /// what stopped search from ever showing a second screenful.
    #[tokio::test]
    async fn browse_forwards_the_requested_page() {
        let (_server, provider) = provider_expecting("\"page\":3").await;

        assert!(provider.browse(BrowseQuery::default(), 3, 24).await.is_ok());
    }

    #[tokio::test]
    async fn browse_forwards_the_requested_page_size() {
        let (_server, provider) = provider_expecting("\"perPage\":24").await;

        assert!(provider.browse(BrowseQuery::default(), 1, 24).await.is_ok());
    }

    /// A page of zero would be rejected by AniList, and means "first page" to
    /// any caller that sends it.
    #[tokio::test]
    async fn browse_treats_page_zero_as_the_first_page() {
        let (_server, provider) = provider_expecting("\"page\":1").await;

        assert!(provider.browse(BrowseQuery::default(), 0, 24).await.is_ok());
    }

    /// A default query carries no filters at all, so the request must not
    /// mention them. Sending an unset filter is how a browse turns into an
    /// accidental exact match.
    #[tokio::test]
    async fn browse_omits_unset_filters() {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(200).set_body_json(browse_response(
                    vec![media_json()],
                    1,
                    1,
                    1,
                ))
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        provider
            .browse(BrowseQuery::default(), 1, 24)
            .await
            .expect("browse should succeed");

        let body = captured.lock().unwrap().clone();
        for absent in [
            "genre_in",
            "search",
            "format:",
            "status:",
            "season:",
            "tag_in",
            "tag_not_in",
            "excludedTags",
            "minimumTagRank",
        ] {
            assert!(
                !body.contains(absent),
                "unset filter {absent} should not be sent; body was {body}"
            );
        }
    }

    /// Every filter travels as a GraphQL variable, never interpolated into the
    /// filter string. That is what makes user-supplied values safe.
    #[tokio::test]
    async fn browse_declares_a_variable_for_each_filter() {
        let (_server, provider) = provider_expecting(
            "$search: String, $genres: [String], $format: MediaFormat, $status: MediaStatus, $season: MediaSeason, $seasonYear: Int, $minScore: Int",
        )
        .await;

        let query = BrowseQuery {
            search: Some("piece".into()),
            genres: vec!["Action".into()],
            format: Some(FormatFilter::Tv),
            status: Some(StatusFilter::Finished),
            season: Some(SeasonFilter::Fall),
            season_year: Some(2024),
            min_score: Some(70),
            sort: SortOption::Score,
            tags: Vec::new(),
            excluded_tags: Vec::new(),
        };

        assert!(provider.browse(query, 1, 24).await.is_ok());
    }

    /// Tags travel as a variable and bring the rank floor with them. Without
    /// the floor a single stray tagging is enough to match, which is why an
    /// unfiltered `tag_in: [Isekai]` surfaced BLEACH.
    #[tokio::test]
    async fn browse_sends_tags_with_the_rank_floor() {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(200).set_body_json(browse_response(
                    vec![media_json()],
                    1,
                    1,
                    1,
                ))
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        let query = BrowseQuery {
            tags: vec!["Isekai".into()],
            ..BrowseQuery::default()
        };

        provider.browse(query, 1, 24).await.expect("should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(body.contains("tag_in: $tags"), "body was {body}");
        assert!(
            body.contains("minimumTagRank: $minimumTagRank"),
            "the rank floor must travel with the tags; body was {body}"
        );
        // AniList rejects the whole query when a variable is used but not
        // declared, so the declaration matters as much as the filter.
        assert!(
            body.contains("$tags: [String], $minimumTagRank: Int"),
            "both variables must be declared; body was {body}"
        );
        assert!(body.contains("\"Isekai\""), "body was {body}");
    }

    /// The rank floor is only meaningful next to `tag_in`, so a query without
    /// tags must not carry it.
    #[tokio::test]
    async fn browse_omits_the_rank_floor_without_tags() {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(200).set_body_json(browse_response(
                    vec![media_json()],
                    1,
                    1,
                    1,
                ))
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        let query = BrowseQuery {
            genres: vec!["Action".into()],
            ..BrowseQuery::default()
        };

        provider.browse(query, 1, 24).await.expect("should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(!body.contains("minimumTagRank"), "body was {body}");
        assert!(!body.contains("tag_in"), "body was {body}");
        assert!(!body.contains("tag_not_in"), "body was {body}");
    }

    /// Exclusion travels as its own variable and its own AniList argument. It
    /// brings the rank floor with it, because the floor governs both tag
    /// directions -- verified against the live API, where a work tagged below
    /// the floor survived its own exclusion.
    #[tokio::test]
    async fn browse_sends_excluded_tags_with_the_rank_floor() {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(200).set_body_json(browse_response(
                    vec![media_json()],
                    1,
                    1,
                    1,
                ))
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        let query = BrowseQuery {
            excluded_tags: vec!["Harem".into()],
            ..BrowseQuery::default()
        };

        provider.browse(query, 1, 24).await.expect("should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(
            body.contains("tag_not_in: $excludedTags"),
            "body was {body}"
        );
        assert!(body.contains("\"Harem\""), "body was {body}");
        assert!(
            body.contains("minimumTagRank: $minimumTagRank"),
            "the rank floor must travel with exclusions too; body was {body}"
        );
        assert!(
            body.contains("$excludedTags: [String]"),
            "the variable must be declared or AniList rejects the query; body was {body}"
        );
        // Exclusion must not smuggle in an inclusion filter.
        assert!(!body.contains("tag_in"), "body was {body}");
    }

    /// The two directions are independent: sending both must produce both
    /// arguments, not one overwriting the other.
    #[tokio::test]
    async fn browse_sends_included_and_excluded_tags_together() {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(200).set_body_json(browse_response(
                    vec![media_json()],
                    1,
                    1,
                    1,
                ))
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        let query = BrowseQuery {
            tags: vec!["Isekai".into()],
            excluded_tags: vec!["Harem".into()],
            ..BrowseQuery::default()
        };

        provider.browse(query, 1, 24).await.expect("should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(body.contains("tag_in: $tags"), "body was {body}");
        assert!(
            body.contains("tag_not_in: $excludedTags"),
            "body was {body}"
        );
        // One floor serves both directions, so it must appear exactly once.
        assert_eq!(
            body.matches("minimumTagRank: $minimumTagRank").count(),
            1,
            "the floor should be emitted once, not per direction; body was {body}"
        );
    }

    /// Enum filters go out as AniList's own literals, mapped from our enum
    /// rather than passed through as free text.
    #[tokio::test]
    async fn browse_maps_enum_filters_to_anilist_literals() {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(200).set_body_json(browse_response(
                    vec![media_json()],
                    1,
                    1,
                    1,
                ))
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        provider
            .browse(
                BrowseQuery {
                    format: Some(FormatFilter::Movie),
                    status: Some(StatusFilter::NotYetReleased),
                    season: Some(SeasonFilter::Winter),
                    ..BrowseQuery::default()
                },
                1,
                24,
            )
            .await
            .expect("browse should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(body.contains("\"format\":\"MOVIE\""), "body was {body}");
        assert!(
            body.contains("\"status\":\"NOT_YET_RELEASED\""),
            "body was {body}"
        );
        assert!(body.contains("\"season\":\"WINTER\""), "body was {body}");
    }

    /// A blank search is an absent filter, not a search for nothing.
    #[tokio::test]
    async fn browse_drops_a_blank_search() {
        let server = MockServer::start().await;
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let sink = captured.clone();

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                *sink.lock().unwrap() = String::from_utf8_lossy(&req.body).to_string();
                ResponseTemplate::new(200).set_body_json(browse_response(
                    vec![media_json()],
                    1,
                    1,
                    1,
                ))
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        provider
            .browse(
                BrowseQuery {
                    search: Some("   ".into()),
                    ..BrowseQuery::default()
                },
                1,
                24,
            )
            .await
            .expect("browse should succeed");

        let body = captured.lock().unwrap().clone();
        assert!(!body.contains("$search"), "body was {body}");
    }

    #[tokio::test]
    async fn browse_sorts_by_the_requested_option() {
        let (_server, provider) = provider_expecting("sort: SCORE_DESC").await;

        assert!(provider
            .browse(
                BrowseQuery {
                    sort: SortOption::Score,
                    ..BrowseQuery::default()
                },
                1,
                24,
            )
            .await
            .is_ok());
    }

    /// Every sort variant must map to a distinct AniList literal. A copy-paste
    /// slip here would make two options behave identically, which the UI would
    /// present as a control that does nothing.
    #[tokio::test]
    async fn each_sort_maps_to_its_own_anilist_literal() {
        let cases = [
            (SortOption::Popularity, "sort: POPULARITY_DESC"),
            (SortOption::Score, "sort: SCORE_DESC"),
            (SortOption::Newest, "sort: START_DATE_DESC"),
            (SortOption::TitleAz, "sort: TITLE_ROMAJI"),
            (SortOption::Trending, "sort: TRENDING_DESC"),
            (SortOption::Favorites, "sort: FAVOURITES_DESC"),
            (SortOption::DateAdded, "sort: ID_DESC"),
            (SortOption::SearchMatch, "sort: SEARCH_MATCH"),
        ];

        // Guard the premise: if two variants shared a literal, the loop below
        // would pass while the options were indistinguishable.
        let literals: Vec<&str> = cases.iter().map(|(_, literal)| *literal).collect();
        let mut unique = literals.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), literals.len(), "two sorts share a literal");

        for (sort, expected) in cases {
            let (_server, provider) = provider_expecting(expected).await;

            assert!(
                provider
                    .browse(
                        BrowseQuery {
                            sort,
                            ..BrowseQuery::default()
                        },
                        1,
                        24,
                    )
                    .await
                    .is_ok(),
                "{sort:?} should send {expected}"
            );
        }
    }

    #[tokio::test]
    async fn browse_maps_page_info() {
        let (_server, provider) =
            provider_with(browse_response(vec![media_json()], 120, 2, 5), 200).await;

        let page = provider
            .browse(BrowseQuery::default(), 2, 24)
            .await
            .expect("browse should succeed");

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, 21);
        assert_eq!(page.page_info.total, 120);
        assert_eq!(page.page_info.current_page, 2);
        assert_eq!(page.page_info.last_page, 5);
        assert!(page.page_info.has_next_page);
    }

    /// The last page must report no next page, or the UI would offer a link to
    /// a page that does not exist.
    #[tokio::test]
    async fn browse_reports_the_last_page_has_no_next() {
        let (_server, provider) =
            provider_with(browse_response(vec![media_json()], 120, 5, 5), 200).await;

        let page = provider
            .browse(BrowseQuery::default(), 5, 24)
            .await
            .expect("browse should succeed");

        assert!(!page.page_info.has_next_page);
    }

    /// An empty result set is not an error, and must still carry page info so
    /// the UI can say "no matches" rather than failing.
    #[tokio::test]
    async fn browse_handles_an_empty_page() {
        let (_server, provider) = provider_with(browse_response(vec![], 0, 1, 1), 200).await;

        let page = provider
            .browse(BrowseQuery::default(), 1, 24)
            .await
            .expect("browse should succeed");

        assert!(page.items.is_empty());
        assert_eq!(page.page_info.total, 0);
        assert!(!page.page_info.has_next_page);
    }

    /// The root `GenreCollection` payload shape.
    fn genre_response(genres: Vec<&str>) -> serde_json::Value {
        serde_json::json!({ "data": { "GenreCollection": genres } })
    }

    /// The root `MediaTagCollection` payload shape. Each entry is
    /// `(name, category, description, isAdult)`.
    fn tag_response(tags: Vec<(&str, &str, Option<&str>, bool)>) -> serde_json::Value {
        let entries: Vec<serde_json::Value> = tags
            .into_iter()
            .map(|(name, category, description, is_adult)| {
                serde_json::json!({
                    "name": name,
                    "category": category,
                    "description": description,
                    "isAdult": is_adult,
                })
            })
            .collect();

        serde_json::json!({ "data": { "MediaTagCollection": entries } })
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

    #[tokio::test]
    async fn tags_carry_their_category() {
        let (_server, provider) = provider_with(
            tag_response(vec![
                ("Isekai", "Theme-Fantasy", None, false),
                ("School", "Setting-Scene", None, false),
            ]),
            200,
        )
        .await;

        let tags = provider.tags().await.expect("tags should succeed");

        // The category is what the filter UI groups by, so losing it would
        // leave every tag in one undifferentiated list.
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "Isekai");
        assert_eq!(tags[0].category, "Theme-Fantasy");
        assert_eq!(tags[1].category, "Setting-Scene");
    }

    /// The provider's prose is what the chip tooltip shows, so it has to
    /// survive the mapping rather than being dropped as unmapped.
    #[tokio::test]
    async fn tags_carry_their_description() {
        let (_server, provider) = provider_with(
            tag_response(vec![(
                "Isekai",
                "Theme-Fantasy",
                Some("Another world."),
                false,
            )]),
            200,
        )
        .await;

        let tags = provider.tags().await.unwrap();
        assert_eq!(tags[0].description.as_deref(), Some("Another world."));
    }

    /// AniList leaves the field nullable, and some entries carry only
    /// whitespace. Either way the chip should render without a tooltip
    /// rather than with an empty one.
    #[tokio::test]
    async fn tags_treat_a_blank_description_as_absent() {
        let (_server, provider) = provider_with(
            tag_response(vec![
                ("Isekai", "Theme-Fantasy", None, false),
                ("School", "Setting-Scene", Some("   "), false),
            ]),
            200,
        )
        .await;

        let tags = provider.tags().await.unwrap();
        assert_eq!(tags[0].description, None);
        assert_eq!(tags[1].description, None);
    }

    /// Adult tags sit interleaved in the collection, not behind their own
    /// field, so they have to be filtered out by flag.
    #[tokio::test]
    async fn tags_drop_the_adult_entries() {
        let (_server, provider) = provider_with(
            tag_response(vec![
                ("Isekai", "Theme-Fantasy", None, false),
                ("Nudity", "Sexual Content", None, true),
            ]),
            200,
        )
        .await;

        let tags = provider.tags().await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Isekai");
    }

    /// The adult flag and the description only arrive if they were asked
    /// for. Omitting the flag would leave every entry deserialising as
    /// non-adult and let the adult tags through while the filter above
    /// still looked correct; omitting the description would silently empty
    /// every tooltip.
    #[tokio::test]
    async fn tags_query_asks_for_the_adult_flag_and_description() {
        let (_server, provider) =
            provider_expecting("MediaTagCollection { name category description isAdult }").await;

        provider.tags().await.expect("tags should succeed");
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
        media["season"] = serde_json::json!("FALL");
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
        // The season is a separate field from the year: AniList reports them
        // independently, and the home page renders them as one column.
        assert_eq!(first.season.as_deref(), Some("FALL"));
    }

    #[tokio::test]
    async fn id_mal_is_mapped_for_episode_enrichment() {
        let mut media = media_json();
        media["idMal"] = serde_json::json!(21);
        let (_server, provider) = provider_with(page_response(vec![media]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        assert_eq!(anime[0].id_mal, Some(21));
    }

    #[tokio::test]
    async fn id_mal_is_absent_when_the_provider_has_none() {
        // Not every work is linked to MAL, so the field must stay `None` rather
        // than defaulting to a wrong id that would enrich the wrong episodes.
        let (_server, provider) = provider_with(page_response(vec![media_json()]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        assert_eq!(anime[0].id_mal, None);
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

    /// Assert the one request the server received carried `expected`.
    ///
    /// `None` means the request must be anonymous. A helper rather than a bare
    /// `assert` per test because reading the header involves an await, an
    /// `Option` chain, and a `to_str` that can fail on non-ASCII -- repeated
    /// across five tests that is five chances to get the unwrapping wrong.
    async fn assert_auth(server: &MockServer, expected: Option<&str>) {
        let requests = server
            .received_requests()
            .await
            .expect("requests should be recorded");

        let actual = requests
            .first()
            .and_then(|request| request.headers.get("authorization"))
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);

        assert_eq!(actual.as_deref(), expected);
    }

    /// The `variables` object of the one request the server received.
    async fn sent_variables(server: &MockServer) -> serde_json::Value {
        let requests = server
            .received_requests()
            .await
            .expect("requests should be recorded");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json body");
        body["variables"].clone()
    }

    fn saved_entry_response() -> serde_json::Value {
        serde_json::json!({
            "data": { "SaveMediaListEntry": { "id": 4, "status": "CURRENT", "progress": 3 } }
        })
    }

    #[tokio::test]
    async fn saving_a_list_entry_sends_the_anilist_literal() {
        let (server, provider) = provider_with(saved_entry_response(), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider
            .save_list_entry(21, ListStatus::Paused, None)
            .await
            .expect("save should succeed");

        // SCREAMING_SNAKE, not the camelCase the frontend uses: the GraphQL
        // enum has no camelCase spelling, so sending the serde form would be
        // rejected.
        assert_eq!(sent_variables(&server).await["status"], "PAUSED");
    }

    #[tokio::test]
    async fn saving_a_list_entry_sends_the_media_id() {
        let (server, provider) = provider_with(saved_entry_response(), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider
            .save_list_entry(21, ListStatus::Current, None)
            .await
            .expect("save should succeed");

        assert_eq!(sent_variables(&server).await["mediaId"], 21);
    }

    /// The whole 400 this fixes: AniList reads the operation from `query` and
    /// takes the kind from the keyword inside the string. Sending the document
    /// under a `mutation` key is answered with "No query or mutation provided",
    /// which is exactly what a real "Add to list" click hit.
    #[tokio::test]
    async fn saving_a_list_entry_sends_the_document_under_the_query_key() {
        let (server, provider) = provider_with(saved_entry_response(), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider
            .save_list_entry(21, ListStatus::Current, None)
            .await
            .expect("save should succeed");

        let requests = server
            .received_requests()
            .await
            .expect("requests should be recorded");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json body");

        let document = body["query"]
            .as_str()
            .expect("the document must travel under the query key");
        assert!(
            document.contains("SaveMediaListEntry"),
            "the mutation body should carry the operation"
        );
        assert!(
            body.get("mutation").is_none(),
            "a `mutation` key is not something AniList reads"
        );
    }

    /// Omitting progress must leave the key OUT of the variables entirely, not
    /// send `null`: a present-but-null argument is validated and rejected with
    /// "The progress must be an integer", whereas an absent variable leaves the
    /// stored value untouched. Sending `0` would also be wrong -- it would reset
    /// the reader's progress every time they changed the status.
    #[tokio::test]
    async fn saving_without_progress_omits_the_variable() {
        let (server, provider) = provider_with(saved_entry_response(), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider
            .save_list_entry(21, ListStatus::Current, None)
            .await
            .expect("save should succeed");

        assert!(
            sent_variables(&server).await.get("progress").is_none(),
            "progress must be absent rather than null or 0"
        );
    }

    #[tokio::test]
    async fn saving_can_set_progress() {
        let (server, provider) = provider_with(saved_entry_response(), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider
            .save_list_entry(21, ListStatus::Current, Some(3))
            .await
            .expect("save should succeed");

        assert_eq!(sent_variables(&server).await["progress"], 3);
    }

    /// A refused write must not look like a success.
    #[tokio::test]
    async fn a_refused_save_is_an_error() {
        let response = serde_json::json!({ "data": { "SaveMediaListEntry": null } });
        let (_server, provider) = provider_with(response, 200).await;
        provider.set_token(Some("secret-token".into()));

        let result = provider
            .save_list_entry(21, ListStatus::Current, None)
            .await;

        // GraphQL answers 200 with a null field when it refuses, so the status
        // check alone would have reported this as success.
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn a_save_carries_the_token() {
        let (server, provider) = provider_with(saved_entry_response(), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider
            .save_list_entry(21, ListStatus::Current, None)
            .await
            .expect("save should succeed");

        assert_auth(&server, Some("Bearer secret-token")).await;
    }

    #[tokio::test]
    async fn a_list_entry_is_read_back() {
        let response = serde_json::json!({
            "data": { "Media": { "mediaListEntry": { "status": "COMPLETED", "progress": 26 } } }
        });
        let (_server, provider) = provider_with(response, 200).await;

        let entry = provider
            .list_entry(21)
            .await
            .expect("read should succeed")
            .expect("an entry should be present");

        assert_eq!(entry.status, ListStatus::Completed);
        assert_eq!(entry.progress, 26);
    }

    /// A work that is not on the list comes back as a null entry.
    #[tokio::test]
    async fn a_work_off_the_list_has_no_entry() {
        let response = serde_json::json!({
            "data": { "Media": { "mediaListEntry": null } }
        });
        let (_server, provider) = provider_with(response, 200).await;

        assert!(provider.list_entry(21).await.expect("read").is_none());
    }

    /// AniList omits progress for an entry that has never been watched, which
    /// means zero rather than an error.
    #[tokio::test]
    async fn a_missing_progress_reads_as_zero() {
        let response = serde_json::json!({
            "data": { "Media": { "mediaListEntry": { "status": "PLANNING" } } }
        });
        let (_server, provider) = provider_with(response, 200).await;

        let entry = provider.list_entry(21).await.expect("read").expect("entry");

        assert_eq!(entry.progress, 0);
    }

    /// An unrecognised status drops the entry rather than guessing at one.
    #[tokio::test]
    async fn an_unknown_status_drops_the_entry() {
        let response = serde_json::json!({
            "data": { "Media": { "mediaListEntry": { "status": "BINGING", "progress": 4 } } }
        });
        let (_server, provider) = provider_with(response, 200).await;

        // AniList could add a status this app has never heard of, and claiming
        // the reader's position is something it is not would be a lie.
        assert!(provider.list_entry(21).await.expect("read").is_none());
    }

    /// A rewatch folds into `Current` rather than being dropped: AniList has
    /// REWATCHING/REPEATING as distinct statuses, and losing those entries
    /// would be worse than showing them under "Watching".
    #[tokio::test]
    async fn a_rewatch_folds_into_current() {
        let response = serde_json::json!({
            "data": { "Media": { "mediaListEntry": { "status": "REWATCHING", "progress": 4 } } }
        });
        let (_server, provider) = provider_with(response, 200).await;

        let entry = provider.list_entry(21).await.expect("read").expect("entry");

        assert_eq!(entry.status, ListStatus::Current);
        assert_eq!(entry.progress, 4);
    }

    /// One `MediaListCollection` entry: the standard media fixture plus the
    /// `updatedAt` stamp the ordering depends on.
    fn collection_entry(updated_at: i64) -> serde_json::Value {
        serde_json::json!({ "media": media_json(), "updatedAt": updated_at })
    }

    fn collection_response(entries: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "data": {
                "MediaListCollection": { "lists": [{ "entries": entries }] }
            }
        })
    }

    /// A provider whose mock answers the two requests `continue_watching`
    /// makes: the `Viewer` lookup, then the collection. A single static
    /// response cannot serve both, because the two have different shapes and
    /// the second depends on the id the first returns.
    async fn provider_for_collection(
        entries: Vec<serde_json::Value>,
    ) -> (MockServer, AniListProvider) {
        let server = MockServer::start().await;
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let collection = collection_response(entries);

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |_req: &wiremock::Request| {
                let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let body = if n == 0 {
                    serde_json::json!({ "data": { "Viewer": { "id": 7 } } })
                } else {
                    collection.clone()
                };
                ResponseTemplate::new(200).set_body_json(body)
            })
            .mount(&server)
            .await;

        let provider = AniListProvider::with_endpoint(server.uri());
        (server, provider)
    }

    #[tokio::test]
    async fn continue_watching_maps_the_current_list() {
        let (_server, provider) = provider_for_collection(vec![collection_entry(100)]).await;

        let anime = provider
            .continue_watching(10)
            .await
            .expect("read should succeed");

        assert_eq!(anime.len(), 1);
        assert_eq!(anime[0].anime.id, 21);
        assert_eq!(anime[0].anime.title.romaji.as_deref(), Some("One Piece"));
    }

    /// One collection entry carrying a `progress`, so the resume point is
    /// observable rather than assumed.
    fn collection_entry_with_progress(progress: u32) -> serde_json::Value {
        serde_json::json!({
            "media": media_json(),
            "updatedAt": 100,
            "progress": progress,
        })
    }

    /// The resume point has to survive: it is what a play button picks up from.
    #[tokio::test]
    async fn continue_watching_carries_the_progress() {
        let (_server, provider) =
            provider_for_collection(vec![collection_entry_with_progress(7)]).await;

        let items = provider.continue_watching(10).await.expect("read");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].progress, 7);
    }

    /// An entry that was never started reports zero rather than failing.
    #[tokio::test]
    async fn continue_watching_defaults_missing_progress_to_zero() {
        let (_server, provider) = provider_for_collection(vec![collection_entry(100)]).await;

        let items = provider.continue_watching(10).await.expect("read");

        assert_eq!(items[0].progress, 0);
    }

    /// One collection entry for a given work id and timestamp, so the ordering
    /// is observable by id rather than inferred from identical fixtures.
    fn collection_entry_for(id: i64, updated_at: i64) -> serde_json::Value {
        let mut media = media_json();
        media["id"] = serde_json::json!(id);
        serde_json::json!({ "media": media, "updatedAt": updated_at })
    }

    /// Most recently touched first: "Continue Watching" is only useful if the
    /// last thing played leads, and AniList does not guarantee an order.
    #[tokio::test]
    async fn continue_watching_orders_by_updated_at_descending() {
        // Deliberately out of order, with distinct ids so the resulting order
        // can be asserted rather than just the count.
        let older = collection_entry_for(1, 100);
        let newest = collection_entry_for(2, 300);
        let middle = collection_entry_for(3, 200);
        let (_server, provider) = provider_for_collection(vec![older, newest, middle]).await;

        let anime = provider.continue_watching(10).await.expect("read");

        let ids: Vec<i64> = anime.iter().map(|item| item.anime.id).collect();
        assert_eq!(ids, vec![2, 3, 1], "newest updatedAt should lead");
    }

    /// An entry whose work is missing cannot be rendered, so it is dropped
    /// rather than surfaced as a blank card.
    #[tokio::test]
    async fn continue_watching_drops_entries_without_media() {
        let mut orphan = collection_entry(300);
        orphan["media"] = serde_json::Value::Null;
        let kept = collection_entry(100);

        let (_server, provider) = provider_for_collection(vec![orphan, kept]).await;

        let anime = provider.continue_watching(10).await.expect("read");

        assert_eq!(anime.len(), 1);
    }

    /// The limit is applied here because `MediaListCollection` takes no paging
    /// arguments.
    #[tokio::test]
    async fn continue_watching_honours_the_limit() {
        let entries: Vec<serde_json::Value> = (0..5).map(|i| collection_entry(i * 10)).collect();
        let (_server, provider) = provider_for_collection(entries).await;

        let anime = provider.continue_watching(2).await.expect("read");

        assert_eq!(anime.len(), 2);
    }

    /// An empty list is not an error.
    #[tokio::test]
    async fn continue_watching_handles_an_empty_list() {
        let (_server, provider) = provider_for_collection(vec![]).await;

        let anime = provider.continue_watching(10).await.expect("read");

        assert!(anime.is_empty());
    }

    /// The reader's own list requires their credential.
    #[tokio::test]
    async fn continue_watching_carries_the_token() {
        let (server, provider) = provider_for_collection(vec![collection_entry(100)]).await;
        provider.set_token(Some("secret-token".into()));

        provider.continue_watching(10).await.expect("read");

        assert_auth(&server, Some("Bearer secret-token")).await;
    }

    /// A full user-list entry: id, status, progress and the media fixture.
    fn user_list_entry(id: i64, status: &str, progress: u32) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "status": status,
            "progress": progress,
            "media": media_json(),
        })
    }

    /// Like [`user_list_entry`] but for a distinct work, so dedupe (which keys
    /// off the media id) does not collapse entries that differ only by status.
    fn user_list_entry_for(
        media_id: i64,
        entry_id: i64,
        status: &str,
        progress: u32,
    ) -> serde_json::Value {
        let mut media = media_json();
        media["id"] = serde_json::json!(media_id);
        serde_json::json!({
            "id": entry_id,
            "status": status,
            "progress": progress,
            "media": media,
        })
    }

    #[tokio::test]
    async fn user_list_maps_every_status() {
        let (_server, provider) = provider_for_collection(vec![
            user_list_entry_for(1, 1, "CURRENT", 3),
            user_list_entry_for(2, 2, "COMPLETED", 26),
            user_list_entry_for(3, 3, "PLANNING", 0),
            user_list_entry_for(4, 4, "PAUSED", 1),
            user_list_entry_for(5, 5, "DROPPED", 2),
        ])
        .await;

        let entries = provider.user_list().await.expect("read");

        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].status, ListStatus::Current);
        assert_eq!(entries[1].status, ListStatus::Completed);
        assert_eq!(entries[2].status, ListStatus::Planning);
        assert_eq!(entries[3].status, ListStatus::Paused);
        assert_eq!(entries[4].status, ListStatus::Dropped);
    }

    /// The list-entry id has to survive: the row's remove action needs it, and
    /// the media id would target the wrong thing.
    #[tokio::test]
    async fn user_list_keeps_the_entry_id_and_progress() {
        let (_server, provider) = provider_for_collection(vec![user_list_entry(7, "CURRENT", 4)]).await;

        let entries = provider.user_list().await.expect("read");

        assert_eq!(entries[0].entry_id, 7);
        assert_eq!(entries[0].progress, 4);
        assert_eq!(entries[0].anime.id, 21);
    }

    /// A rewatch is not a separate tab here, so it lands under Current rather
    /// than being lost.
    #[tokio::test]
    async fn user_list_folds_rewatches_into_current() {
        let (_server, provider) = provider_for_collection(vec![user_list_entry(1, "REWATCHING", 5)]).await;

        let entries = provider.user_list().await.expect("read");

        assert_eq!(entries[0].status, ListStatus::Current);
    }

    /// A work in both a status list and a custom list arrives twice; the list
    /// page must show it once.
    #[tokio::test]
    async fn user_list_deduplicates_by_media() {
        // Same media fixture (id 21) in two groups.
        let (_server, provider) = provider_for_collection(vec![
            user_list_entry(1, "CURRENT", 3),
            user_list_entry(2, "PLANNING", 0),
        ])
        .await;

        let entries = provider.user_list().await.expect("read");

        assert_eq!(entries.len(), 1);
    }

    /// An entry with no id cannot be deleted, so it is dropped rather than
    /// rendering a row whose remove button is dead.
    #[tokio::test]
    async fn user_list_drops_entries_without_an_id() {
        let mut no_id = user_list_entry(1, "CURRENT", 3);
        no_id["id"] = serde_json::Value::Null;

        let (_server, provider) = provider_for_collection(vec![
            no_id,
            user_list_entry(2, "COMPLETED", 26),
        ])
        .await;

        let entries = provider.user_list().await.expect("read");

        // Only the media fixture has id 21, so dedupe alone would leave one --
        // this asserts the id-less one was skipped, not merely deduped.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_id, 2);
    }

    fn deleted_response(deleted: bool) -> serde_json::Value {
        serde_json::json!({ "data": { "DeleteMediaListEntry": { "deleted": deleted } } })
    }

    #[tokio::test]
    async fn deleting_a_list_entry_sends_the_entry_id() {
        let (server, provider) = provider_with(deleted_response(true), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider.delete_list_entry(7).await.expect("delete");

        assert_eq!(sent_variables(&server).await["id"], 7);
    }

    /// A delete that AniList refuses answers `deleted: false`, which must be an
    /// error rather than a silent success.
    #[tokio::test]
    async fn a_refused_delete_is_an_error() {
        let (_server, provider) = provider_with(deleted_response(false), 200).await;
        provider.set_token(Some("secret-token".into()));

        assert!(provider.delete_list_entry(7).await.is_err());
    }

    /// The outer field being null is also a failure.
    #[tokio::test]
    async fn a_null_delete_response_is_an_error() {
        let response = serde_json::json!({ "data": { "DeleteMediaListEntry": null } });
        let (_server, provider) = provider_with(response, 200).await;
        provider.set_token(Some("secret-token".into()));

        assert!(provider.delete_list_entry(7).await.is_err());
    }

    /// Anonymous until a token is set, which is how every request worked before
    /// sign-in existed.
    #[tokio::test]
    async fn requests_are_anonymous_without_a_token() {
        let (server, provider) = provider_with(page_response(vec![media_json()]), 200).await;

        provider.trending(1).await.unwrap();

        // An anonymous request must not carry a credential.
        assert_auth(&server, None).await;
    }

    #[tokio::test]
    async fn a_token_is_sent_as_a_bearer_credential() {
        let (server, provider) = provider_with(page_response(vec![media_json()]), 200).await;
        provider.set_token(Some("secret-token".into()));

        provider.trending(1).await.unwrap();

        assert_auth(&server, Some("Bearer secret-token")).await;
    }

    #[tokio::test]
    async fn clearing_the_token_makes_later_requests_anonymous() {
        let (server, provider) = provider_with(page_response(vec![media_json()]), 200).await;
        provider.set_token(Some("secret-token".into()));
        provider.clear_token();

        provider.trending(1).await.unwrap();

        assert_auth(&server, None).await;
    }

    /// A token pasted out of a browser address bar often carries whitespace,
    /// and `Bearer  secret ` would be rejected by the server.
    #[tokio::test]
    async fn a_token_is_trimmed_before_it_is_sent() {
        let (server, provider) = provider_with(page_response(vec![media_json()]), 200).await;
        provider.set_token(Some("  secret-token\n".into()));

        provider.trending(1).await.unwrap();

        assert_auth(&server, Some("Bearer secret-token")).await;
    }

    #[test]
    fn a_blank_token_is_not_stored() {
        let provider = AniListProvider::with_endpoint("http://localhost");

        provider.set_token(Some("   ".into()));

        // A blank bearer would make every request 401 instead of leaving it
        // anonymous, so it is discarded rather than sent.
        assert!(!provider.has_token());
    }

    #[test]
    fn has_token_reflects_the_stored_token() {
        let provider = AniListProvider::with_endpoint("http://localhost");
        assert!(!provider.has_token());

        provider.set_token(Some("secret-token".into()));
        assert!(provider.has_token());

        provider.set_token(None);
        assert!(!provider.has_token());
    }

    /// AniList reads the operation from the `query` field and takes the kind
    /// from the keyword INSIDE the document. A mutation sent under a
    /// `mutation` key is answered with "No query or mutation provided", which
    /// is the 400 this guards against.
    #[tokio::test]
    async fn a_mutation_is_sent_under_the_query_key() {
        let response = serde_json::json!({ "data": { "ok": true } });
        let (server, provider) = provider_with(response, 200).await;

        let _: serde_json::Value = provider
            .mutate(
                "mutation { SaveMediaListEntry(mediaId: 1) { id } }",
                serde_json::json!({}),
            )
            .await
            .expect("mutation should succeed");

        let requests = server
            .received_requests()
            .await
            .expect("requests should be recorded");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json body");

        assert!(
            body.get("query").is_some(),
            "the document must travel under the query key"
        );
        assert!(
            body.get("mutation").is_none(),
            "a `mutation` key is not something AniList reads"
        );
        // The keyword inside the string is what makes it a mutation.
        let document = body["query"].as_str().expect("query should be a string");
        assert!(
            document.contains("mutation"),
            "the document must still carry the mutation keyword"
        );
    }

    /// The whole point of the shared transport: a write carries the credential.
    #[tokio::test]
    async fn a_mutation_carries_the_token() {
        let response = serde_json::json!({ "data": { "ok": true } });
        let (server, provider) = provider_with(response, 200).await;
        provider.set_token(Some("secret-token".into()));

        let _: serde_json::Value = provider
            .mutate(
                "mutation { SaveMediaListEntry(mediaId: 1) { id } }",
                serde_json::json!({}),
            )
            .await
            .expect("mutation should succeed");

        assert_auth(&server, Some("Bearer secret-token")).await;
    }

    /// A rejected write must report an error rather than look like a success,
    /// which is the failure a separate mutation path could hide.
    #[tokio::test]
    async fn a_rejected_mutation_is_an_error() {
        let response = serde_json::json!({
            "data": null,
            "errors": [{ "message": "not authorized" }]
        });
        let (_server, provider) = provider_with(response, 200).await;

        let result: Result<serde_json::Value, ProviderError> = provider
            .mutate(
                "mutation { SaveMediaListEntry(mediaId: 1) { id } }",
                serde_json::json!({}),
            )
            .await;

        assert!(
            result.is_err(),
            "a GraphQL error must not be reported as a successful write"
        );
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

    /// A detail response carrying relations, recommendations and a trailer.
    fn detail_media_json() -> serde_json::Value {
        serde_json::json!({
            "id": 21,
            "title": { "romaji": "Attack on Titan" },
            "relations": { "edges": [
                {
                    "relationType": "SEQUEL",
                    "node": {
                        "id": 865,
                        "type": "ANIME",
                        "format": "TV",
                        "status": "FINISHED",
                        "episodes": 12,
                        "title": { "romaji": "Attack on Titan Season 2" },
                        "coverImage": { "large": "https://example.test/s2.jpg" }
                    }
                },
                {
                    "relationType": "SOURCE",
                    "node": {
                        "id": 999,
                        "type": "MANGA",
                        "title": { "romaji": "Attack on Titan (manga)" }
                    }
                }
            ] },
            "recommendations": { "edges": [
                {
                    "node": {
                        "rating": 42,
                        "mediaRecommendation": {
                            "id": 16498,
                            "type": "ANIME",
                            "title": { "romaji": "Fullmetal Alchemist: Brotherhood" },
                            "coverImage": { "large": "https://example.test/fmab.jpg" },
                            "format": "TV"
                        }
                    }
                },
                {
                    "node": { "rating": 1, "mediaRecommendation": null }
                }
            ] },
            "trailer": {
                "id": "LHtdKWJdif4",
                "site": "youtube",
                "thumbnail": "https://example.test/trailer.jpg"
            }
        })
    }

    #[tokio::test]
    async fn by_id_maps_relations_and_drops_manga_edges() {
        let response = serde_json::json!({ "data": { "Media": detail_media_json() } });
        let (_server, provider) = provider_with(response, 200).await;

        let anime = provider.by_id(21).await.unwrap().expect("should be found");

        // The manga edge is filtered out, leaving only the anime sequel.
        assert_eq!(anime.relations.len(), 1);
        let related = &anime.relations[0];
        assert_eq!(related.id, 865);
        assert_eq!(related.relation_type, "SEQUEL");
        assert_eq!(related.episode_count, Some(12));
    }

    #[tokio::test]
    async fn by_id_maps_recommendations_with_their_rating() {
        let response = serde_json::json!({ "data": { "Media": detail_media_json() } });
        let (_server, provider) = provider_with(response, 200).await;

        let anime = provider.by_id(21).await.unwrap().expect("should be found");

        // The empty recommendation is dropped, leaving one.
        assert_eq!(anime.recommendations.len(), 1);
        assert_eq!(anime.recommendations[0].rating, 42);
        assert_eq!(anime.recommendations[0].anime.id, 16498);
    }

    /// A downvoted recommendation must not fail the whole decode.
    ///
    /// AniList returns `rating: -1` for a suggestion users downvoted. Modelled as
    /// `u32`, that single field rejected the entire response, so a title whose
    /// recommendation list happened to contain one (My Hero Academia S2, among
    /// others) could not be opened at all -- while every other title worked.
    #[tokio::test]
    async fn by_id_accepts_a_negative_recommendation_rating() {
        let response = serde_json::json!({ "data": { "Media": {
            "id": 21856,
            "title": { "romaji": "Boku no Hero Academia 2" },
            "recommendations": { "edges": [
                {
                    "node": {
                        "rating": -1,
                        "mediaRecommendation": {
                            "id": 20521,
                            "type": "ANIME",
                            "title": { "romaji": "Hamatora THE ANIMATION" }
                        }
                    }
                },
                {
                    "node": {
                        "rating": 40,
                        "mediaRecommendation": {
                            "id": 20,
                            "type": "ANIME",
                            "title": { "romaji": "NARUTO" }
                        }
                    }
                }
            ] }
        } } });
        let (_server, provider) = provider_with(response, 200).await;

        let anime = provider
            .by_id(21856)
            .await
            .expect("a downvoted recommendation must not fail the decode")
            .expect("the title should be found");

        // Both edges survive, and the negative value is carried through as-is
        // rather than clamped: -1 is real data, not an error.
        assert_eq!(anime.recommendations.len(), 2);
        assert_eq!(anime.recommendations[0].rating, -1);
        assert_eq!(anime.recommendations[1].rating, 40);
    }

    /// The detail query must actually REQUEST the new fields.
    ///
    /// The mapping tests above feed a payload straight into `map_media`, so they
    /// pass even when the query never asks AniList for the data -- which is
    /// exactly the bug this guards. The request body is inspected directly.
    #[tokio::test]
    async fn by_id_query_requests_the_detail_fields() {
        let response = serde_json::json!({ "data": { "Media": media_json() } });
        let (server, provider) = provider_with(response, 200).await;

        provider.by_id(21).await.unwrap();

        let requests = server
            .received_requests()
            .await
            .expect("requests should be recorded");
        let body = String::from_utf8_lossy(&requests[0].body);
        assert!(body.contains("relations"), "query should ask for relations");
        assert!(
            body.contains("recommendations"),
            "query should ask for recommendations"
        );
        assert!(body.contains("trailer"), "query should ask for the trailer");
    }

    /// A recommended work is shown through `HoverPreview` on the detail page,
    /// and that panel renders the synopsis, score, year, episode count and
    /// genres. The mapping tests feed a payload straight in, so they pass even
    /// when the sub-query never asks for those -- which is exactly how the
    /// panel came to show a title and nothing else.
    #[tokio::test]
    async fn by_id_query_requests_the_preview_fields_per_recommendation() {
        let response = serde_json::json!({ "data": { "Media": detail_media_json() } });
        let (server, provider) = provider_with(response, 200).await;

        provider.by_id(21).await.unwrap();

        let requests = server
            .received_requests()
            .await
            .expect("requests should be recorded");
        let body = String::from_utf8_lossy(&requests[0].body);

        // Slice from `mediaRecommendation` onwards. A whole-body check would be
        // vacuous: this same request also carries `MEDIA_FIELDS`, which already
        // asks for genres, description, score and the rest for the work itself,
        // so every field below would be found whether or not the nested block
        // asked for it.
        let start = body
            .find("mediaRecommendation")
            .expect("the detail query should carry a recommendation sub-query");
        let recommendation = &body[start..];

        for field in [
            "genres",
            "description",
            "averageScore",
            "duration",
            "episodes",
            "status",
            "seasonYear",
            "native",
        ] {
            assert!(
                recommendation.contains(field),
                "the recommendation sub-query should ask for {field}"
            );
        }
    }

    /// And a list query must NOT request them, since the payload would bloat.
    #[tokio::test]
    async fn trending_query_omits_the_detail_fields() {
        let (server, provider) = provider_with(page_response(vec![media_json()]), 200).await;

        provider.trending(1).await.unwrap();

        let requests = server
            .received_requests()
            .await
            .expect("requests should be recorded");
        let body = String::from_utf8_lossy(&requests[0].body);
        assert!(
            !body.contains("recommendations"),
            "list query should be lean"
        );
    }

    #[tokio::test]
    async fn by_id_maps_the_trailer() {
        let response = serde_json::json!({ "data": { "Media": detail_media_json() } });
        let (_server, provider) = provider_with(response, 200).await;

        let anime = provider.by_id(21).await.unwrap().expect("should be found");
        let trailer = anime.trailer.expect("trailer should be present");
        assert_eq!(trailer.id, "LHtdKWJdif4");
        assert_eq!(trailer.site, "youtube");
    }

    /// A list query never asks for the detail fields, so they come back empty.
    #[tokio::test]
    async fn trending_leaves_the_detail_fields_empty() {
        let (_server, provider) = provider_with(page_response(vec![media_json()]), 200).await;

        let anime = provider.trending(1).await.unwrap();
        assert!(anime[0].relations.is_empty());
        assert!(anime[0].recommendations.is_empty());
        assert!(anime[0].trailer.is_none());
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
}
