//! Kitsu metadata client.
//!
//! Kitsu speaks JSON:API over `https://kitsu.io/api/edge`. Four details shape
//! this module:
//!
//! - **Ids are a different namespace.** A Kitsu id and an AniList id are not
//!   interchangeable: Cowboy Bebop is `1` on both (coincidence), but Attack on
//!   Titan is `7442` on Kitsu while its AniList id is `16498`. Nothing here
//!   pretends otherwise -- `Anime.id` carries the Kitsu id and
//!   `Anime.provider` says so.
//! - **`include=mappings` is unreliable on list endpoints.** Probed live:
//!   `/trending/anime?include=mappings` returns the relationship link but NO
//!   `included` array, so the parameter is silently ignored. It DOES work on
//!   `/anime/{id}` and on `/anime?filter[text]=...`. Only the single-title
//!   lookup therefore asks for mappings, and only it can fill `id_mal`.
//! - **`page[limit]` maxes at 20**, where AniList allows 50, so the clamp is
//!   per-provider and must not be shared.
//! - **The media type header is mandatory.** JSON:API servers are entitled to
//!   answer 406 without `Accept: application/vnd.api+json`.
//!
//! The client deliberately exposes only what the provider impl needs plus the
//! mapping lookup; there is no generic JSON:API layer here, because one
//! consumer does not justify one.

use std::time::Duration;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::traits::{AnimeProvider, ProviderError};
use crate::types::{
    Anime, AnimePage, BrowseQuery, EpisodeInfo, FormatFilter, ListFilter, MediaTag, PageInfo,
    ProviderId, RecommendationsPage, RelatedAnime, ScheduledEpisode, SeasonFilter, SortOption,
    StatusFilter, Title, Trailer,
};

/// Kitsu's public JSON:API base.
pub const KITSU_ENDPOINT: &str = "https://kitsu.io/api/edge";

/// The largest page Kitsu will serve.
///
/// Measured against the live API: `page[limit]=21` is rejected, `20` is
/// accepted. AniList's cap is 50, so this cannot be a shared constant.
pub const MAX_PAGE_LIMIT: u32 = 20;

/// How many related works one detail lookup will ask for.
///
/// Applied in the REQUEST, not by truncating the response: One Piece has 88
/// relations and a sidebar shows a handful, so asking for all of them wastes
/// bandwidth and JSON parsing for rows nobody scrolls to.
pub const MAX_RELATIONS: u32 = 12;

/// How many episode pages one catalogue request will walk.
///
/// Kitsu serves 20 episodes a page, so this caps a single detail-page open at
/// 5 requests / 100 episodes. A 1400-episode work would otherwise fire 70
/// requests for a grid the reader scrolls a few rows of. The AIRED COUNT is
/// still exact, because it comes from the first page's `meta.count`; only the
/// tail of the list is dropped.
pub const MAX_EPISODE_PAGES: u32 = 5;

/// How long to wait before retrying a rate-limited request.
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// How many times one request is retried on a 429 before giving up.
const MAX_RETRIES: u32 = 2;

/// The `externalSite` spelling Kitsu uses for AniList mappings.
pub const ANILIST_MAPPING_SITE: &str = "anilist/anime";

/// The `externalSite` spelling Kitsu uses for MyAnimeList mappings.
pub const MAL_MAPPING_SITE: &str = "myanimelist/anime";

/// A Kitsu REST client.
pub struct KitsuProvider {
    http: reqwest::Client,
    endpoint: String,
}

impl KitsuProvider {
    /// Build a client against the real Kitsu endpoint.
    pub fn new() -> Self {
        Self::with_endpoint(KITSU_ENDPOINT)
    }

