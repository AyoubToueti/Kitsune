//! Torrent indexers and the pipeline that turns a search into a ranked list.
//!
//! The work splits cleanly in two:
//!
//! * Pure parsing and ranking ([`normalize`], [`parse`], [`quality`],
//!   [`match_spec`], [`rank`]) -- ported from Sonarr, fully unit tested.
//! * A thin transport per indexer ([`nyaa`]) -- fetching a feed and mapping it
//!   into [`crate::types::Release`].
//!
//! [`search`] is the seam between them: it fetches, drops the releases that are
//! not the requested episode, and orders what is left. It is a free function
//! over `&dyn Indexer` so it can be tested against a stub without any network.

pub mod commands;
pub mod match_spec;
pub mod normalize;
pub mod nyaa;
pub mod parse;
pub mod probe;
pub mod quality;
pub mod rank;
pub mod traits;

pub use commands::{search_all, search_releases, IndexerRegistry};
pub use match_spec::{EpisodeRequest, MatchRejection};
pub use nyaa::{NyaaIndexer, NYAA_ENDPOINT};
pub use traits::{Indexer, IndexerError};

use crate::types::{Release, ReleasePreference};

/// What to search for, assembled from an [`crate::types::Anime`] by the UI.
///
/// The title and episode travel together because an anime search is always
/// "this work, this episode"; splitting them would let a caller search for a
/// title with no episode and silently get a season pack back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseRequest {
    /// The work's display title, as the indexers should search it.
    pub title: String,
    /// The episode to find, or `None` for a film or a title-level search.
    pub episode: Option<u32>,
}

impl ReleaseRequest {
    /// The query string sent to an indexer.
    ///
    /// Kept to the title and, when present, the episode: Nyaa's search is a
    /// substring match over the release name, and adding an episode narrows it
    /// without excluding the ` - 05` spelling.
    pub fn query(&self) -> String {
        match self.episode {
            Some(episode) => format!("{} {:02}", self.title.trim(), episode),
            None => self.title.trim().to_string(),
        }
    }

    /// The matcher's view of this request.
    pub fn as_match(&self) -> Option<EpisodeRequest> {
        self.episode.map(EpisodeRequest::anime)
    }
}

/// Run a search end to end: fetch, filter to the episode, rank best-first.
///
/// Filtering happens before ranking so the ranker only ever orders releases
/// that are actually the right episode. When the request names no episode the
/// filter is skipped, which is what makes a film or a batch search possible.
pub async fn search(
    indexer: &dyn Indexer,
    request: &ReleaseRequest,
    preference: &ReleasePreference,
) -> Result<Vec<Release>, IndexerError> {
    let found = indexer.search(&request.query()).await?;
    Ok(refine(found, request, preference))
}

/// Filter and rank an already-fetched set of releases.
///
/// Split from [`search`] so the pipeline can be tested on fixtures with no
/// indexer at all.
pub fn refine(
    releases: Vec<Release>,
    request: &ReleaseRequest,
    preference: &ReleasePreference,
) -> Vec<Release> {
    let mut kept = match request.as_match() {
        Some(matcher) => releases
            .into_iter()
            .filter(|release| match_spec::matches(release, &matcher).is_ok())
            .collect(),
        None => releases,
    };

    // A search that filtered everything out is a legitimate empty result; the
    // UI shows "no releases found" rather than falling back to the unfiltered
    // list, which would hand the viewer the wrong episode.
    rank::rank(&mut kept, preference);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProviderId, ReleaseSource, Resolution};

    fn release_named(title: &str) -> Release {
        Release {
            title: title.to_string(),
            indexer: ProviderId::Nyaa,
            magnet_uri: "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862"
                .into(),
            torrent_url: None,
            info_hash: None,
            size_bytes: None,
            seeders: Some(10),
            leechers: None,
            resolution: Resolution::R1080p,
            source: ReleaseSource::WebDl,
            remux: false,
            trusted: false,
            parsed: parse::parse_release(title),
            score: 0,
        }
    }

    #[test]
    fn query_includes_a_padded_episode() {
        let request = ReleaseRequest {
            title: "Show".into(),
            episode: Some(5),
        };
        assert_eq!(request.query(), "Show 05");
    }

    #[test]
    fn query_without_an_episode_is_just_the_title() {
        let request = ReleaseRequest {
            title: "  Show  ".into(),
            episode: None,
        };
        assert_eq!(request.query(), "Show");
    }

    #[test]
    fn refine_drops_other_episodes_and_ranks_the_rest() {
        let request = ReleaseRequest {
            title: "Show".into(),
            episode: Some(5),
        };
        let preference = ReleasePreference::default();

        let releases = vec![
            release_named("[G] Show - 09 [1080p]"),
            release_named("[G] Show - 05 [480p]"),
            release_named("[G] Show - 05 [1080p]"),
        ];

        let refined = refine(releases, &request, &preference);

        assert_eq!(refined.len(), 2, "only episode 5 survives");
        assert!(
            refined[0].title.contains("1080p"),
            "the preferred resolution should be first"
        );
    }

    #[test]
    fn refine_without_an_episode_keeps_everything() {
        let request = ReleaseRequest {
            title: "Show".into(),
            episode: None,
        };
        let preference = ReleasePreference::default();

        let releases = vec![
            release_named("[G] Show - 05 [1080p]"),
            release_named("[G] Show Batch [1080p]"),
        ];

        let refined = refine(releases, &request, &preference);
        assert_eq!(refined.len(), 2);
    }

    #[test]
    fn refine_of_an_empty_search_is_empty() {
        let request = ReleaseRequest {
            title: "Show".into(),
            episode: Some(1),
        };
        let preference = ReleasePreference::default();

        assert!(refine(Vec::new(), &request, &preference).is_empty());
    }

    /// A stub indexer proves `search` wires the pipeline without a network.
    struct StubIndexer {
        releases: Vec<Release>,
    }

    #[async_trait::async_trait]
    impl Indexer for StubIndexer {
        fn name(&self) -> &str {
            "stub"
        }

        async fn search(&self, _query: &str) -> Result<Vec<Release>, IndexerError> {
            Ok(self.releases.clone())
        }
    }

    #[tokio::test]
    async fn search_fetches_filters_and_ranks() {
        let indexer = StubIndexer {
            releases: vec![
                release_named("[G] Show - 05 [1080p]"),
                release_named("[G] Show - 05 [480p]"),
                release_named("[G] Show - 06 [1080p]"),
            ],
        };

        let request = ReleaseRequest {
            title: "Show".into(),
            episode: Some(5),
        };
        let releases = search(&indexer, &request, &ReleasePreference::default())
            .await
            .expect("stub should succeed");

        assert_eq!(releases.len(), 2);
        assert!(releases[0].title.contains("1080p"));
    }
}