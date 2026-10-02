//! Shared domain types passed between the providers, the torrent layer and
//! the frontend.
//!
//! These are deliberately provider-agnostic: `Anime` describes a work as the
//! UI needs it, not as AniList or TMDB happen to shape their JSON. Each
//! provider is responsible for mapping its own response into these types.
//!
//! All types serialise to camelCase so the Svelte side can consume them
//! without a translation layer.

use serde::{Deserialize, Serialize};

/// Which upstream source an id belongs to.
///
/// Carried alongside every id so that, once movie providers exist, an AniList
/// id can never be mistaken for a TMDB id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    /// <https://anilist.co>
    AniList,
    /// <https://www.themoviedb.org>
    Tmdb,
    /// <https://nyaa.si> — torrent index, not a metadata source.
    Nyaa,
    /// <https://yts.mx> — torrent index, not a metadata source.
    Yts,
}

/// Which curated list to fetch.
///
/// Deliberately provider-agnostic: a variant names an intent ("top airing"),
/// not an AniList sort value. Each provider translates it into its own
/// vocabulary, so adding a source never changes this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListFilter {
    /// What is hot right now.
    Trending,
    /// Currently airing, most popular first.
    TopAiring,
    /// Most popular of all time.
    MostPopular,
    /// Highest rated.
    TopRated,
    /// Finished airing, most recently started first.
    LatestCompleted,
    /// Announced but not yet aired.
    Upcoming,
}

/// Where a page of results sits in the whole set.
///
/// Mirrors AniList's `Page.pageInfo`. The frontend needs `last_page` and
/// `has_next_page` to render pagination at all, and `total` to say how many
/// matches there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    /// Total matches. AniList caps this at 5000, so it is a floor rather than
    /// an exact count for very broad queries.
    pub total: u32,
    pub current_page: u32,
    pub last_page: u32,
    pub has_next_page: bool,
}

/// One page of results, with the paging metadata needed to ask for another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePage {
    pub items: Vec<Anime>,
    pub page_info: PageInfo,
}

/// How to order results.
///
/// Provider-agnostic, like [`ListFilter`]: a variant names an intent, and each
/// provider maps it to its own sort argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortOption {
    /// Most popular first.
    #[default]
    Popularity,
    /// Highest rated first.
    Score,
    /// Most recently started first.
    Newest,
    /// Alphabetical by the provider's own title form.
    TitleAz,
    /// What is hot right now, rather than what is popular overall.
    Trending,
    /// Most often favourited.
    Favorites,
    /// Most recently added to the provider's catalogue, which is not the same
    /// as most recently aired.
    DateAdded,
    /// Closest textual match. Only meaningful alongside a search term; without
    /// one the provider falls back to its default order.
    SearchMatch,
}

/// Which release status to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusFilter {
    Releasing,
    Finished,
    NotYetReleased,
}

/// Where a work sits on the reader's own AniList list.
///
/// Distinct from [`StatusFilter`] even though the names overlap: that one is
/// the WORK's release state ("is it still airing"), this is the READER's
/// relationship to it ("am I watching it"). A finished show can be `Current`
/// on someone's list, so conflating them would be wrong.
///
/// The variants serialise camelCase for the frontend, and [`Self::literal`]
/// maps them to the SCREAMING_SNAKE values AniList expects -- the same split
/// the other filter enums use, because the two audiences disagree on spelling
/// and one enum cannot satisfy both without a hand-rolled conversion anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListStatus {
    /// Watching it now.
    Current,
    /// Not started, intending to.
    Planning,
    /// Finished.
    Completed,
    /// Started and abandoned.
    Dropped,
    /// Started, set aside, possibly resumable.
    Paused,
}