    /// Build a client against an arbitrary base, so tests can point at a local
    /// `wiremock` server.
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            endpoint: endpoint.into(),
        }
    }

    /// Fetch one URL and decode it.
    ///
    /// The single place the media-type header, the retry, and the status check
    /// live, so a new endpoint cannot forget one of them.
    ///
    /// 429 is the one status worth retrying: it means "later", not "no".
    /// Every other failure is reported as-is rather than retried, so a caller
    /// is not left waiting on an error that will never clear.
    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, ProviderError> {
        let mut attempt = 0;

        loop {
            let response = self
                .http
                .get(url)
                .header(reqwest::header::ACCEPT, "application/vnd.api+json")
                .send()
                .await
                .map_err(|e| ProviderError::Transport(e.to_string()))?;

            let status = response.status();

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

            return serde_json::from_str(&text)
                .map_err(|e| ProviderError::Decode(format!("{e}; body was: {text}")));
        }
    }

    /// One page of anime, already mapped.
    ///
    /// `query` is appended verbatim to `/anime?`, so callers own their filters
    /// (text, season, sort, ...) without this method growing a parameter per
    /// filter.
    ///
    /// Mappings are deliberately NOT requested: Kitsu ignores the parameter on
    /// collection endpoints, so asking would only add noise. Consequence:
    /// results carry no `id_mal`, and episode enrichment is unavailable for
    /// them until a detail lookup fills it in.
    pub async fn anime_page(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Anime>, ProviderError> {
        let limit = clamp_limit(limit);
        let url = format!("{}/anime?{query}&page[limit]={limit}", self.endpoint);

        let envelope: CollectionEnvelope = self.get(&url).await?;

        Ok(envelope
            .data
            .into_iter()
            .filter(|resource| !resource.attributes.nsfw)
            .map(|resource| map_anime(&resource, &[]))
            .collect())
    }

    /// One page of anime, with the paging metadata the UI needs.
    ///
    /// Separate from [`Self::anime_page`] because only some callers can use a
    /// `PageInfo`; the home shelves just want the rows.
    ///
    /// `has_next_page` is derived from the presence of a `next` link rather
    /// than from arithmetic on `meta.count`, because Kitsu's offsets and
    /// limits do not always divide evenly and a wrong `false` strands the list.
    pub async fn anime_page_full(
        &self,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        let page = page.max(1);
        let limit = clamp_limit(per_page);
        let offset = (page - 1) * limit;
        let url = format!(
            "{}/anime?{query}&page[limit]={limit}&page[offset]={offset}",
            self.endpoint
        );

        let envelope: CollectionEnvelope = self.get(&url).await?;

        let items: Vec<Anime> = envelope
            .data
            .into_iter()
            .filter(|resource| !resource.attributes.nsfw)
            .map(|resource| map_anime(&resource, &[]))
            .collect();

        let has_next_page = envelope.links.next.is_some();
        // Kitsu reports the total in `meta.count`; when it is absent, the items
        // on this page are the most that can be claimed.
        let total = envelope.meta.count.max(items.len() as u32);
        let last_page = if limit == 0 {
            page
        } else {
            total.div_ceil(limit).max(1)
        };

        Ok(AnimePage {
            items,
            page_info: PageInfo {
                total,
                current_page: page,
                last_page,
                has_next_page,
            },
        })
    }

    /// One title, with the external ids Kitsu knows for it.
    ///
    /// This is the ONLY request that asks for mappings, because it is the only
    /// one where Kitsu honours the parameter.
    pub async fn anime_by_id(&self, kitsu_id: i64) -> Result<Option<Anime>, ProviderError> {
        let url = format!("{}/anime/{kitsu_id}?include=mappings", self.endpoint);

        let envelope: SingleEnvelope = self.get(&url).await?;

        let Some(resource) = envelope.data else {
            return Ok(None);
        };

        let mut anime = map_anime(&resource, &envelope.included);

        // Relations live behind their own endpoint, so they are fetched here
        // rather than mapped. A failure is NOT fatal: the work itself is what
        // the page is for, and losing the sidebar is better than losing the
        // page. It also keeps this to ONE extra request on the detail path.
        anime.relations = self.relations(kitsu_id).await.unwrap_or_default();

        Ok(Some(anime))
    }

    /// The ANILIST id for a Kitsu work, when Kitsu has a mapping for it.
    ///
    /// The mirror of [`Self::kitsu_id_for_anilist`], and needed by the sync
    /// engine: an entry added from a Kitsu card has no AniList id until this
    /// resolves one, and without it the entry can never be pushed.
    ///
    /// Answered from Kitsu's OWN mapping table, so it works while AniList is
    /// unreachable -- though the push that follows still needs AniList.
    pub async fn anilist_id_for_kitsu(
        &self,
        kitsu_id: i64,
    ) -> Result<Option<i64>, ProviderError> {
        let url = format!(
            "{}/anime/{kitsu_id}/mappings?filter[externalSite]={ANILIST_MAPPING_SITE}\
             &page[limit]=1",
            self.endpoint
        );

        let envelope: MappingEnvelope = self.get(&url).await?;

        // The mapping's `externalId` IS the AniList id, as a string.
        Ok(envelope
            .data
            .into_iter()
            .find(|mapping| mapping.attributes.external_site == ANILIST_MAPPING_SITE)
            .and_then(|mapping| mapping.attributes.external_id.parse().ok()))
    }

    /// The works Kitsu links to this one, for the detail page's sidebar.
    ///
    /// `include=destination` asks Kitsu to inline the RELATED work, so the
    /// titles and posters arrive in the same response as the link itself.
    /// Without it we would get bare destination ids and need a request per row.
    ///
    /// Capped at [`MAX_RELATIONS`]: One Piece has 88, and a sidebar shows a
    /// handful. The cap is applied by the REQUEST rather than by truncating
    /// afterwards, so the payload stays small too.
    pub async fn relations(&self, kitsu_id: i64) -> Result<Vec<RelatedAnime>, ProviderError> {
        let url = format!(
            "{}/anime/{kitsu_id}/media-relationships\
             ?include=destination&page[limit]={MAX_RELATIONS}",
            self.endpoint
        );

        let envelope: RelationshipEnvelope = self.get(&url).await?;

        // `included` holds the destination works; the edge carries only the
        // role. Joining them by id is what makes a row renderable.
        let mut destinations: std::collections::HashMap<String, &AnimeResource> =
            std::collections::HashMap::new();
        for resource in &envelope.included {
            // Only ANIME destinations are indexed. Kitsu mixes manga into the
            // same graph (One Piece links to its manga as `adaptation`), and a
            // manga row would open as an anime with an id from the wrong
            // namespace.
            if resource.kind == "anime" {
                destinations.insert(resource.id.clone(), resource);
            }
        }

        Ok(envelope
            .data
            .iter()
            .filter_map(|edge| {
                let role = non_empty(edge.attributes.role.clone())?;
                let destination_id = edge.destination_id()?;
                let resource = destinations.get(&destination_id)?;
                Some(map_related(resource, &role))
            })
            .collect())
    }

    /// The Kitsu id for an AniList id, when Kitsu has a mapping for it.
    ///
    /// Used by the sync layer to push a locally-recorded entry to AniList, and
    /// by nothing on the read path -- a read must not pay for this.
    ///
    /// Kitsu answers from its OWN mapping table, so this works whether or not
    /// AniList itself is reachable.
    pub async fn kitsu_id_for_anilist(
        &self,
        anilist_id: i64,
    ) -> Result<Option<i64>, ProviderError> {
        let url = format!(
            "{}/mappings?filter[externalSite]={ANILIST_MAPPING_SITE}\
             &filter[externalId]={anilist_id}&include=item&page[limit]=1",
            self.endpoint
        );

        let envelope: MappingEnvelope = self.get(&url).await?;

        // The item relationship carries the mapped anime's id directly, so the
        // `included` payload (which this call does not need) is ignored.
        Ok(envelope
            .data
            .into_iter()
            .find_map(|mapping| mapping.item_id()))
    }

    /// The episode catalogue for a Kitsu work, with per-episode stills.
    ///
    /// This is what replaces Jikan for episode lists. Verified against the live
    /// API: each entry carries `number`, `canonicalTitle`, `airdate` and a
    /// `thumbnail.original` URL that resolves to a real JPEG, which is what the
    /// episode cards render.
    ///
    /// Two deliberate limits:
    ///
    /// - **`aired` comes from `meta.count` on the FIRST page**, then the list is
    ///   trimmed to it. Kitsu's `meta.count` is the number of episode RECORDS,
    ///   and a record exists only once the episode has aired -- which is what
    ///   makes it usable as an aired count rather than an announced total.
    /// - **Paging stops at [`MAX_EPISODE_PAGES`].** A 1400-episode work would
    ///   otherwise issue 70 requests from one click. The count is still exact;
    ///   only the tail of the list is dropped, and the UI already renders a
    ///   truncated grid.
    pub async fn episodes(&self, kitsu_id: i64) -> Result<EpisodeCatalogue, ProviderError> {
        let mut all: Vec<EpisodeInfo> = Vec::new();
        let mut aired: u32 = 0;
        let mut page = 1u32;

        loop {
            let offset = (page - 1) * MAX_PAGE_LIMIT;
            let url = format!(
                "{}/anime/{kitsu_id}/episodes?page[limit]={MAX_PAGE_LIMIT}&page[offset]={offset}",
                self.endpoint
            );

            let envelope: EpisodeEnvelope = self.get(&url).await?;

            if page == 1 {
                // `meta.count` is the total number of records, which Kitsu only
                // creates for episodes that have aired. An absent count means
                // "unknown", not "zero" -- the empty list speaks for itself.
                aired = envelope.meta.count;
            }

            let before = all.len();
            all.extend(
                envelope
                    .data
                    .into_iter()
                    .filter_map(map_kitsu_episode),
            );

            // Stop on a short page, or when the guard trips. A page that added
            // nothing also stops: without that, a malformed `links.next` could
            // loop for MAX_EPISODE_PAGES requests that all return the same rows.
            let full_page = all.len() - before >= MAX_PAGE_LIMIT as usize;
            if !full_page || page >= MAX_EPISODE_PAGES {
                break;
            }
            page += 1;
        }

        all.sort_by_key(|episode| episode.number);

        Ok(EpisodeCatalogue { info: all, aired })
    }
}

/// A work's episode list plus how many have aired.
///
/// Two numbers rather than one because they answer different questions: `info`
/// is what to RENDER, `aired` is what to SAY. For a long runner the list is
/// capped by [`MAX_EPISODE_PAGES`] while `aired` stays exact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EpisodeCatalogue {
    /// The episodes fetched, sorted by number. May be shorter than `aired`.
    pub info: Vec<EpisodeInfo>,
    /// How many episodes have aired, from Kitsu's `meta.count`.
    pub aired: u32,
}

impl EpisodeCatalogue {
    /// Whether there is anything at all to show.
    pub fn is_empty(&self) -> bool {
        self.info.is_empty() && self.aired == 0
    }
}

impl Default for KitsuProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Clamp a requested page size to what Kitsu will serve.
///
/// Zero is not a valid limit -- Kitsu treats it as "use the default" -- so it
/// is floored at 1 to keep callers honest about what they asked for.
fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, MAX_PAGE_LIMIT)
}

