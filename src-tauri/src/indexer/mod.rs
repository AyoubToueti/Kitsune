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
pub mod probe_rank;
pub mod quality;
pub mod query;
pub mod rank;
pub mod traits;

pub use commands::{search_all, search_releases, IndexerRegistry};
pub use match_spec::{EpisodeRequest, MatchRejection};
pub use nyaa::{NyaaIndexer, NYAA_ENDPOINT};
pub use query::SearchMode;
pub use traits::{Indexer, IndexerError};

use crate::types::{Release, ReleasePreference};

/// What to search for, assembled from an [`crate::types::Anime`] by the UI.
///
/// Titles and episode travel together because an anime search is always "this
/// work, this episode"; splitting them would let a caller search for a title
/// with no episode and silently get a season pack back.
///
/// There are several titles, not one, because a work has an English title and a
/// romaji one and either may be what an uploader used. They are searched in the
/// order given, best first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseRequest {
    /// The work's title forms, best first, as the indexers should search them.
    pub titles: Vec<String>,
    /// The episode to find, or `None` for a film or a title-level search.
    pub episode: Option<u32>,
    /// The work-wide episode number, when it differs from `episode`.
    ///
    /// A later cour's list restarts at 1 while its releases carry the running
    /// total, so both spellings are worth matching. `None` means the two agree
    /// and only `episode` is in play.
    pub absolute_episode: Option<u32>,
    /// Whether to find the one episode or the packs that contain it.
    pub mode: SearchMode,
}

impl ReleaseRequest {
    /// The query strings to send to an indexer, most promising first.
    ///
    /// A single query cannot find a release reliably: a colon in a title breaks
    /// Nyaa's term matching, releases spell an episode `S03E09` rather than
    /// `09`, and a release may name the work by either its English or its romaji
    /// title. The construction lives in [`query`] so it can be tested on its own.
    pub fn queries(&self) -> Vec<String> {
        query::build_queries_in_mode(
            &self.titles,
            self.episode,
            self.mode,
            query::MAX_QUERIES,
        )
    }

    /// The matcher's view of this request.
    ///
    /// The season comes from the title, defaulting to 1 once an episode is in
    /// play: a season-1 entry is titled without a marker, but its releases are
    /// still named `S01E01`. Without the default, a season-2 release carrying
    /// the same episode number would satisfy a season-1 search.
    ///
    /// A pack search returns `None`: filtering for packs is `is_pack`, not an
    /// episode match, and [`refine`] handles that case directly.
    pub fn as_match(&self) -> Option<EpisodeRequest> {
        if self.mode == SearchMode::Packs {
            return None;
        }

        // The first title is the one the UI prefers, so it is the one whose
        // season marker the matcher should honour.
        let title = self.titles.first().map(String::as_str).unwrap_or("");
        self.episode.map(|episode| {
            let season = query::split_season(title).1.unwrap_or(1);
            match self.absolute_episode {
                Some(absolute) => {
                    EpisodeRequest::anime_in_season_with_absolute(episode, absolute, season)
                }
                None => EpisodeRequest::anime_in_season(episode, season),
            }
        })
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
        // The queries are run one at a time rather than concurrently. Nyaa is a
        // scraped mirror, and a burst of simultaneous requests is the quickest
        // way to be throttled -- the wrong trade for a search a viewer is
        // waiting on. Results are merged and de-duplicated before ranking, since
        // the variants deliberately overlap.
        let mut found: Vec<Release> = Vec::new();
        for query in request.queries() {
            let mut batch = indexer.search(&query).await?;
            found.append(&mut batch);
        }

        dedupe(&mut found);
        Ok(refine(found, request, preference))
    }