impl ListStatus {
    /// The value AniList's GraphQL expects.
    pub fn literal(self) -> &'static str {
        match self {
            Self::Current => "CURRENT",
            Self::Planning => "PLANNING",
            Self::Completed => "COMPLETED",
            Self::Dropped => "DROPPED",
            Self::Paused => "PAUSED",
        }
    }

    /// Read a status AniList reported back.
    ///
    /// `None` for anything unrecognised rather than a default. AniList could
    /// add a status this app has never heard of, and reporting the reader's
    /// list position as "Planning" when it is something else would be a lie
    /// where showing nothing is merely incomplete.
    ///
    /// `REWATCHING` and `REPEATING` are AniList's own extra statuses for a work
    /// being watched again. This app has no separate tab for them, so they fold
    /// into `Current` rather than being dropped: a rewatch IS active watching,
    /// and silently losing those entries would be worse than showing them under
    /// "Watching".
    pub fn from_literal(raw: &str) -> Option<Self> {
        match raw {
            "CURRENT" | "REWATCHING" | "REPEATING" => Some(Self::Current),
            "PLANNING" => Some(Self::Planning),
            "COMPLETED" => Some(Self::Completed),
            "DROPPED" => Some(Self::Dropped),
            "PAUSED" => Some(Self::Paused),
            _ => None,
        }
    }
}

/// Which release format to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FormatFilter {
    Tv,
    Movie,
    Ova,
    Ona,
    Special,
    Music,
}

/// Which release season to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SeasonFilter {
    Winter,
    Spring,
    Summer,
    Fall,
}

/// A browse request.
///
/// Every filter is optional, so the frontend sends only what the user set and
/// the default is simply "most popular".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowseQuery {
    /// Free-text search. Blank is treated as absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    /// Genres to require.
    ///
    /// Combined with AND, not OR: a provider returns only works carrying
    /// every listed genre.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    /// Tags to require, by name. Also combined with AND.
    ///
    /// These are the provider's tag namespace, not its genres: AniList keeps
    /// "Isekai" and "School" as tags while "Action" and "Mecha" are genres.
    /// Sending a name the provider does not know is not an error -- it simply
    /// matches nothing -- so callers must only send names that came from
    /// [`MediaTag`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Tags to reject, by name. Combined with AND, like `tags`.
    ///
    /// The provider's rank floor governs this direction too -- verified
    /// against the live API, not assumed -- so excluding a tag only drops
    /// works carrying it at or above that rank. A work whose tagging is
    /// weaker survives the exclusion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<FormatFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<SeasonFilter>,
    /// Requires `season` to be meaningful on AniList, which scopes the year to
    /// a season rather than the whole calendar year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_year: Option<u32>,
    /// Minimum average score on the provider's own scale (AniList: 0-100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_score: Option<u8>,
    pub sort: SortOption,
    /// Reverse the sort's natural direction.
    ///
    /// A bool rather than `asc`/`desc`, because the natural direction differs
    /// per sort -- score is highest-first, title is A-Z -- so naming a
    /// direction would need a per-sort default. `false` (the default) is the
    /// natural order.
    #[serde(default)]
    pub reversed: bool,
}

/// A work's title in the several forms providers offer.
///
/// Kept as a struct rather than a single string because no single field is
/// reliably populated, and different users prefer different forms.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub romaji: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub english: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native: Option<String>,
    /// The viewer's own preference, when one has been chosen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_preferred: Option<String>,
}

impl Title {
    /// Best available title, following the usual preference order.
    ///
    /// Returns `None` only when every field is absent or blank, which lets
    /// callers decide how to render a genuinely untitled entry instead of
    /// showing an empty string.
    pub fn display(&self) -> Option<&str> {
        [
            self.user_preferred.as_deref(),
            self.english.as_deref(),
            self.romaji.as_deref(),
            self.native.as_deref(),
        ]
        .into_iter()
        .flatten()
        .find(|candidate| !candidate.trim().is_empty())
    }
}

/// A link to an official streaming source for a work.
///
/// AniList calls these "streaming episodes" and sources them from the
/// licensed services themselves. We surface them as-is: we do not host or
/// resolve any video ourselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingEpisode {
    /// Title of the entry on the external site.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Where to watch it. Required: a link with nowhere to go is useless.
    pub url: String,
    /// The service's name, e.g. "Crunchyroll".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