/// A JSON:API collection (`data` is an array).
#[derive(Deserialize)]
struct CollectionEnvelope {
    #[serde(default)]
    data: Vec<AnimeResource>,
    /// Paging links. `next` is present exactly when another page exists, which
    /// is the only reliable "has more" signal Kitsu offers -- `meta.count` is a
    /// total, not a cursor.
    #[serde(default)]
    links: CollectionLinks,
    #[serde(default)]
    meta: CollectionMeta,
}

#[derive(Deserialize, Default)]
struct CollectionLinks {
    #[serde(default)]
    next: Option<String>,
}

#[derive(Deserialize, Default)]
struct CollectionMeta {
    /// Total matches across all pages.
    #[serde(default)]
    count: u32,
}

/// A JSON:API collection of episodes.
///
/// Shares [`CollectionMeta`] with the anime collection: `meta.count` means the
/// same thing on both -- how many records exist in total.
#[derive(Deserialize)]
struct EpisodeEnvelope {
    #[serde(default)]
    data: Vec<EpisodeResource>,
    #[serde(default)]
    meta: CollectionMeta,
}

#[derive(Deserialize)]
struct EpisodeResource {
    #[serde(default)]
    attributes: EpisodeAttributes,
}

/// The attributes this provider reads from an episode resource.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EpisodeAttributes {
    /// 1-based within the work, which is the number the rest of the app keys
    /// on. `relativeNumber` is the same for a single-cour work and is NOT used,
    /// because for a split cour it restarts and would collide.
    #[serde(default)]
    number: Option<u32>,
    #[serde(default)]
    canonical_title: Option<String>,
    /// `YYYY-MM-DD`, no time.
    #[serde(default)]
    airdate: Option<String>,
    /// Per-episode still. This is the whole point of using Kitsu here.
    #[serde(default)]
    thumbnail: Option<ImageSet>,
}

/// Map one Kitsu episode, dropping anything without a number.
///
/// An entry with no number cannot be placed in the grid or matched against a
/// release, so it is skipped rather than guessed at.
///
/// `filler` and `recap` are always `false`: Kitsu does not model them. Jikan did,
/// but it is gone, and inventing a guess would put a FILLER badge on the wrong
/// episodes.
fn map_kitsu_episode(resource: EpisodeResource) -> Option<EpisodeInfo> {
    let attrs = resource.attributes;
    let number = attrs.number?;

    Some(EpisodeInfo {
        number,
        title: non_empty(attrs.canonical_title),
        aired: non_empty(attrs.airdate),
        thumbnail: attrs
            .thumbnail
            .as_ref()
            .and_then(|image| pick_image(image)),
        filler: false,
        recap: false,
    })
}

/// A JSON:API single resource (`data` is one object, or null).
#[derive(Deserialize)]
struct SingleEnvelope {
    #[serde(default)]
    data: Option<AnimeResource>,
    /// Mapping resources, when the caller asked for `include=mappings`.
    ///
    /// Typed loosely on purpose: `included` may hold ANY related type, so
    /// entries are filtered by `type` rather than assumed to be mappings.
    #[serde(default)]
    included: Vec<MappingResource>,
}

/// A JSON:API collection of mappings.
#[derive(Deserialize)]
struct MappingEnvelope {
    #[serde(default)]
    data: Vec<MappingResource>,
}

/// One anime resource.
#[derive(Deserialize)]
struct AnimeResource {
    /// JSON:API ids are STRINGS. Parsed rather than trusted to be numeric.
    #[serde(default)]
    id: String,
    /// `"anime"` or `"manga"`. Load-bearing: Kitsu mixes BOTH into the relation
    /// graph, so without this a related MANGA row would render as a clickable
    /// anime and open the wrong entry. Verified live on One Piece:
    /// `adaptation | 38 | One Piece | manga`.
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    attributes: AnimeAttributes,
}

/// The attributes this provider reads from an anime resource.
///
/// Everything is defaulted: Kitsu omits fields freely (a film has no
/// `episodeCount`, an unrated work has no `averageRating`), and a missing field
/// must mean "unknown", not a failed decode.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AnimeAttributes {
    #[serde(default)]
    canonical_title: Option<String>,
    #[serde(default)]
    titles: AnimeTitles,
    /// Kitsu reports the same prose twice; `description` is the one it fills
    /// for older rows, `synopsis` for newer ones.
    #[serde(default)]
    synopsis: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    poster_image: Option<ImageSet>,
    #[serde(default)]
    cover_image: Option<ImageSet>,
    #[serde(default)]
    episode_count: Option<u32>,
    /// Episode length in MINUTES, despite the name.
    #[serde(default)]
    episode_length: Option<u32>,
    /// A decimal STRING, e.g. `"82.27"` on a 0-100 scale. Parsed, not assumed
    /// numeric -- serde would reject a JSON string for an `f64` field.
    #[serde(default)]
    average_rating: Option<String>,
    #[serde(default)]
    user_count: Option<u32>,
    #[serde(default)]
    start_date: Option<String>,
    /// `"current"`, `"finished"`, `"tba"`, `"upcoming"`.
    #[serde(default)]
    status: Option<String>,
    /// `"TV"`, `"movie"`, `"OVA"`, ...
    #[serde(default)]
    subtype: Option<String>,
    #[serde(default)]
    youtube_video_id: Option<String>,
    /// Hidden for unauthenticated requests, but present as `false` on the ones
    /// that do come back. Filtered at the collection boundary.
    #[serde(default)]
    nsfw: bool,
}

#[derive(Deserialize, Default)]
struct AnimeTitles {
    #[serde(default)]
    en: Option<String>,
    /// Kitsu's romaji field.
    #[serde(default)]
    en_jp: Option<String>,
    /// Native (Japanese) title.
    #[serde(default)]
    ja_jp: Option<String>,
}

#[derive(Deserialize, Default)]
struct ImageSet {
    #[serde(default)]
    large: Option<String>,
    #[serde(default)]
    original: Option<String>,
}

