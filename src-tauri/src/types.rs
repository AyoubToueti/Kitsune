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

/// A work (anime now; movies and series later) as presented in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anime {
    /// Id as issued by `provider`.
    pub id: i64,
    pub provider: ProviderId,
    pub title: Title,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    /// Official places to watch this legally, as reported by the provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub streaming_episodes: Vec<StreamingEpisode>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_anime() -> Anime {
        Anime {
            id: 21,
            provider: ProviderId::AniList,
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
            season_year: Some(1999),
            streaming_episodes: vec![StreamingEpisode {
                title: Some("Episode 1".into()),
                url: "https://crunchyroll.test/one-piece/1".into(),
                site: Some("Crunchyroll".into()),
                thumbnail: None,
            }],
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
        // And never the snake_case form.
        assert!(json.get("cover_image").is_none());
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