/// A descriptor a provider associates with a work.
///
/// AniList keeps these separate from genres: a genre is a broad category
/// (Action, Romance) while a tag is a specific attribute (Isekai, Time Loop).
/// Each tag carries the category that groups it in a filter UI, so it travels
/// as a pair rather than a bare string -- the frontend groups by category and
/// would otherwise have nothing to group on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTag {
    pub name: String,
    /// Grouping label, e.g. "Setting-Scene" or "Demographic".
    pub category: String,
    /// The provider's own prose for what the tag means, shown on hover.
    ///
    /// Optional because the provider may have no description for a tag;
    /// a tooltip-less chip is fine, an empty tooltip is not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A work (anime now; movies and series later) as presented in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anime {
    /// Id as issued by `provider`.
    pub id: i64,
    pub provider: ProviderId,
    pub title: Title,
    /// The work's id on MyAnimeList, when the provider knows it.
    ///
    /// AniList carries this as `idMal`. It is the bridge to episode-level data:
    /// AniList has no full episode list for long runners, but MAL does (via
    /// Jikan), and this id is what addresses it. `None` means no MAL entry is
    /// linked, so episode enrichment is skipped rather than guessed at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_mal: Option<i64>,
    /// Cover image URL, already sized by the provider layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    /// Wide banner image, for the hero area. Not every provider has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Total episodes, when known. Absent for films and ongoing shows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_count: Option<u32>,
    /// Episode length in minutes, when the provider reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_minutes: Option<u32>,
    /// Release format in the provider's wording, e.g. "TV", "MOVIE".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    /// Mean score on the provider's own scale (AniList: 0-100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_score: Option<u8>,
    /// How many users have the work on a list. A rough popularity signal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popularity: Option<u32>,
    /// Airing status in the provider's own wording, e.g. "FINISHED".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_year: Option<u32>,
    /// Release season in the provider's own wording, e.g. "FALL".
    ///
    /// Only meaningful alongside `season_year`, and providers may report one
    /// without the other, so neither is required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<String>,
    /// Official places to watch this legally, as reported by the provider.
    #[serde(default)]
    pub streaming_episodes: Vec<StreamingEpisode>,
    /// Other works this one is connected to, as the provider reports.
    ///
    /// Populated only by the single-title lookup; list queries leave it empty,
    /// since a card has no room for it and it would bloat every response.
    #[serde(default)]
    pub relations: Vec<RelatedAnime>,
    /// Community recommendations, highest-rated first. Detail-only.
    #[serde(default)]
    pub recommendations: Vec<RecommendedAnime>,
    /// The work's trailer, when the provider has one. Detail-only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailer: Option<Trailer>,
}

/// A work connected to another, as the provider links them.
///
/// `relation_type` is the provider's own vocabulary ("SEQUEL", "PREQUEL",
/// "SIDE_STORY", ...) and is surfaced verbatim: the set is open-ended, and
/// translating it into an enum of our own would drop any type a future
/// provider adds.
///
/// Deliberately not a whole [`Anime`]: a sidebar row needs a cover, a title and
/// a couple of facts, and the full type would drag genres, description and
/// streaming links through every detail response for nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedAnime {
    pub id: i64,
    pub title: Title,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_count: Option<u32>,
    /// How this work relates to the one being viewed, e.g. "SEQUEL".
    pub relation_type: String,
}

/// A reader who suggested a recommendation.
///
/// Carried so the UI can credit the suggester. AniList has no free-text "why"
/// field on a recommendation -- the reason is only the vote tally and who first
/// posted it -- so this is the most context the provider offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommenderUser {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// How the signed-in reader voted on a recommendation.
///
/// Distinct from the `rating` tally: this is the reader's OWN vote, which the
/// UI highlights. AniList reports `NO_RATING` (or null) for a reader who has not
/// voted, and null for a signed-out one; the two collapse to `None` here because
/// neither should highlight anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecommendationRating {
    /// Not voted (AniList's `NO_RATING`).
    NoRating,
    /// Voted up.
    RateUp,
    /// Voted down.
    RateDown,
}

impl RecommendationRating {
    /// The value AniList's GraphQL expects.
    pub fn literal(self) -> &'static str {
        match self {
            Self::NoRating => "NO_RATING",
            Self::RateUp => "RATE_UP",
            Self::RateDown => "RATE_DOWN",
        }
    }

    /// Read a rating AniList reported back.
    ///
    /// `None` for anything unrecognised: AniList could add a variant, and
    /// lighting up the wrong arrow would be worse than lighting up none.
    pub fn from_literal(value: &str) -> Option<Self> {
        match value {
            "NO_RATING" => Some(Self::NoRating),
            "RATE_UP" => Some(Self::RateUp),
            "RATE_DOWN" => Some(Self::RateDown),
            _ => None,
        }
    }
}