/// One mapping resource.
#[derive(Deserialize, Default)]
struct MappingResource {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    attributes: MappingAttributes,
    #[serde(default)]
    relationships: MappingRelationships,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MappingAttributes {
    #[serde(default)]
    external_site: String,
    #[serde(default)]
    external_id: String,
}

/// A collection of media relationships, with the destinations inlined.
#[derive(Deserialize)]
struct RelationshipEnvelope {
    #[serde(default)]
    data: Vec<RelationshipResource>,
    /// The destination works, when `include=destination` was honoured.
    #[serde(default)]
    included: Vec<AnimeResource>,
}

/// One `media-relationships` edge: the role, plus a pointer to the destination.
#[derive(Deserialize, Default)]
struct RelationshipResource {
    #[serde(default)]
    attributes: RelationshipAttributes,
    #[serde(default)]
    relationships: RelationshipEdges,
}

#[derive(Deserialize, Default)]
struct RelationshipAttributes {
    /// Kitsu's vocabulary: `sequel`, `prequel`, `side_story`, `summary`,
    /// `alternative_setting`, `parent_story`, `spin_off`, ...
    ///
    /// Surfaced verbatim, as [`RelatedAnime::relation_type`] documents: the set
    /// is open-ended and an enum here would drop any role Kitsu adds.
    #[serde(default)]
    role: Option<String>,
}

#[derive(Deserialize, Default)]
struct RelationshipEdges {
    #[serde(default)]
    destination: Option<Relationship>,
}

impl RelationshipResource {
    /// The destination work's id, as a string (JSON:API ids are strings).
    fn destination_id(&self) -> Option<String> {
        let data = self.relationships.destination.as_ref()?.data.as_ref()?;
        if data.id.is_empty() {
            None
        } else {
            Some(data.id.clone())
        }
    }
}

#[derive(Deserialize, Default)]
struct MappingRelationships {
    #[serde(default)]
    item: Option<Relationship>,
}

#[derive(Deserialize, Default)]
struct Relationship {
    #[serde(default)]
    data: Option<RelationshipData>,
}

#[derive(Deserialize, Default)]
struct RelationshipData {
    #[serde(default)]
    id: String,
}

impl MappingResource {
    /// The mapped anime's Kitsu id, when this mapping points at an anime.
    fn item_id(&self) -> Option<i64> {
        let data = self.relationships.item.as_ref()?.data.as_ref()?;
        data.id.parse().ok()
    }
}

/// Map a Kitsu resource onto the domain type.
///
/// `included` is empty for every collection response and only populated for the
/// single-title lookup, which is the only caller that can fill `id_mal`.
fn map_anime(resource: &AnimeResource, included: &[MappingResource]) -> Anime {
    let attrs = &resource.attributes;

    Anime {
        id: resource.id.parse().unwrap_or_default(),
        // NOT AniList: a Kitsu id is meaningless to AniList's API, and saying
        // otherwise would send list writes and progress records to the wrong
        // work.
        provider: ProviderId::Kitsu,
        id_mal: external_id(included, MAL_MAPPING_SITE),
        // Unknown from a LIST response: Kitsu exposes the aired count only
        // through `/anime/{id}/episodes`' `meta.count`, which is a separate
        // request. The detail page fills this in; a card does not need it.
        aired_episodes: None,
        title: map_title(attrs),
        cover_image: attrs
            .poster_image
            .as_ref()
            .and_then(|image| pick_image(image)),
        banner_image: attrs
            .cover_image
            .as_ref()
            .and_then(|image| pick_image(image)),
        description: non_empty(attrs.synopsis.clone())
            .or_else(|| non_empty(attrs.description.clone())),
        episode_count: attrs.episode_count,
        duration_minutes: attrs.episode_length,
        format: non_empty(attrs.subtype.clone()),
        // Categories live behind a relationship, so a list response carries no
        // genres. Fetching them per row would multiply the request count, so
        // they are left empty here and filled by the detail page's own call.
        genres: Vec::new(),
        average_score: parse_score(attrs.average_rating.as_deref()),
        popularity: attrs.user_count,
        status: normalize_status(attrs.status.as_deref()),
        season_year: parse_year(attrs.start_date.as_deref()),
        season: None,
        streaming_episodes: Vec::new(),
        relations: Vec::new(),
        recommendations: Vec::new(),
        trailer: map_trailer(attrs.youtube_video_id.as_deref()),
    }
}

/// Build the title, preferring Kitsu's canonical spelling as the fallback.
fn map_title(attrs: &AnimeAttributes) -> Title {
    let canonical = non_empty(attrs.canonical_title.clone());

    Title {
        romaji: non_empty(attrs.titles.en_jp.clone()).or_else(|| canonical.clone()),
        english: non_empty(attrs.titles.en.clone()),
        native: non_empty(attrs.titles.ja_jp.clone()),
        user_preferred: None,
    }
}

/// Map one related work for the detail sidebar.
///
/// Deliberately NOT a whole [`Anime`]: the row shows a poster, a title, the
/// relation label and a couple of facts. Going through `map_anime` would
/// allocate genres, descriptions and streaming links per row for nothing.
fn map_related(resource: &AnimeResource, role: &str) -> RelatedAnime {
    let attrs = &resource.attributes;

    RelatedAnime {
        // A KITSU id: clicking this row navigates by id, and the namespace has
        // to travel with it. `Anime.provider` does that for the row's own
        // `Anime`, and `CachedProvider` records each relation as a partial so
        // the click resolves to this source rather than defaulting to AniList.
        id: resource.id.parse().unwrap_or_default(),
        title: map_title(attrs),
        cover_image: attrs
            .poster_image
            .as_ref()
            .and_then(|image| pick_image(image)),
        format: non_empty(attrs.subtype.clone()),
        status: normalize_status(attrs.status.as_deref()),
        episode_count: attrs.episode_count,
        relation_type: role.to_string(),
    }
}

/// Pick the mapped external id for `site`.
fn external_id(included: &[MappingResource], site: &str) -> Option<i64> {
    included
        .iter()
        .filter(|mapping| mapping.kind == "mappings")
        .find(|mapping| mapping.attributes.external_site == site)
        .and_then(|mapping| mapping.attributes.external_id.parse().ok())
}

/// Kitsu's `"82.27"` -> `82`.
///
/// Clamped to `u8` because that is what the domain type carries, and a rogue
/// value above 100 should not wrap around to a low score.
fn parse_score(raw: Option<&str>) -> Option<u8> {
    let value: f64 = raw?.trim().parse().ok()?;
    if !(0.0..=100.0).contains(&value) {
        return None;
    }
    Some(value.round() as u8)
}

/// `"1998-04-03"` -> `1998`.
fn parse_year(raw: Option<&str>) -> Option<u32> {
    raw?.get(0..4)?.parse().ok()
}

/// Translate Kitsu's status vocabulary into the one the UI already renders.
///
/// AniList's spellings are reused deliberately: the frontend branches on
/// `"RELEASING"` / `"FINISHED"` / `"NOT_YET_RELEASED"`, and introducing a
/// second vocabulary would force every one of those branches to learn it.
fn normalize_status(raw: Option<&str>) -> Option<String> {
    let mapped = match raw? {
        "current" => "RELEASING",
        "finished" => "FINISHED",
        // Both "tba" and "upcoming" mean the same thing to a reader: not out
        // yet, no reliable date.
        "tba" | "upcoming" => "NOT_YET_RELEASED",
        _ => return None,
    };
    Some(mapped.to_string())
}

/// Build a trailer from Kitsu's bare YouTube id.
///
/// Kitsu stores only the video id, so the site is implied. A missing id means
/// no trailer rather than a trailer with nowhere to go.
///
/// The THUMBNAIL is derived rather than fetched: YouTube serves a still for
/// every video at a predictable URL, and Kitsu does not publish one. AniList
/// reports its own `thumbnail`, so both providers end up with the field
/// populated and the trailer card never renders as an empty box.
///
/// `hqdefault` deliberately, not `maxresdefault`: the larger still looks
/// sharper but is only generated for some uploads -- an absent one is served
/// as a tiny grey placeholder rather than a 404, which fails silently and
/// looks like a bug. `hqdefault` exists for every video. Verified live:
/// `img.youtube.com/vi/CmTeYj2FmRc/hqdefault.jpg` -> 200, image/jpeg.
fn map_trailer(youtube_id: Option<&str>) -> Option<Trailer> {
    let id = non_empty(youtube_id.map(str::to_string))?;
    let thumbnail = format!("https://img.youtube.com/vi/{id}/hqdefault.jpg");
    Some(Trailer {
        id,
        site: "youtube".to_string(),
        thumbnail: Some(thumbnail),
    })
}

/// Prefer `large` (card-sized) over `original` (often several megabytes).
fn pick_image(image: &ImageSet) -> Option<String> {
    non_empty(image.large.clone()).or_else(|| non_empty(image.original.clone()))
}

/// Treat blank strings as absent, as the AniList and Jikan mappers do.
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

#[async_trait]
impl AnimeProvider for KitsuProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Kitsu
    }

    async fn trending(&self, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        // `/trending/anime` is its own endpoint and takes a bare `limit`
        // rather than JSON:API paging. It returns a PLAIN array, not an
        // envelope, which is why it cannot go through `anime_page`.
        let limit = clamp_limit(limit);
        let url = format!("{}/trending/anime?limit={limit}", self.endpoint);

        let envelope: TrendingEnvelope = self.get(&url).await?;

        Ok(envelope
            .data
            .into_iter()
            .filter(|resource| !resource.attributes.nsfw)
            .map(|resource| map_anime(&resource, &[]))
            .collect())
    }

    async fn list(&self, filter: ListFilter, limit: u32) -> Result<Vec<Anime>, ProviderError> {
        if filter == ListFilter::Trending {
            return self.trending(limit).await;
        }
        // Reuse the paged path so the filter-to-query translation lives in one
        // place; only the first page is ever asked for here.
        let page = self.list_page(filter, 1, limit).await?;
        Ok(page.items)
    }

    async fn list_page(
        &self,
        filter: ListFilter,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        if filter == ListFilter::Trending {
            // Trending has no paging: the endpoint returns a single ranked
            // list. Presenting it as one page is honest -- there is no second.
            let items = self.trending(per_page).await?;
            let total = items.len() as u32;
            return Ok(AnimePage {
                items,
                page_info: PageInfo {
                    total,
                    current_page: 1,
                    last_page: 1,
                    has_next_page: false,
                },
            });
        }

        self.anime_page_full(&list_query(filter), page, per_page)
            .await
    }

    async fn browse(
        &self,
        query: BrowseQuery,
        page: u32,
        per_page: u32,
    ) -> Result<AnimePage, ProviderError> {
        self.anime_page_full(&browse_query(&query), page, per_page)
            .await
    }

    async fn by_id(&self, id: i64) -> Result<Option<Anime>, ProviderError> {
        self.anime_by_id(id).await
    }

    async fn by_id_in(
        &self,
        provider: ProviderId,
        id: i64,
    ) -> Result<Option<Anime>, ProviderError> {
        match provider {
            ProviderId::Kitsu => self.anime_by_id(id).await,
            // An AniList id is a DIFFERENT number for the same work, so it
            // cannot be sent to Kitsu directly: `16498` is Attack on Titan on
            // AniList and an unrelated title on Kitsu.
            ProviderId::AniList => self.by_anilist_id(id).await,
            // Every other namespace (TMDB, indexers) is not ours to resolve.
            _ => Ok(None),
        }
    }

    async fn by_anilist_id(&self, anilist_id: i64) -> Result<Option<Anime>, ProviderError> {
        // Two requests, only on the detail path: resolve the mapping, then
        // fetch. Both are answered by Kitsu, so this works during an AniList
        // outage -- which is the whole point.
        let Some(kitsu_id) = self.kitsu_id_for_anilist(anilist_id).await? else {
            return Ok(None);
        };
        self.anime_by_id(kitsu_id).await
    }

    async fn recommendations(
        &self,
        _id: i64,
        _page: u32,
        _per_page: u32,
    ) -> Result<RecommendationsPage, ProviderError> {
        // Kitsu has no recommendations endpoint. `mediaRelationships` is a
        // relation graph (sequels, side stories), which is a different thing
        // and is already served by `by_id`.
        //
        // An empty page rather than an error: the caller renders "nothing here"
        // either way, and an error would make a Kitsu-sourced detail page look
        // broken rather than sparse.
        Ok(RecommendationsPage {
            items: Vec::new(),
            page_info: PageInfo {
                total: 0,
                current_page: 1,
                last_page: 1,
                has_next_page: false,
            },
        })
    }

    async fn genres(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/genres?page[limit]={MAX_PAGE_LIMIT}", self.endpoint);
        let envelope: GenreEnvelope = self.get(&url).await?;

        Ok(envelope
            .data
            .into_iter()
            .filter_map(|genre| non_empty(genre.attributes.title))
            .collect())
    }

    async fn tags(&self) -> Result<Vec<MediaTag>, ProviderError> {
        // Kitsu models genres as a flat list and has no tag namespace with
        // categories or descriptions, which is the shape `MediaTag` carries.
        // Returning an empty catalogue makes the tag filter disappear rather
        // than offering names that would match nothing.
        Ok(Vec::new())
    }

    async fn schedule(
        &self,
        _from: i64,
        _to: i64,
        _limit: u32,
    ) -> Result<Vec<ScheduledEpisode>, ProviderError> {
        // Kitsu publishes no airing schedule. The failover decorator routes
        // this call to AniList unconditionally, so this body is only reachable
        // when Kitsu is the SELECTED source -- in which case an empty schedule
        // is the truthful answer.
        Ok(Vec::new())
    }
}

