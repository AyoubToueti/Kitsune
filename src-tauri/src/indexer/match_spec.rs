//! Decide whether a release is the episode that was asked for.
//!
//! Ported from Sonarr's `SingleEpisodeSearchMatchSpecification` and its anime
//! branch. The rule for anime is deliberately looser than for western TV: a
//! release is kept when *either* its absolute episode or its season/episode
//! pair matches, and a mismatched episode number alone does not disqualify a
//! release whose title and season line up -- because anime numbering is
//! inconsistent across groups.
//!
//! The one thing always rejected is a whole pack. A batch is not one episode.
//!
//! Pure: no network, no clock, no panics.

use crate::types::{ParsedRelease, Release};

/// What the caller is looking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpisodeRequest {
    /// The episode number to match, counting from the first episode of the
    /// work. This is the number AniList's streaming-episode titles yield.
    pub episode: u32,
    /// Absolute episode number, when the work is numbered that way. Usually
    /// the same as `episode` for anime.
    pub absolute_episode: Option<u32>,
    /// Season number, when the caller knows which one. `None` accepts any.
    pub season: Option<u32>,
}

impl EpisodeRequest {
    /// A request for a single anime episode.
    ///
    /// The absolute number defaults to the episode number, which is the
    /// common anime case.
    pub fn anime(episode: u32) -> Self {
        Self {
            episode,
            absolute_episode: Some(episode),
            season: None,
        }
    }
}

/// Why a release was rejected.
///
/// Carried rather than dropped so the UI can explain an empty result and a
/// test can assert *which* rule fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchRejection {
    /// The release is a multi-episode or whole-season pack.
    Pack,
    /// The release names a different season.
    WrongSeason,
    /// The release names a different episode.
    WrongEpisode,
    /// The release states no episode number at all.
    NoEpisode,
}

/// Whether a release satisfies a request, and why not when it does not.
pub fn matches(release: &Release, request: &EpisodeRequest) -> Result<(), MatchRejection> {
    matches_parsed(&release.title, &release.parsed, request)
}

/// The decision, over the parsed fields alone.
///
/// Split from [`matches`] so the rule can be tested without building a whole
/// [`Release`], and so a caller that already has a [`ParsedRelease`] (e.g. the
/// file selector) can reuse it.
pub fn matches_parsed(
    title: &str,
    parsed: &ParsedRelease,
    request: &EpisodeRequest,
) -> Result<(), MatchRejection> {
    if super::parse::is_pack(title) {
        return Err(MatchRejection::Pack);
    }

    // A stated season must agree, when the caller named one. Anime releases
    // rarely state a season, so an absent one is not a rejection.
    if let (Some(wanted), Some(actual)) = (request.season, parsed.season) {
        if wanted != actual {
            return Err(MatchRejection::WrongSeason);
        }
    }

    let absolute_hit = request
        .absolute_episode
        .zip(parsed.absolute_episode)
        .is_some_and(|(wanted, actual)| wanted == actual);
    let relative_hit = parsed
        .episode
        .is_some_and(|actual| actual == request.episode);

    if absolute_hit || relative_hit {
        return Ok(());
    }

    // Nothing stated an episode. That is a miss, not a pass: a release with no
    // number is usually a movie or a mistagged upload.
    if parsed.absolute_episode.is_none() && parsed.episode.is_none() {
        return Err(MatchRejection::NoEpisode);
    }

    Err(MatchRejection::WrongEpisode)
}