/// A community recommendation for a work.
///
/// Carries the recommended work whole so a poster card renders without a second
/// lookup. `rating` is the number of users who upvoted the suggestion, kept
/// beside the work so the UI can show or sort by it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedAnime {
    pub anime: Anime,
    /// Community vote tally: positive when upvoted, negative when downvoted.
    ///
    /// Signed because AniList returns -1 for a net-downvoted suggestion, not 0.
    pub rating: i32,
    /// Who first posted the recommendation, when the provider reports it.
    ///
    /// Optional: an anonymous or deleted account leaves the credit off rather
    /// than failing the decode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<RecommenderUser>,
    /// The signed-in reader's own vote, when the provider reports one.
    ///
    /// Absent for a signed-out reader, and for a signed-in one who has not
    /// voted, so the UI has nothing to highlight in either case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_rating: Option<RecommendationRating>,
}

/// One page of recommendations, with the paging metadata needed to ask for
/// another.
///
/// Mirrors [`AnimePage`]: the full-recommendations view scrolls, so it needs the
/// same `has_next_page`/`last_page` signals to know when to stop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationsPage {
    pub items: Vec<RecommendedAnime>,
    pub page_info: PageInfo,
}

/// A promotional video for a work.
///
/// `site` and `id` stay apart rather than joined into one URL because the
/// provider reports the platform and the video id separately, and each platform
/// spells its watch URL differently -- composing the link is the UI's job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trailer {
    /// The video id on `site`, e.g. a YouTube video id.
    pub id: String,
    /// Hosting platform, e.g. "youtube" or "dailymotion".
    pub site: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

/// One upcoming (or recent) broadcast, as listed on a schedule.
///
/// Distinct from [`Episode`]: an `Episode` is something this app could play,
/// whereas a `ScheduledEpisode` is a provider's announcement that a work will
/// air at a given time. No source is resolved here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledEpisode {
    /// The work being aired. Carried whole so the UI can render a card without
    /// a second lookup per entry.
    pub anime: Anime,
    /// When it airs, as a unix timestamp in seconds.
    ///
    /// Kept as an absolute instant rather than a pre-formatted string: the
    /// provider has no idea what timezone the viewer is in, so converting to
    /// local time is the frontend's job.
    pub airing_at: i64,
    /// Episode number being aired, when the provider states one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode: Option<u32>,
}
/// Metadata for a single episode, as a provider reports it.
///
/// Distinct from [`Episode`], which is a playable entry tied to a resolved
/// release. This is catalogue data: what the episode is called, when it aired,
/// and whether it is filler or a recap -- the things AniList omits for long
/// runners and Jikan supplies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeInfo {
    /// 1-based episode number, as the provider numbers it.
    pub number: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// When it first aired, as the provider's own date string.
    ///
    /// Kept verbatim rather than parsed into a timestamp: Jikan sometimes omits
    /// the time and occasionally the day, so parsing would invent precision that
    /// is not there. The frontend formats it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aired: Option<String>,
    /// Anime-original filler, which a viewer may want to skip.
    pub filler: bool,
    /// A recap of earlier events.
    pub recap: bool,
}
/// One playable entry within an [`Anime`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    /// Episode number as the release names it (may be fractional or special).
    pub number: u32,
    /// Id of the [`Anime`] this episode belongs to.
    pub anime_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The release chosen for this episode, once one has been resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnet: Option<MagnetLink>,
}

/// A resolved magnet link, ready to hand to the torrent engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MagnetLink {
    /// The full `magnet:?xt=urn:btih:...` URI.
    pub uri: String,
    /// Lowercase hex info hash, when it could be extracted from `uri`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_hash: Option<String>,
    /// Release name, used for display and file selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeders: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leechers: Option<u32>,
}

/// Vertical resolution a release is encoded at.
///
/// Carries the numbering used by release names (`1080p`) rather than an
/// index, so the UI can render it directly and comparisons are ordering-free.
/// `Unknown` is the absence of a claim, not a low quality: a release that
/// never states a resolution must not be treated as 360p.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Resolution {
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "360p")]
    R360p,
    #[serde(rename = "480p")]
    R480p,
    #[serde(rename = "540p")]
    R540p,
    #[serde(rename = "576p")]
    R576p,
    #[serde(rename = "720p")]
    R720p,
    #[serde(rename = "1080p")]
    R1080p,
    #[serde(rename = "2160p")]
    R2160p,
}