/// The plain-array envelope `/trending/anime` returns.
#[derive(Deserialize)]
struct TrendingEnvelope {
    #[serde(default)]
    data: Vec<AnimeResource>,
}

/// A collection of genre resources.
#[derive(Deserialize)]
struct GenreEnvelope {
    #[serde(default)]
    data: Vec<GenreResource>,
}

#[derive(Deserialize)]
struct GenreResource {
    #[serde(default)]
    attributes: GenreAttributes,
}

#[derive(Deserialize, Default)]
struct GenreAttributes {
    #[serde(default)]
    title: Option<String>,
}

/// Translate a curated-list intent into Kitsu's own sort and filters.
///
/// Split out so it can be tested without a server, and so `list` and
/// `list_page` cannot drift apart.
///
/// Kitsu's `-` prefix means descending. Status values are Kitsu's lowercase
/// spellings, not AniList's.
fn list_query(filter: ListFilter) -> String {
    match filter {
        // Handled by the caller via `/trending/anime`; kept for exhaustiveness.
        ListFilter::Trending => "sort=-userCount".to_string(),
        ListFilter::TopAiring => "filter[status]=current&sort=-userCount".to_string(),
        ListFilter::MostPopular => "sort=-userCount".to_string(),
        ListFilter::TopRated => "sort=-averageRating".to_string(),
        ListFilter::LatestCompleted => "filter[status]=finished&sort=-startDate".to_string(),
        ListFilter::Upcoming => "filter[status]=upcoming&sort=-userCount".to_string(),
    }
}

/// Build the query string for a browse request.
///
/// Only filters Kitsu actually supports are emitted. Tags are dropped rather
/// than translated: Kitsu has no tag namespace, and `genres` is the closest
/// equivalent, so silently widening a tag filter into a genre filter would
/// return a different set than the reader asked for.
fn browse_query(query: &BrowseQuery) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("filter[text]={}", urlencode(search)));
    }

    for genre in &query.genres {
        // Kitsu takes categories as slugs. Lowercasing is the documented
        // mapping; a multi-word genre keeps its spaces, which urlencode
        // escapes.
        parts.push(format!("filter[categories]={}", urlencode(&genre.to_lowercase())));
    }

    if let Some(format) = query.format {
        parts.push(format!("filter[subtype]={}", format_slug(format)));
    }

    if let Some(status) = query.status {
        parts.push(format!("filter[status]={}", status_slug(status)));
    }

    if let Some(season) = query.season {
        parts.push(format!("filter[season]={}", season_slug(season)));
    }

    if let Some(year) = query.season_year {
        parts.push(format!("filter[seasonYear]={year}"));
    }

    // Kitsu has no score floor, and faking one client-side would silently
    // truncate a page. It is dropped, and the UI hides the control for Kitsu.
    let _ = query.min_score;

    parts.push(format!("sort={}", sort_slug(query.sort, query.reversed)));

    parts.join("&")
}

/// Kitsu's subtype vocabulary, lowercased.
fn format_slug(format: FormatFilter) -> &'static str {
    match format {
        FormatFilter::Tv => "TV",
        FormatFilter::Movie => "movie",
        FormatFilter::Ova => "OVA",
        FormatFilter::Ona => "ONA",
        FormatFilter::Special => "special",
        FormatFilter::Music => "music",
    }
}