    /// Drop releases that name the same torrent more than once.
    ///
    /// The query fan-out means the same release is routinely returned for
    /// several spellings, so the merge would otherwise show it repeatedly. The
    /// info hash is the real identity when the indexer gave one; otherwise the
    /// release name is compared with its spelling differences collapsed, so
    /// `Show_01` and `Show.01` are recognised as the same upload.
    ///
    /// Run before ranking: de-duplication is a set operation and the ranker's
    /// output is an order, so doing it afterwards would mean sorting entries
    /// that are about to be thrown away.
    pub fn dedupe(releases: &mut Vec<Release>) {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        releases.retain(|release| {
            let key = match release.info_hash.as_deref() {
                Some(hash) if !hash.trim().is_empty() => format!("hash:{}", hash.trim().to_lowercase()),
                _ => format!(
                    "title:{}",
                    normalize::normalize(&release.title).to_lowercase()
                ),
            };
            seen.insert(key)
        });
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
    let mut kept = match (request.mode, request.as_match()) {
        // A pack search keeps only the releases that ARE packs. Without this
        // branch `as_match` returning `None` would leave the list unfiltered,
        // handing back every single-episode release too.
        (SearchMode::Packs, _) => releases
            .into_iter()
            .filter(|release| parse::is_pack(&release.title))
            .collect(),
        (_, Some(matcher)) => releases
            .into_iter()
            .filter(|release| match_spec::matches(release, &matcher).is_ok())
            .collect(),
        // A film or whole-work search names no episode, so there is nothing to
        // filter against: the title match is the whole answer.
        (_, None) => releases,
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
        fn queries_include_a_padded_episode_spelling() {
            let request = ReleaseRequest {
                titles: vec!["Show".into()],
                episode: Some(5),
                absolute_episode: None,
                mode: SearchMode::Episodes,
            };
            let built = request.queries();
            assert!(
                built.contains(&"Show S01E05".to_string()),
                "got {built:?}"
            );
            assert!(built.contains(&"Show 05".to_string()), "got {built:?}");
        }

        #[test]
        fn queries_without_an_episode_are_just_the_titles() {
            let request = ReleaseRequest {
                titles: vec!["  Show  ".into()],
                episode: None,
                absolute_episode: None,
                mode: SearchMode::Episodes,
            };
            assert_eq!(request.queries(), vec!["Show".to_string()]);
        }

        #[test]
        fn dedupe_drops_the_same_info_hash_twice() {
            let mut releases = vec![
                release_named("[G] Show - 05 [1080p]"),
                release_named("[G] Show - 05 [1080p]"),
            ];
            // Give both the same hash, as two indexers reporting one torrent would.
            for release in releases.iter_mut() {
                release.info_hash = Some("AABBCC".into());
            }

            dedupe(&mut releases);
            assert_eq!(releases.len(), 1);
        }

        #[test]
        fn dedupe_compares_hashes_case_insensitively() {
            let mut releases = vec![
                release_named("[G] Show - 05 [1080p]"),
                release_named("[G] Another Name - 05 [1080p]"),
            ];
            releases[0].info_hash = Some("AABBCC".into());
            releases[1].info_hash = Some("aabbcc".into());

            dedupe(&mut releases);
            assert_eq!(releases.len(), 1);
        }

        #[test]
        fn dedupe_falls_back_to_the_release_name() {
            let mut releases = vec![
                release_named("[G] Show_05 [1080p]"),
                release_named("[G] Show.05 [1080p]"),
            ];
            // No hash on either, and normalise folds `_` and `.` to a space, so
            // the two names are the same upload.
            dedupe(&mut releases);
            assert_eq!(releases.len(), 1);
        }

        #[test]
        fn dedupe_keeps_genuinely_different_releases() {
            let mut releases = vec![
                release_named("[G] Show - 05 [1080p]"),
                release_named("[G] Show - 05 [720p]"),
            ];
            dedupe(&mut releases);
            assert_eq!(releases.len(), 2);
        }

    #[test]
    fn refine_drops_other_episodes_and_ranks_the_rest() {
        let request = ReleaseRequest {
            titles: vec!["Show".into()],
            episode: Some(5),
            absolute_episode: None,
            mode: SearchMode::Episodes,
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
    fn as_match_defaults_a_season_less_title_to_season_one() {
        // A season-1 entry is titled without a marker, but its releases are named
        // `S01E01`, so the matcher must be told season 1 to reject `S02E01`.
        let request = ReleaseRequest {
            titles: vec!["Mushoku Tensei: Jobless Reincarnation".into()],
            episode: Some(5),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let matcher = request.as_match().expect("an episode was requested");
        assert_eq!(matcher.season, Some(1));
        assert_eq!(matcher.episode, 5);
    }

    #[test]
    fn as_match_takes_the_season_the_title_states() {
        let request = ReleaseRequest {
            titles: vec!["Mushoku Tensei: Jobless Reincarnation Season 3".into()],
            episode: Some(9),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let matcher = request.as_match().expect("an episode was requested");
        assert_eq!(matcher.season, Some(3));
    }

    #[test]
    fn as_match_threads_the_absolute_episode() {
        // Both numbers reach the matcher, so a later cour's releases are
        // accepted under either spelling.
        let request = ReleaseRequest {
            titles: vec!["Show Season 2".into()],
            episode: Some(1),
            absolute_episode: Some(13),
            mode: SearchMode::Episodes,
        };
        let matcher = request.as_match().expect("an episode was requested");
        assert_eq!(matcher.episode, 1);
        assert_eq!(matcher.absolute_episode, Some(13));
        assert_eq!(matcher.season, Some(2));
    }

    #[test]
    fn as_match_defaults_the_absolute_number_to_the_episode() {
        // With no separate absolute number the two agree, which is the ordinary
        // season case.
        let request = ReleaseRequest {
            titles: vec!["Show".into()],
            episode: Some(5),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let matcher = request.as_match().expect("an episode was requested");
        assert_eq!(matcher.absolute_episode, Some(5));
    }

    #[test]
    fn as_match_is_none_without_an_episode() {
        // A film search must not acquire a season.
        let request = ReleaseRequest {
            titles: vec!["Some Movie".into()],
            episode: None,
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        assert!(request.as_match().is_none());
    }

    #[test]
    fn refine_keeps_a_season_one_release_and_drops_a_season_two_one() {
        let request = ReleaseRequest {
            titles: vec!["Show".into()],
            episode: Some(5),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let preference = ReleasePreference::default();

        let releases = vec![
            release_named("[G] Show S02E05 [1080p]"),
            release_named("[G] Show S01E05 [1080p]"),
        ];

        let refined = refine(releases, &request, &preference);

        assert_eq!(refined.len(), 1, "only the season-1 release survives");
        assert!(refined[0].title.contains("S01E05"));
    }

    #[test]
    fn refine_keeps_an_sxxexx_release_for_the_wanted_episode() {
        // The end-to-end case the query fan-out exists for: a release named
        // `S03E09` must survive filtering for episode 9 of a season-3 title.
        let request = ReleaseRequest {
            titles: vec!["Mushoku Tensei: Jobless Reincarnation Season 3".into()],
            episode: Some(9),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let preference = ReleasePreference::default();

        let releases = vec![
            release_named("[SubsPlease] Mushoku Tensei - 09 [1080p]"),
            release_named("[SubsPlease] Mushoku Tensei S03E09 [1080p]"),
            release_named("[SubsPlease] Mushoku Tensei S03E10 [1080p]"),
        ];

        let refined = refine(releases, &request, &preference);

        assert_eq!(refined.len(), 2, "episode 9 survives, episode 10 does not");
        assert!(
            refined.iter().any(|r| r.title.contains("S03E09")),
            "the SxxExx spelling must be kept: {refined:?}"
        );
    }

    #[test]
    fn refine_in_packs_mode_keeps_packs_and_drops_episodes() {
        // The whole point of the mode: an episode search rejects a season pack,
        // so a pack search is the only way to surface one.
        let request = ReleaseRequest {
            titles: vec!["Great Teacher Onizuka".into()],
            episode: Some(1),
            absolute_episode: None,
            mode: SearchMode::Packs,
        };
        let preference = ReleasePreference::default();

        let releases = vec![
            release_named("GTO Great Teacher Onizuka S01 1080p NF WEB-DL -VARYG"),
            release_named("[Group] Great Teacher Onizuka 01-43 [480p] (Batch)"),
            release_named("[SubsPlease] Great Teacher Onizuka - 01 [1080p]"),
        ];

        let refined = refine(releases, &request, &preference);

        assert_eq!(refined.len(), 2, "only the two packs survive: {refined:?}");
        assert!(
            refined.iter().all(|r| !r.title.contains("- 01 ")),
            "the single episode must be gone: {refined:?}"
        );
    }

    #[test]
    fn refine_in_episode_mode_still_drops_packs() {
        // The regression guard: adding Packs must not loosen Episodes.
        let request = ReleaseRequest {
            titles: vec!["Show".into()],
            episode: Some(5),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let preference = ReleasePreference::default();

        let releases = vec![
            release_named("[G] Show S01 1080p"),
            release_named("[G] Show - 05 [1080p]"),
        ];

        let refined = refine(releases, &request, &preference);

        assert_eq!(refined.len(), 1, "only episode 5 survives: {refined:?}");
        assert!(refined[0].title.contains("- 05"));
    }

    #[tokio::test]
    async fn search_fans_out_over_the_query_variants_and_dedupes() {
        // The stub returns the same release for every query, as a real indexer
        // would for overlapping spellings. The pipeline must collapse them.
        struct CountingIndexer;

        #[async_trait::async_trait]
        impl Indexer for CountingIndexer {
            fn name(&self) -> &str {
                "counting"
            }

            async fn search(&self, _query: &str) -> Result<Vec<Release>, IndexerError> {
                Ok(vec![release_named("[G] Show S01E05 [1080p]")])
            }
        }

        let request = ReleaseRequest {
            titles: vec!["Show".into(), "Shou".into()],
            episode: Some(5),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };

        let found = search(
            &CountingIndexer,
            &request,
            &ReleasePreference::default(),
        )
        .await
        .expect("stub should succeed");

        assert!(
            request.queries().len() > 1,
            "the request should fan out: {:?}",
            request.queries()
        );
        assert_eq!(found.len(), 1, "the repeat must be de-duplicated");
    }

    #[test]
    fn refine_without_an_episode_keeps_everything() {
        let request = ReleaseRequest {
            titles: vec!["Show".into()],
            episode: None,
            absolute_episode: None,
            mode: SearchMode::Episodes,
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
            titles: vec!["Show".into()],
            episode: Some(1),
            absolute_episode: None,
            mode: SearchMode::Episodes,
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
            titles: vec!["Show".into()],
            episode: Some(5),
            absolute_episode: None,
            mode: SearchMode::Episodes,
        };
        let releases = search(&indexer, &request, &ReleasePreference::default())
            .await
            .expect("stub should succeed");

        assert_eq!(releases.len(), 2);
        assert!(releases[0].title.contains("1080p"));
    }
}