impl Resolution {
    /// How many vertical lines, or `None` when the release did not say.
    pub fn lines(self) -> Option<u32> {
        match self {
            Self::Unknown => None,
            Self::R360p => Some(360),
            Self::R480p => Some(480),
            Self::R540p => Some(540),
            Self::R576p => Some(576),
            Self::R720p => Some(720),
            Self::R1080p => Some(1080),
            Self::R2160p => Some(2160),
        }
    }
}

/// Where a release's video came from.
///
/// Mirrors Sonarr's source vocabulary, ordered roughly worst-to-best for
/// anime: a broadcast capture (`Sdtv`) is a worse source than a disc rip
/// (`BluRay`). The ordering is used by the ranker, which is why the two DVD
/// variants sit together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ReleaseSource {
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "sdtv")]
    Sdtv,
    #[serde(rename = "tvrip")]
    TvRip,
    #[serde(rename = "dsr")]
    Dsr,
    #[serde(rename = "pdtv")]
    Pdtv,
    #[serde(rename = "dvd")]
    Dvd,
    #[serde(rename = "hdtv")]
    Hdtv,
    #[serde(rename = "webrip")]
    WebRip,
    #[serde(rename = "webdl")]
    WebDl,
    #[serde(rename = "bdrip")]
    BdRip,
    #[serde(rename = "brrip")]
    BrRip,
    #[serde(rename = "bluray")]
    BluRay,
    #[serde(rename = "rawhd")]
    RawHd,
}

impl ReleaseSource {
    /// A rough quality ordering, higher being a better source.
    ///
    /// `BluRay` and the two disc rips outrank streaming, which outranks
    /// broadcast. `Unknown` sits at the bottom but is deliberately not
    /// negative, so it never inverts a comparison.
    pub fn rank(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Sdtv => 1,
            Self::TvRip => 2,
            Self::Dsr => 2,
            Self::Pdtv => 3,
            Self::Dvd => 4,
            Self::Hdtv => 5,
            Self::WebRip => 6,
            Self::WebDl => 7,
            Self::BdRip => 8,
            Self::BrRip => 8,
            Self::BluRay => 9,
            Self::RawHd => 5,
        }
    }
}

/// What the release-name parser learned about a name.
///
/// A release is described two ways at once because the naming is ambiguous:
/// an anime release names an *absolute* episode (`Show - 37`) while a
/// western-style release names season and episode (`Show S04E01`). Both are
/// captured when present, and the matcher accepts either.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRelease {
    /// The work's name, as the release spells it. `None` when the parser
    /// could not isolate one, which is not the same as an empty title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode: Option<u32>,
    /// Episode number counting from the first episode of the work, which is
    /// how anime indexers number releases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub absolute_episode: Option<u32>,
    /// Fansub or release group, e.g. "SubsPlease".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subgroup: Option<String>,
}

/// A candidate release found by an indexer, ready to hand to the UI.
///
/// Flattens what the indexer returned (the magnet) with what the parsers
/// derived (quality, episode numbers) and the ranker's verdict (`score`), so
/// the frontend can render the list without re-deriving anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    /// The release name as the indexer spelled it. This is what the torrent
    /// engine and file selector will see, so it is never rewritten.
    pub title: String,
    pub indexer: ProviderId,
    /// Full magnet URI, validated by the indexer layer.
    pub magnet_uri: String,
    /// Direct `.torrent` download URL, when the indexer exposed one.
    ///
    /// Kept beside the magnet rather than derived from it: Nyaa's feed carries
    /// a download link that is not the info hash, and the UI offers "save the
    /// torrent file" as an alternative to moving the magnet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torrent_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeders: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leechers: Option<u32>,
    pub resolution: Resolution,
    pub source: ReleaseSource,
    /// True when the release is a remux of another, which is a better copy.
    pub remux: bool,
    /// Whether the indexer marks the uploader as trusted.
    pub trusted: bool,
    /// What the name parser extracted. Carried so the UI can show the
    /// episode a release claims and the matcher can filter on it.
    pub parsed: ParsedRelease,
    /// The ranker's verdict: higher is a better pick for the request. Zero
    /// means the release was not scored (e.g. returned unfiltered).
    pub score: i64,
}