/// Kitsu's airing-status vocabulary.
fn status_slug(status: StatusFilter) -> &'static str {
    match status {
        StatusFilter::Releasing => "current",
        StatusFilter::Finished => "finished",
        StatusFilter::NotYetReleased => "upcoming",
    }
}

/// Kitsu's season vocabulary.
fn season_slug(season: SeasonFilter) -> &'static str {
    match season {
        SeasonFilter::Winter => "winter",
        SeasonFilter::Spring => "spring",
        SeasonFilter::Summer => "summer",
        SeasonFilter::Fall => "fall",
    }
}

/// Kitsu's sort vocabulary, with the `-` descending prefix.
///
/// Each variant has a NATURAL direction, so `reversed` flips it rather than
/// naming a direction -- the same contract the other providers follow.
fn sort_slug(sort: SortOption, reversed: bool) -> String {
    let (field, natural_desc) = match sort {
        SortOption::Popularity | SortOption::Favorites => ("userCount", true),
        SortOption::Score => ("averageRating", true),
        SortOption::Newest | SortOption::DateAdded => ("startDate", true),
        SortOption::TitleAz => ("canonicalTitle", false),
        // Trending has no sort field; popularity is the closest honest proxy.
        SortOption::Trending => ("userCount", true),
        // Kitsu exposes no relevance ordering. The match itself is done by
        // `filter[text]`, so ordering by popularity keeps the best-known
        // matches on the first page rather than burying them.
        SortOption::SearchMatch => ("userCount", true),
    };

    let descending = natural_desc != reversed;
    if descending {
        format!("-{field}")
    } else {
        field.to_string()
    }
}