/// Keep only the releases that match, preserving their order.
pub fn filter<'a>(releases: &'a [Release], request: &EpisodeRequest) -> Vec<&'a Release> {
    releases
        .iter()
        .filter(|release| matches(release, request).is_ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::parse::parse_release;
    use crate::types::{ProviderId, ReleaseSource, Resolution};

    fn release_named(title: &str) -> Release {
        Release {
            title: title.to_string(),
            indexer: ProviderId::Nyaa,
            magnet_uri: "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862".into(),
            torrent_url: None,
            info_hash: None,
            size_bytes: None,
            seeders: None,
            leechers: None,
            resolution: Resolution::Unknown,
            source: ReleaseSource::Unknown,
            remux: false,
            trusted: false,
            parsed: parse_release(title),
            score: 0,
        }
    }

    #[test]
    fn an_absolute_match_passes() {
        let release = release_named("[G] Show - 05 [1080p]");
        assert_eq!(matches(&release, &EpisodeRequest::anime(5)), Ok(()));
    }

    #[test]
    fn a_relative_match_passes() {
        let release = release_named("[G] Show S04E07 [1080p]");
        let request = EpisodeRequest {
            episode: 7,
            absolute_episode: None,
            season: Some(4),
        };
        assert_eq!(matches(&release, &request), Ok(()));
    }

    #[test]
    fn a_pack_is_rejected_even_when_it_spans_the_episode() {
        // "01-12" contains episode 5, but it is not the single episode asked.
        let release = release_named("[G] Show 01-12 [1080p]");
        assert_eq!(
            matches(&release, &EpisodeRequest::anime(5)),
            Err(MatchRejection::Pack)
        );
    }

    #[test]
    fn a_batch_word_is_rejected() {
        let release = release_named("[G] Show Batch [1080p]");
        assert_eq!(
            matches(&release, &EpisodeRequest::anime(5)),
            Err(MatchRejection::Pack)
        );
    }

    #[test]
    fn a_season_marker_with_the_wanted_episode_is_kept() {
        // `Season 3 - 09` names episode 9 of season 3, not a season pack. The
        // matcher must not reject it on the `Season N` marker alone.
        let release = release_named("[G] Show Season 3 - 09 [1080p]");
        assert_eq!(matches(&release, &EpisodeRequest::anime(9)), Ok(()));
    }

    #[test]
    fn a_bare_season_marker_is_still_a_pack() {
        let release = release_named("[G] Show Season 3 [1080p]");
        assert_eq!(
            matches(&release, &EpisodeRequest::anime(9)),
            Err(MatchRejection::Pack)
        );
    }

    #[test]
    fn a_wrong_episode_is_rejected() {
        let release = release_named("[G] Show - 09 [1080p]");
        assert_eq!(
            matches(&release, &EpisodeRequest::anime(5)),
            Err(MatchRejection::WrongEpisode)
        );
    }

    #[test]
    fn a_wrong_stated_season_is_rejected() {
        let release = release_named("[G] Show S02E05 [1080p]");
        let request = EpisodeRequest {
            episode: 5,
            absolute_episode: Some(5),
            season: Some(4),
        };
        assert_eq!(matches(&release, &request), Err(MatchRejection::WrongSeason));
    }

    #[test]
    fn a_relative_only_release_matches_by_episode() {
        // The release states S04E05 but no absolute; the request's episode
        // number still matches, which is the looser anime rule.
        let release = release_named("[G] Show S04E05 [1080p]");
        assert_eq!(matches(&release, &EpisodeRequest::anime(5)), Ok(()));
    }

    #[test]
    fn a_season_episode_tail_absolute_matches() {
        // "S04E01 - 60" and the caller wants absolute 60.
        let release = release_named("[G] Show S04E01 - 60 [1080p]");
        let request = EpisodeRequest {
            episode: 60,
            absolute_episode: Some(60),
            season: None,
        };
        assert_eq!(matches(&release, &request), Ok(()));
    }

    #[test]
    fn a_release_with_no_episode_is_rejected() {
        let release = release_named("[G] Show Movie [1080p]");
        assert_eq!(
            matches(&release, &EpisodeRequest::anime(5)),
            Err(MatchRejection::NoEpisode)
        );
    }

    #[test]
    fn filter_keeps_order_and_drops_non_matches() {
        let releases = vec![
            release_named("[G] Show - 05 [1080p]"),
            release_named("[G] Show - 09 [1080p]"),
            release_named("[G] Show Batch [1080p]"),
            release_named("[G] Show - 05 [720p]"),
        ];

        let kept = filter(&releases, &EpisodeRequest::anime(5));
        let titles: Vec<&str> = kept.iter().map(|r| r.title.as_str()).collect();

        assert_eq!(
            titles,
            vec!["[G] Show - 05 [1080p]", "[G] Show - 05 [720p]"]
        );
    }

    #[test]
    fn an_absent_season_on_the_release_is_allowed() {
        let release = release_named("[G] Show - 05 [1080p]");
        let request = EpisodeRequest {
            episode: 5,
            absolute_episode: Some(5),
            season: Some(4),
        };
        // The release never states a season, so it is not "wrong".
        assert_eq!(matches(&release, &request), Ok(()));
    }
}