/// What the viewer prefers when several releases are equally correct.
///
/// Deliberately small: the ranker already knows the correct episode and a
/// sane source ordering, so the only genuine taste left is resolution and how
/// much seeding matters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReleasePreference {
    /// Resolutions to prefer, in order. The first match wins, so the list is
    /// a ranking rather than a set.
    pub preferred_resolutions: Vec<Resolution>,
    /// Releases below this many seeders are dropped when anything else is
    /// available. Zero keeps everything.
    pub min_seeders: u32,
}

impl Default for ReleasePreference {
    /// 1080p, then 720p, then whatever else — and no seeder floor, since a
    /// rare release with one seeder is still watchable.
    fn default() -> Self {
        Self {
            preferred_resolutions: vec![Resolution::R1080p, Resolution::R720p],
            min_seeders: 0,
        }
    }
}

/// A `Release` with its rank score, used while sorting.
///
/// Kept separate from [`Release`] so the scoring pass is a pure function over
/// borrowed data and the ordering rule can be tested without constructing
/// whole releases twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoredRelease {
    /// Index into the slice that was scored.
    pub index: usize,
    pub score: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_anime() -> Anime {
        Anime {
            id: 21,
            provider: ProviderId::AniList,
                id_mal: Some(21),
            title: Title {
                romaji: Some("One Piece".into()),
                english: Some("One Piece".into()),
                native: Some("ワンピース".into()),
                user_preferred: None,
            },
            cover_image: Some("https://example.test/cover.jpg".into()),
            banner_image: Some("https://example.test/banner.jpg".into()),
            description: Some("A pirate adventure.".into()),
            episode_count: Some(1100),
            duration_minutes: Some(24),
            format: Some("TV".into()),
            genres: vec!["Action".into(), "Adventure".into()],
            average_score: Some(88),
            popularity: Some(250_000),
            status: Some("RELEASING".into()),
            season: Some("FALL".into()),
            season_year: Some(1999),
            streaming_episodes: vec![StreamingEpisode {
                title: Some("Episode 1".into()),
                url: "https://crunchyroll.test/one-piece/1".into(),
                site: Some("Crunchyroll".into()),
                thumbnail: None,
            }],
            relations: vec![RelatedAnime {
                id: 22,
                title: Title {
                    romaji: Some("One Piece Film: Red".into()),
                    english: None,
                    native: None,
                    user_preferred: None,
                },
                cover_image: Some("https://example.test/related.jpg".into()),
                format: Some("MOVIE".into()),
                status: Some("FINISHED".into()),
                episode_count: Some(1),
                relation_type: "SEQUEL".into(),
            }],
            recommendations: vec![],
            trailer: Some(Trailer {
                id: "dQw4w9WgXcQ".into(),
                site: "youtube".into(),
                thumbnail: Some("https://example.test/trailer.jpg".into()),
            }),
        }
    }

    #[test]
    fn anime_survives_a_json_round_trip() {
        let original = sample_anime();
        let json = serde_json::to_string(&original).expect("serialize Anime");
        let restored: Anime = serde_json::from_str(&json).expect("deserialize Anime");
        assert_eq!(original, restored);
    }

    #[test]
    fn anime_serialises_to_camel_case_keys() {
        let json = serde_json::to_value(sample_anime()).expect("serialize Anime");
        assert!(json.get("coverImage").is_some(), "expected camelCase key");
        assert!(json.get("episodeCount").is_some());
        assert!(json.get("averageScore").is_some());
        // The detail-only additions travel camelCase too.
        assert!(json.get("relations").is_some());
        assert!(json.get("trailer").is_some());
        // And never the snake_case form.
        assert!(json.get("cover_image").is_none());
    }

    #[test]
    fn anime_always_serialises_its_array_fields() {
        // Regression: these four were once `skip_serializing_if = "Vec::is_empty"`,
        // so an empty list omitted the key entirely. The frontend types declare
        // them as required arrays, so it received `undefined` and threw while
        // rendering the detail page -- which, with no error boundary, left the
        // skeleton on screen forever. An empty list must serialise as `[]`.
        let mut sparse = sample_anime();
        sparse.genres = vec![];
        sparse.streaming_episodes = vec![];
        sparse.relations = vec![];
        sparse.recommendations = vec![];

        let json = serde_json::to_value(sparse).expect("serialize Anime");

        for key in [
            "genres",
            "streamingEpisodes",
            "relations",
            "recommendations",
        ] {
            let value = json
                .get(key)
                .unwrap_or_else(|| panic!("expected `{key}` to be present even when empty"));
            assert_eq!(
                value.as_array().map(Vec::len),
                Some(0),
                "expected `{key}` to serialise as an empty array",
            );
        }
    }

    #[test]
    fn provider_serialises_lowercase() {
        let json = serde_json::to_string(&ProviderId::AniList).expect("serialize provider");
        assert_eq!(json, "\"anilist\"");
    }

    #[test]
    fn anime_deserialises_with_optional_fields_missing() {
        // The minimum a provider must supply. Everything else is optional, so
        // a sparse upstream response does not fail the whole fetch.
        let json = r#"{
            "id": 21,
            "provider": "anilist",
            "title": { "romaji": "One Piece" }
        }"#;

        let anime: Anime = serde_json::from_str(json).expect("deserialize sparse Anime");
        assert_eq!(anime.id, 21);
        assert_eq!(anime.provider, ProviderId::AniList);
        assert!(anime.cover_image.is_none());
        assert!(anime.genres.is_empty());
    }

    #[test]
    fn episode_survives_a_json_round_trip() {
        let original = Episode {
            number: 1,
            anime_id: 21,
            title: Some("I'm Luffy!".into()),
            magnet: Some(sample_magnet()),
        };
        let json = serde_json::to_string(&original).expect("serialize Episode");
        let restored: Episode = serde_json::from_str(&json).expect("deserialize Episode");
        assert_eq!(original, restored);
    }

    fn sample_magnet() -> MagnetLink {
        MagnetLink {
            uri: "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862".into(),
            info_hash: Some("cab507494d02ebb1178b38f2e9d7be299c86b862".into()),
            title: Some("[SubsPlease] One Piece - 01 (1080p)".into()),
            size_bytes: Some(1_400_000_000),
            seeders: Some(42),
            leechers: Some(3),
        }
    }

    #[test]
    fn magnet_survives_a_json_round_trip() {
        let original = sample_magnet();
        let json = serde_json::to_string(&original).expect("serialize MagnetLink");
        let restored: MagnetLink = serde_json::from_str(&json).expect("deserialize MagnetLink");
        assert_eq!(original, restored);
    }

    #[test]
    fn magnet_serialises_size_and_hash_in_camel_case() {
        let json = serde_json::to_value(sample_magnet()).expect("serialize MagnetLink");
        assert!(json.get("sizeBytes").is_some());
        assert!(json.get("infoHash").is_some());
        assert!(json.get("size_bytes").is_none());
    }

    // --- Title::display preference order ---------------------------------

    #[test]
    fn title_prefers_user_preferred_over_everything() {
        let title = Title {
            user_preferred: Some("My Title".into()),
            english: Some("English".into()),
            romaji: Some("Romaji".into()),
            native: Some("Native".into()),
        };
        assert_eq!(title.display(), Some("My Title"));
    }

    #[test]
    fn title_falls_back_english_then_romaji_then_native() {
        let english_only = Title {
            english: Some("English".into()),
            romaji: Some("Romaji".into()),
            ..Default::default()
        };
        assert_eq!(english_only.display(), Some("English"));

        let romaji_only = Title {
            romaji: Some("Romaji".into()),
            native: Some("Native".into()),
            ..Default::default()
        };
        assert_eq!(romaji_only.display(), Some("Romaji"));

        let native_only = Title {
            native: Some("Native".into()),
            ..Default::default()
        };
        assert_eq!(native_only.display(), Some("Native"));
    }

    #[test]
    fn title_skips_blank_candidates() {
        // Providers sometimes return "" for a field that is not really set.
        // An empty string is not a usable title.
        let title = Title {
            english: Some("   ".into()),
            romaji: Some("Real Title".into()),
            ..Default::default()
        };
        assert_eq!(title.display(), Some("Real Title"));
    }

    #[test]
    fn title_returns_none_when_everything_is_missing() {
        assert_eq!(Title::default().display(), None);
    }
}