/// Percent-encode a query-string value.
///
/// Hand-rolled rather than pulling a dependency: the set that must be escaped
/// in a query value is small and fixed, and every byte outside it (notably
/// UTF-8 in a search term) is escaped byte-wise, which is what the URL spec
/// requires.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// A realistic anime resource. Values copied from a live response.
    fn anime_json(id: &str, nsfw: bool) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "type": "anime",
            "attributes": {
                "slug": "attack-on-titan",
                "synopsis": "Centuries ago, mankind was slaughtered.",
                "description": "Older duplicate prose.",
                "canonicalTitle": "Attack on Titan",
                "titles": {
                    "en": "Attack on Titan",
                    "en_jp": "Shingeki no Kyojin",
                    "ja_jp": "進撃の巨人"
                },
                "averageRating": "84.4",
                "userCount": 617909,
                "startDate": "2013-04-07",
                "status": "finished",
                "subtype": "TV",
                "posterImage": {
                    "large": "https://media.kitsu.app/anime/poster_images/7442/large.jpg",
                    "original": "https://media.kitsu.app/anime/poster_images/7442/original.jpg"
                },
                "coverImage": {
                    "large": "https://media.kitsu.app/anime/cover_images/7442/large.jpg",
                    "original": "https://media.kitsu.app/anime/cover_images/7442/original.png"
                },
                "episodeCount": 25,
                "episodeLength": 24,
                "youtubeVideoId": "LHtdKWJdif4",
                "nsfw": nsfw
            }
        })
    }

    fn mapping_json(site: &str, external_id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "254628",
            "type": "mappings",
            "attributes": { "externalSite": site, "externalId": external_id }
        })
    }

    #[tokio::test]
    async fn anime_page_maps_fields() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .and(header("accept", "application/vnd.api+json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [anime_json("7442", false)]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let items = provider.anime_page("", 5).await.unwrap();

        assert_eq!(items.len(), 1);
        let anime = &items[0];
        assert_eq!(anime.id, 7442);
        assert_eq!(anime.provider, ProviderId::Kitsu);
        assert_eq!(anime.title.romaji.as_deref(), Some("Shingeki no Kyojin"));
        assert_eq!(anime.title.english.as_deref(), Some("Attack on Titan"));
        assert_eq!(anime.title.native.as_deref(), Some("進撃の巨人"));
        assert_eq!(anime.average_score, Some(84));
        assert_eq!(anime.episode_count, Some(25));
        assert_eq!(anime.duration_minutes, Some(24));
        assert_eq!(anime.season_year, Some(2013));
        assert_eq!(anime.status.as_deref(), Some("FINISHED"));
        assert_eq!(anime.format.as_deref(), Some("TV"));
        assert_eq!(anime.description.as_deref(), Some("Centuries ago, mankind was slaughtered."));
        assert_eq!(anime.trailer.as_ref().map(|t| t.id.as_str()), Some("LHtdKWJdif4"));
        assert_eq!(anime.trailer.as_ref().map(|t| t.site.as_str()), Some("youtube"));
        assert!(anime.cover_image.as_deref().unwrap().ends_with("large.jpg"));
        assert!(anime.banner_image.as_deref().unwrap().ends_with("large.jpg"));
    }

    #[tokio::test]
    async fn anime_page_drops_nsfw_entries() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [anime_json("1", false), anime_json("2", true)]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let items = provider.anime_page("", 5).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, 1);
    }

    #[tokio::test]
    async fn list_results_never_carry_a_mal_id() {
        // Kitsu IGNORES `include=mappings` on collection endpoints, so a list
        // response cannot populate `id_mal`. Guard the consequence: the mapper
        // must not invent one.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [anime_json("7442", false)]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let items = provider.anime_page("filter[text]=titan", 5).await.unwrap();

        assert_eq!(items[0].id_mal, None);
    }

    #[tokio::test]
    async fn by_id_reads_the_mal_mapping() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/7442"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": anime_json("7442", false),
                "included": [
                    mapping_json("thetvdb/series", "267440"),
                    mapping_json(MAL_MAPPING_SITE, "16498"),
                    mapping_json(ANILIST_MAPPING_SITE, "16498")
                ]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let anime = provider.anime_by_id(7442).await.unwrap().unwrap();

        assert_eq!(anime.id_mal, Some(16498));
        // The Kitsu id stays the Kitsu id: mapping is not re-keying.
        assert_eq!(anime.id, 7442);
        assert_eq!(anime.provider, ProviderId::Kitsu);
    }

    #[tokio::test]
    async fn by_id_without_mappings_leaves_mal_id_empty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/7442"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": anime_json("7442", false),
                "included": []
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let anime = provider.anime_by_id(7442).await.unwrap().unwrap();

        assert_eq!(anime.id_mal, None);
    }

    #[tokio::test]
    async fn by_id_returns_none_when_data_is_null() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/999999"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": null })),
            )
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        assert!(provider.anime_by_id(999999).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn reverse_mapping_returns_the_kitsu_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/mappings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "id": "254628",
                    "type": "mappings",
                    "attributes": {
                        "externalSite": ANILIST_MAPPING_SITE,
                        "externalId": "16498"
                    },
                    "relationships": {
                        "item": { "data": { "type": "anime", "id": "7442" } }
                    }
                }]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let kitsu_id = provider.kitsu_id_for_anilist(16498).await.unwrap();

        assert_eq!(kitsu_id, Some(7442));
    }

    #[tokio::test]
    async fn reverse_mapping_returns_none_when_kitsu_has_no_row() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/mappings"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": [] })),
            )
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        assert_eq!(provider.kitsu_id_for_anilist(1).await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_rate_limit_is_retried_then_reported() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let error = provider.anime_page("", 5).await.unwrap_err();

        match error {
            ProviderError::Status { status, .. } => assert_eq!(status, 429),
            other => panic!("expected a 429, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_server_error_is_not_retried() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .respond_with(ResponseTemplate::new(503).set_body_string("down"))
            .expect(1)
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let error = provider.anime_page("", 5).await.unwrap_err();

        match error {
            ProviderError::Status { status, body } => {
                assert_eq!(status, 503);
                assert_eq!(body, "down");
            }
            other => panic!("expected a 503, got {other:?}"),
        }
    }

    #[test]
    fn every_list_filter_maps_to_its_own_query() {
        // Each variant must produce a DISTINCT query: two shelves sharing one
        // would render the same rows under different headings.
        let filters = [
            ListFilter::Trending,
            ListFilter::TopAiring,
            ListFilter::MostPopular,
            ListFilter::TopRated,
            ListFilter::LatestCompleted,
            ListFilter::Upcoming,
        ];

        let queries: Vec<String> = filters.iter().map(|f| list_query(*f)).collect();

        for (index, query) in queries.iter().enumerate() {
            assert!(!query.is_empty(), "empty query for {:?}", filters[index]);
        }

        // Trending is served by its own endpoint, so it is excluded from the
        // uniqueness check; the rest must not collide.
        let non_trending: std::collections::HashSet<&String> = queries[1..].iter().collect();
        assert_eq!(
            non_trending.len(),
            queries.len() - 1,
            "two list filters produced the same query: {queries:?}"
        );
    }

    #[test]
    fn top_rated_sorts_by_rating_not_popularity() {
        let query = list_query(ListFilter::TopRated);
        assert!(query.contains("sort=-averageRating"), "got {query}");
        assert!(!query.contains("userCount"), "got {query}");
    }

    #[test]
    fn top_airing_restricts_to_current_shows() {
        let query = list_query(ListFilter::TopAiring);
        assert!(query.contains("filter[status]=current"), "got {query}");
    }

    #[test]
    fn browse_encodes_a_search_term() {
        let query = BrowseQuery {
            search: Some("cowboy bebop".to_string()),
            ..Default::default()
        };
        let built = browse_query(&query);
        assert!(built.contains("filter[text]=cowboy+bebop"), "got {built}");
    }

    #[test]
    fn browse_blank_search_is_omitted() {
        let query = BrowseQuery {
            search: Some("   ".to_string()),
            ..Default::default()
        };
        assert!(!browse_query(&query).contains("filter[text]"));
    }

    #[test]
    fn browse_encodes_non_ascii_byte_wise() {
        // A search term with a non-ASCII character must be percent-encoded
        // byte-by-byte, not passed through raw.
        assert_eq!(urlencode("進撃"), "%E9%80%B2%E6%92%83");
    }

    #[test]
    fn browse_reversed_flips_the_natural_direction() {
        let natural = browse_query(&BrowseQuery {
            sort: SortOption::Score,
            ..Default::default()
        });
        assert!(natural.contains("sort=-averageRating"), "got {natural}");

        let reversed = browse_query(&BrowseQuery {
            sort: SortOption::Score,
            reversed: true,
            ..Default::default()
        });
        assert!(reversed.contains("sort=averageRating"), "got {reversed}");
        assert!(!reversed.contains("sort=-averageRating"), "got {reversed}");
    }

    #[test]
    fn title_sort_is_ascending_by_default() {
        let query = browse_query(&BrowseQuery {
            sort: SortOption::TitleAz,
            ..Default::default()
        });
        assert!(query.contains("sort=canonicalTitle"), "got {query}");
        assert!(!query.contains("sort=-canonicalTitle"), "got {query}");
    }

    #[test]
    fn browse_drops_tags_rather_than_widening_them_into_genres() {
        // Kitsu has no tag namespace. Silently treating "Isekai" as a genre
        // would return a different set than the reader asked for.
        let query = BrowseQuery {
            tags: vec!["Isekai".to_string()],
            excluded_tags: vec!["Time Loop".to_string()],
            ..Default::default()
        };
        let built = browse_query(&query);
        assert!(!built.contains("Isekai"), "got {built}");
        assert!(!built.contains("Time"), "got {built}");
    }

    #[tokio::test]
    async fn list_page_reports_has_next_from_the_next_link() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [anime_json("7442", false)],
                "meta": { "count": 45 },
                "links": { "next": "https://kitsu.io/api/edge/anime?page[offset]=20" }
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let page = provider
            .list_page(ListFilter::TopRated, 1, 20)
            .await
            .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.page_info.total, 45);
        assert_eq!(page.page_info.current_page, 1);
        assert_eq!(page.page_info.last_page, 3, "45 items at 20 per page");
        assert!(page.page_info.has_next_page);
    }

    #[tokio::test]
    async fn last_page_has_no_next() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [anime_json("7442", false)],
                "meta": { "count": 1 },
                "links": {}
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let page = provider
            .list_page(ListFilter::TopRated, 1, 20)
            .await
            .unwrap();

        assert!(!page.page_info.has_next_page);
        assert_eq!(page.page_info.last_page, 1);
    }

    #[tokio::test]
    async fn trending_uses_its_own_endpoint_and_never_pages() {
        let server = MockServer::start().await;
        // A PLAIN array, not a JSON:API envelope: this endpoint is the odd one.
        Mock::given(method("GET"))
            .and(path("/trending/anime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [anime_json("7442", false)]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let items = provider.trending(5).await.unwrap();

        assert_eq!(items.len(), 1);

        // Asking for page 2 of trending must not invent one.
        let page = provider
            .list_page(ListFilter::Trending, 2, 5)
            .await
            .unwrap();
        assert!(!page.page_info.has_next_page);
        assert_eq!(page.page_info.current_page, 1);
    }

    #[tokio::test]
    async fn recommendations_are_an_empty_page_not_an_error() {
        let provider = KitsuProvider::with_endpoint("http://unused.invalid");
        let page = provider.recommendations(1, 1, 24).await.unwrap();

        assert!(page.items.is_empty());
        assert_eq!(page.page_info.total, 0);
    }

    #[tokio::test]
    async fn tags_and_schedule_are_honestly_empty() {
        // Kitsu has neither a tag namespace nor an airing schedule. Both must
        // report empty rather than reach the network.
        let provider = KitsuProvider::with_endpoint("http://unused.invalid");

        assert!(provider.tags().await.unwrap().is_empty());
        assert!(provider.schedule(0, i64::MAX, 50).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn genres_are_read_from_the_genres_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/genres"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    { "id": "1", "type": "genres", "attributes": { "title": "Action" } },
                    { "id": "2", "type": "genres", "attributes": { "title": "Romance" } },
                    { "id": "3", "type": "genres", "attributes": { "title": null } }
                ]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let genres = provider.genres().await.unwrap();

        assert_eq!(genres, vec!["Action".to_string(), "Romance".to_string()]);
    }

    #[tokio::test]
    async fn relations_join_the_edge_role_to_the_included_destination() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/12/media-relationships"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {
                        "id": "1",
                        "type": "mediaRelationships",
                        "attributes": { "role": "sequel" },
                        "relationships": {
                            "destination": { "data": { "type": "anime", "id": "1906" } }
                        }
                    },
                    {
                        "id": "2",
                        "type": "mediaRelationships",
                        "attributes": { "role": "summary" },
                        "relationships": {
                            "destination": { "data": { "type": "anime", "id": "5225" } }
                        }
                    }
                ],
                "included": [
                    {
                        "id": "1906",
                        "type": "anime",
                        "attributes": {
                            "canonicalTitle": "One Piece Movie 8",
                            "titles": { "en_jp": "One Piece Movie 8" },
                            "subtype": "movie",
                            "status": "finished",
                            "episodeCount": 1,
                            "posterImage": { "large": "https://media.kitsu.app/m8.jpg" }
                        }
                    },
                    {
                        "id": "5225",
                        "type": "anime",
                        "attributes": {
                            "canonicalTitle": "One Piece Recap",
                            "titles": { "en_jp": "One Piece Recap" },
                            "subtype": "OVA",
                            "status": "finished"
                        }
                    }
                ]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let relations = provider.relations(12).await.unwrap();

        assert_eq!(relations.len(), 2);
        assert_eq!(relations[0].id, 1906);
        assert_eq!(relations[0].relation_type, "sequel");
        assert_eq!(
            relations[0].title.romaji.as_deref(),
            Some("One Piece Movie 8")
        );
        assert_eq!(relations[0].format.as_deref(), Some("movie"));
        assert_eq!(relations[0].episode_count, Some(1));
        assert!(relations[0].cover_image.is_some());
        assert_eq!(relations[1].relation_type, "summary");
    }

    #[tokio::test]
    async fn relations_drop_edges_whose_destination_is_missing() {
        // An edge with no inlined destination cannot be rendered: there is no
        // title and no poster. Dropping it beats a blank row.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/12/media-relationships"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {
                        "id": "1",
                        "type": "mediaRelationships",
                        "attributes": { "role": "sequel" },
                        "relationships": {
                            "destination": { "data": { "type": "anime", "id": "999" } }
                        }
                    }
                ],
                "included": []
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        assert!(provider.relations(12).await.unwrap().is_empty());
    }

    /// Kitsu mixes MANGA into the relation graph — verified live on One Piece,
    /// which links to its own manga as `adaptation`. A manga row rendered as an
    /// anime would be clickable and would open an id from a namespace that has
    /// no entry for it, so the filter is load-bearing, not cosmetic.
    #[test]
    fn a_trailer_carries_a_derived_youtube_thumbnail() {
        // Without this the detail page renders an empty box with a play
        // triangle: the card binds its Image to `trailer.thumbnail`.
        let trailer = map_trailer(Some("CmTeYj2FmRc")).expect("a trailer");

        assert_eq!(trailer.id, "CmTeYj2FmRc");
        assert_eq!(trailer.site, "youtube");
        assert_eq!(
            trailer.thumbnail.as_deref(),
            Some("https://img.youtube.com/vi/CmTeYj2FmRc/hqdefault.jpg")
        );
    }

    #[test]
    fn a_blank_youtube_id_yields_no_trailer() {
        assert!(map_trailer(Some("   ")).is_none());
        assert!(map_trailer(None).is_none());
    }

    #[tokio::test]
    async fn relations_skip_manga_destinations() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/12/media-relationships"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {
                        "id": "1",
                        "type": "mediaRelationships",
                        "attributes": { "role": "adaptation" },
                        "relationships": {
                            "destination": { "data": { "type": "manga", "id": "38" } }
                        }
                    },
                    {
                        "id": "2",
                        "type": "mediaRelationships",
                        "attributes": { "role": "side_story" },
                        "relationships": {
                            "destination": { "data": { "type": "anime", "id": "421" } }
                        }
                    }
                ],
                "included": [
                    {
                        "id": "38",
                        "type": "manga",
                        "attributes": {
                            "canonicalTitle": "One Piece",
                            "titles": { "en_jp": "One Piece" },
                            "subtype": "manga"
                        }
                    },
                    {
                        "id": "421",
                        "type": "anime",
                        "attributes": {
                            "canonicalTitle": "One Piece Movie 1",
                            "titles": { "en_jp": "One Piece Movie 1" },
                            "subtype": "movie"
                        }
                    }
                ]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let relations = provider.relations(12).await.unwrap();

        assert_eq!(relations.len(), 1, "the manga row must be dropped");
        assert_eq!(relations[0].id, 421);
        assert_eq!(relations[0].relation_type, "side_story");
    }

    #[tokio::test]
    async fn relations_drop_edges_without_a_role() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/12/media-relationships"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "id": "1",
                    "type": "mediaRelationships",
                    "attributes": {},
                    "relationships": {
                        "destination": { "data": { "type": "anime", "id": "1906" } }
                    }
                }],
                "included": [{
                    "id": "1906",
                    "type": "anime",
                    "attributes": { "canonicalTitle": "X", "titles": {} }
                }]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        assert!(
            provider.relations(12).await.unwrap().is_empty(),
            "a row with no relation label says nothing"
        );
    }

    #[tokio::test]
    async fn by_id_in_uses_the_kitsu_id_for_the_kitsu_namespace() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/anime/7442"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": anime_json("7442", false),
                "included": []
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let anime = provider
            .by_id_in(ProviderId::Kitsu, 7442)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(anime.id, 7442);
    }

    /// The bug the reader hit: a card rendered by Kitsu carries a KITSU id, so
    /// opening it must not ask Kitsu for that number in the ANILIST namespace.
    #[tokio::test]
    async fn by_id_in_resolves_an_anilist_id_through_the_mapping_table() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/mappings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "id": "254628",
                    "type": "mappings",
                    "attributes": {
                        "externalSite": ANILIST_MAPPING_SITE,
                        "externalId": "16498"
                    },
                    "relationships": {
                        "item": { "data": { "type": "anime", "id": "7442" } }
                    }
                }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/anime/7442"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": anime_json("7442", false),
                "included": [mapping_json(MAL_MAPPING_SITE, "16498")]
            })))
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        let anime = provider
            .by_id_in(ProviderId::AniList, 16498)
            .await
            .unwrap()
            .unwrap();

        // Resolved through the table, NOT treated as a Kitsu id.
        assert_eq!(anime.id, 7442);
        assert_eq!(anime.id_mal, Some(16498));
    }

    #[tokio::test]
    async fn by_id_in_ignores_namespaces_it_cannot_address() {
        let provider = KitsuProvider::with_endpoint("http://unused.invalid");
        assert!(provider.by_id_in(ProviderId::Tmdb, 1).await.unwrap().is_none());
        assert!(provider.by_id_in(ProviderId::Nyaa, 1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn by_anilist_id_returns_none_when_no_mapping_exists() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/mappings"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": [] })),
            )
            .mount(&server)
            .await;

        let provider = KitsuProvider::with_endpoint(server.uri());
        assert!(provider.by_anilist_id(999999).await.unwrap().is_none());
    }

    #[test]
    fn clamp_limit_keeps_requests_within_kitsu_bounds() {
        assert_eq!(clamp_limit(0), 1, "zero is not a valid limit");
        assert_eq!(clamp_limit(10), 10);
        assert_eq!(clamp_limit(20), 20);
        assert_eq!(clamp_limit(50), 20, "Kitsu rejects page[limit] above 20");
    }

    #[test]
    fn score_is_parsed_and_rounded() {
        assert_eq!(parse_score(Some("84.4")), Some(84));
        assert_eq!(parse_score(Some("82.27")), Some(82));
        assert_eq!(parse_score(Some("")), None);
        assert_eq!(parse_score(None), None);
        // A nonsense value must not wrap around a u8.
        assert_eq!(parse_score(Some("900")), None);
    }

    #[test]
    fn status_vocabulary_matches_anilists() {
        assert_eq!(normalize_status(Some("current")).as_deref(), Some("RELEASING"));
        assert_eq!(normalize_status(Some("finished")).as_deref(), Some("FINISHED"));
        assert_eq!(
            normalize_status(Some("upcoming")).as_deref(),
            Some("NOT_YET_RELEASED")
        );
        assert_eq!(normalize_status(Some("tba")).as_deref(), Some("NOT_YET_RELEASED"));
        assert_eq!(normalize_status(Some("weird")), None);
        assert_eq!(normalize_status(None), None);
    }
}
