//! Re-rank releases using what the probes learned about them.
//!
//! [`super::rank`] orders releases by what the indexer *claimed*. This module
//! adds what the trackers and the swarm *confirmed*: real peer counts and
//! whether the metadata could actually be fetched. The two are summed, because
//! they answer different questions -- "is this the copy I want?" and "is this
//! copy still there?" -- and a release has to satisfy both.
//!
//! The weighting is a deliberate departure from `rank` in one respect. There,
//! resolution dominates everything so a preference feels like a preference.
//! Here, liveness dominates, because a dead release is not a lower-quality
//! choice, it is no choice at all: a torrent nobody is seeding cannot be
//! streamed, whatever its resolution.
//!
//! Pure: no network, no clock, no panics. The probe results are data by the
//! time they arrive here.

use serde::{Deserialize, Serialize};

use super::probe::{MetadataProbe, ProbeResult};
use super::rank;
use crate::types::{Release, ReleasePreference};

/// What a probe that fetched metadata is worth.
///
/// Larger than the entire static score range (a resolution match tops out at
/// about one million), so *any* release with confirmed metadata outranks *any*
/// release without it. That is the point: the probe exists to stop the app
/// recommending torrents that are no longer alive.
pub const METADATA_RESOLVED_BONUS: i64 = 2_000_000;

/// How much a real seeder count is scaled by, on top of `rank`'s log curve.
///
/// The log curve gives 0..500; scaled by this, a real scrape is worth up to
/// 500_000. That is enough to reorder releases of the same liveness by swarm
/// size -- a well-seeded release streams smoothly where a barely-seeded one
/// stalls -- but not enough to outrank confirmed metadata.
pub const SEEDER_BONUS_SCALE: i64 = 1_000;

/// The largest bonus a fast metadata fetch can earn.
///
/// Deliberately small: speed is a tiebreaker between equally live releases,
/// and it is the noisiest signal here, so it must never outweigh seeders. It
/// is set to the natural maximum of [`speed_bonus`] -- one point per tenth of
/// a second inside the metadata budget -- so the clamp only ever guards
/// against a duration that somehow exceeds the budget.
pub const SPEED_BONUS_MAX: i64 = 300;

/// A coarse health verdict for the UI, from one probe.
///
/// Three levels rather than a number because the underlying evidence is
/// coarse: "confirmed alive", "some sign of life", and "nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthBadge {
    /// Metadata arrived and a tracker reports real peers: the release is
    /// demonstrably downloadable.
    Green,
    /// Metadata arrived, or a tracker reports peers, but not both. Usually
    /// healthy, but not proven.
    Yellow,
    /// Neither. Either the release is dead or the probe could not reach
    /// anyone -- the UI should not promise it will play.
    Red,
}

/// How much a probe result should shift a release's rank.
///
/// Higher is better, like [`rank::score`], and the two are added together by
/// [`combined_score`]. A probe that learned nothing scores zero, so a failed
/// probe leaves a release exactly where the static ranking put it rather than
/// punishing it for our own timeout.
pub fn probe_score(probe: &ProbeResult) -> i64 {
    let mut total: i64 = 0;

    // Confirmed metadata is the headline signal. It is set high enough that
    // no amount of static quality can outrank it.
    if metadata_resolved(probe) {
        total += METADATA_RESOLVED_BONUS;
    }

    // Real seeders, log-compressed the same way the static ranker does it so
    // the two curves do not disagree about what "many seeders" means.
    if let Some(scrape) = &probe.scrape {
        total += rank::seeders_log_bonus(scrape.seeders) * SEEDER_BONUS_SCALE;
    }

    // A tiebreaker among confirmed releases only: an unresolved probe has no
    // fetch to have been fast.
    if let Some(metadata) = &probe.metadata {
        if metadata.resolved {
            total += speed_bonus(metadata);
        }
    }

    total
}

/// The static score plus whatever the probe is worth.
pub fn combined_score(static_score: i64, probe: &ProbeResult) -> i64 {
    static_score + probe_score(probe)
}

/// The badge for one probe result.
///
/// Green needs both halves of the evidence: metadata proves the swarm served
/// the info dictionary, and a tracker's peer count proves other peers are
/// present. Either alone is worth only Yellow, because a single source can be
/// misleading -- a tracker can list seeders that have since left, and one peer
/// can serve metadata for a torrent nobody will sustain.
///
/// Seeders that are *unknown* (no tracker answered) cannot reach Green, which
/// is what makes "no trackers listed" resolve to Yellow rather than a
/// confident Green.
pub fn health_badge(probe: &ProbeResult) -> HealthBadge {
    let resolved = metadata_resolved(probe);
    let seeders = probe.scrape.as_ref().map(|scrape| scrape.seeders).unwrap_or(0);

    if resolved && seeders > 10 {
        HealthBadge::Green
    } else if resolved || seeders > 0 {
        HealthBadge::Yellow
    } else {
        HealthBadge::Red
    }
}

/// Re-rank releases with probe data, stamping each with its combined score.
///
/// Probes are given as `(index, result)` pairs -- the shape
/// [`super::probe::probe_releases`] returns -- rather than matched by info
/// hash, because a release is not guaranteed to carry one and an index cannot
/// go missing. Pairs whose index is out of range are ignored, so a stale probe
/// from an earlier, longer list can never panic or mis-attribute a score.
///
/// The static half is recomputed from `preference` rather than read back from
/// `release.score`, which makes this idempotent: calling it twice, or calling
/// it after [`rank::rank`], gives the same answer instead of accumulating
/// bonuses.
///
/// Releases below the seeder floor are dropped, exactly as [`rank::rank`]
/// does, so the two produce comparable lists.
pub fn rank_with_probes(
    releases: &mut Vec<Release>,
    probes: &[(usize, ProbeResult)],
    preference: &ReleasePreference,
) {
    // Index by position: probes carry the index of the release they describe,
    // which is how the two are lined up without relying on info hash.
    let mut probe_scores: Vec<Option<i64>> = vec![None; releases.len()];
    for (index, probe) in probes {
        if let Some(slot) = probe_scores.get_mut(*index) {
            *slot = Some(probe_score(probe));
        }
    }

    for (index, release) in releases.iter_mut().enumerate() {
        let base = rank::score(release, preference);
        let probe = probe_scores.get(index).copied().flatten().unwrap_or(0);
        release.score = base + probe;
    }

    // Same rule as `rank`: drop what the seeder floor rejects, then order by
    // score with the title as a stable tiebreaker.
    let mut scored: Vec<(usize, i64)> = releases
        .iter()
        .enumerate()
        .filter(|(_, release)| rank::passes_seeder_floor(release, preference))
        .map(|(index, release)| (index, release.score))
        .collect();

    scored.sort_by(|(idx_a, score_a), (idx_b, score_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| releases[*idx_a].title.cmp(&releases[*idx_b].title))
    });

    let order: Vec<usize> = scored.into_iter().map(|(index, _)| index).collect();

    let mut reordered: Vec<Release> = Vec::with_capacity(order.len());
    let mut slots: Vec<Option<Release>> = releases.drain(..).map(Some).collect();
    for index in order {
        if let Some(release) = slots[index].take() {
            reordered.push(release);
        }
    }

    *releases = reordered;
}

/// Whether the metadata probe came back with an answer.
fn metadata_resolved(probe: &ProbeResult) -> bool {
    probe
        .metadata
        .as_ref()
        .is_some_and(|metadata| metadata.resolved)
}

/// A small bonus for how quickly metadata arrived.
///
/// Measured against the probe's own timeout rather than as an absolute
/// duration, so the value stays meaningful if the timeout is retuned. One
/// point per tenth of a second saved, which is why the whole 30s budget is
/// worth exactly [`SPEED_BONUS_MAX`].
fn speed_bonus(metadata: &MetadataProbe) -> i64 {
    let budget = super::probe::METADATA_TIMEOUT.as_millis() as u64;
    let used = metadata.duration_ms.min(budget);

    let saved_ms = budget.saturating_sub(used);
    ((saved_ms as i64) / 100).clamp(0, SPEED_BONUS_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::probe::{TrackerScrape, METADATA_TIMEOUT};
    use crate::types::{ProviderId, Release, ReleasePreference, ReleaseSource, Resolution};

    /// A probe that learned nothing: no tracker answered, no metadata came.
    fn empty_probe() -> ProbeResult {
        ProbeResult {
            info_hash: "cab507494d02ebb1178b38f2e9d7be299c86b862".into(),
            scrape: None,
            metadata: Some(MetadataProbe {
                resolved: false,
                file_count: None,
                total_bytes: None,
                duration_ms: METADATA_TIMEOUT.as_millis() as u64,
            }),
            total_duration_ms: METADATA_TIMEOUT.as_millis() as u64,
        }
    }

    fn resolved_metadata(duration_ms: u64) -> MetadataProbe {
        MetadataProbe {
            resolved: true,
            file_count: Some(1),
            total_bytes: Some(1_000_000),
            duration_ms,
        }
    }

    fn scrape_with(seeders: u32) -> TrackerScrape {
        TrackerScrape {
            seeders,
            leechers: 0,
            completed: 0,
            tracker_url: "udp://tracker.test:1337/announce".into(),
            duration_ms: 50,
        }
    }

    fn release_with(
        title: &str,
        resolution: Resolution,
        source: ReleaseSource,
        seeders: u32,
    ) -> Release {
        Release {
            title: title.to_string(),
            indexer: ProviderId::Nyaa,
            magnet_uri: "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862".into(),
            torrent_url: None,
            info_hash: None,
            size_bytes: None,
            seeders: Some(seeders),
            leechers: None,
            resolution,
            source,
            remux: false,
            trusted: false,
            parsed: Default::default(),
            score: 0,
        }
    }

    // --- probe_score ------------------------------------------------------

    #[test]
    fn a_probe_that_learned_nothing_scores_zero() {
        // The whole point: a failed probe must leave a release where the
        // static rank put it, not push it down for our own timeout.
        assert_eq!(probe_score(&empty_probe()), 0);
    }

    #[test]
    fn resolved_metadata_earns_the_full_bonus() {
        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(0));

        // At duration 0 the speed bonus is also maximal, so the total is the
        // metadata bonus plus SPEED_BONUS_MAX.
        assert_eq!(
            probe_score(&probe),
            METADATA_RESOLVED_BONUS + SPEED_BONUS_MAX
        );
    }

    #[test]
    fn unresolved_metadata_earns_no_bonus_at_all() {
        let mut probe = empty_probe();
        probe.scrape = Some(scrape_with(500));
        let with_scrape_only = probe_score(&probe);

        probe.metadata = Some(MetadataProbe {
            resolved: false,
            ..resolved_metadata(0)
        });

        assert_eq!(
            probe_score(&probe),
            with_scrape_only,
            "an unresolved probe must not add or subtract anything"
        );
    }

    #[test]
    fn resolved_metadata_outranks_the_largest_possible_static_score() {
        // The static score tops out at a resolution match (1_000_000) plus a
        // source (<= 10_000), a remux (2_000), trusted (1_000) and the seeder
        // curve (<= 500). If this bound ever stops holding, the probe can no
        // longer rescue a live-but-low-quality release, which is its job.
        let static_max = 1_000_000 + 10_000 + 2_000 + 1_000 + 500;
        assert!(
            METADATA_RESOLVED_BONUS > static_max,
            "metadata must outweigh every static signal combined"
        );
    }

    #[test]
    fn real_seeders_raise_the_score() {
        let mut many = empty_probe();
        many.scrape = Some(scrape_with(500));

        let mut few = empty_probe();
        few.scrape = Some(scrape_with(2));

        assert!(probe_score(&many) > probe_score(&few));
    }

    #[test]
    fn a_faster_metadata_fetch_scores_higher() {
        let mut fast = empty_probe();
        fast.metadata = Some(resolved_metadata(1_000));

        let mut slow = empty_probe();
        slow.metadata = Some(resolved_metadata(25_000));

        assert!(probe_score(&fast) > probe_score(&slow));
    }

    #[test]
    fn the_speed_bonus_cannot_outweigh_a_seeder_difference() {
        // Fastest possible fetch vs slowest, at equal liveness. The speed gap
        // is SPEED_BONUS_MAX; one order of magnitude of real seeders is worth
        // 100 * SEEDER_BONUS_SCALE, which is far larger.
        let mut fast_few = empty_probe();
        fast_few.metadata = Some(resolved_metadata(0));
        fast_few.scrape = Some(scrape_with(10));

        let mut slow_many = empty_probe();
        slow_many.metadata = Some(resolved_metadata(30_000));
        slow_many.scrape = Some(scrape_with(100));

        assert!(
            probe_score(&slow_many) > probe_score(&fast_few),
            "seeders must dominate the speed tiebreaker"
        );
    }

    #[test]
    fn combined_score_adds_the_two_halves() {
        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(30_000));
        let probe_only = probe_score(&probe);

        assert_eq!(combined_score(12_345, &probe), 12_345 + probe_only);
    }

    // --- health_badge -----------------------------------------------------

    #[test]
    fn green_needs_metadata_and_a_real_swarm() {
        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(1_000));
        probe.scrape = Some(scrape_with(50));

        assert_eq!(health_badge(&probe), HealthBadge::Green);
    }

    #[test]
    fn metadata_alone_is_yellow() {
        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(1_000));

        assert_eq!(health_badge(&probe), HealthBadge::Yellow);
    }

    #[test]
    fn a_few_seeders_with_metadata_is_yellow_not_green() {
        // Metadata resolved, but only a handful of peers: real, but not
        // something to promise a smooth stream on.
        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(1_000));
        probe.scrape = Some(scrape_with(3));

        assert_eq!(health_badge(&probe), HealthBadge::Yellow);
    }

    #[test]
    fn seeders_alone_are_yellow() {
        // A tracker that lists peers for a torrent whose metadata we could
        // not fetch: some sign of life, but unproven.
        let mut probe = empty_probe();
        probe.scrape = Some(scrape_with(50));

        assert_eq!(health_badge(&probe), HealthBadge::Yellow);
    }

    #[test]
    fn no_evidence_at_all_is_red() {
        assert_eq!(health_badge(&empty_probe()), HealthBadge::Red);
    }

    #[test]
    fn zero_seeders_and_no_metadata_is_red() {
        let mut probe = empty_probe();
        probe.scrape = Some(scrape_with(0));

        assert_eq!(health_badge(&probe), HealthBadge::Red);
    }

    #[test]
    fn unknown_seeders_cannot_reach_green() {
        // A magnet with no trackers: metadata resolves, but no tracker ever
        // reports a peer count. Green would be a promise we cannot keep.
        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(1_000));
        probe.scrape = None;

        assert_eq!(health_badge(&probe), HealthBadge::Yellow);
    }

    // --- rank_with_probes -------------------------------------------------

    #[test]
    fn a_live_release_overtakes_a_dead_one_of_higher_static_quality() {
        // The headline behaviour: the prettier release is dead, so the live
        // one must come first despite losing on resolution.
        let preference = ReleasePreference::default();

        let mut releases = vec![
            // Better static quality (1080p BluRay) but nothing is seeding it.
            release_with("dead 1080p", Resolution::R1080p, ReleaseSource::BluRay, 200),
            // Worse static quality (720p HDTV) but confirmed alive.
            release_with("live 720p", Resolution::R720p, ReleaseSource::Hdtv, 5),
        ];

        let mut live_probe = empty_probe();
        live_probe.metadata = Some(resolved_metadata(1_000));
        live_probe.scrape = Some(scrape_with(40));

        // Index 0 (the dead 1080p) gets an empty probe; index 1 gets the live
        // one.
        let probes = vec![(0, empty_probe()), (1, live_probe)];

        rank_with_probes(&mut releases, &probes, &preference);

        assert_eq!(
            releases[0].title, "live 720p",
            "confirmed liveness must beat resolution"
        );
    }

    #[test]
    fn re_ranking_is_idempotent() {
        // Scores are recomputed from `preference` each call, so a second pass
        // must not stack the probe bonus again.
        let preference = ReleasePreference::default();

        let mut releases = vec![release_with("a", Resolution::R1080p, ReleaseSource::WebDl, 5)];

        let mut probe = empty_probe();
        probe.scrape = Some(scrape_with(100));
        let probes = vec![(0, probe)];

        rank_with_probes(&mut releases, &probes, &preference);
        let after_first = releases[0].score;

        rank_with_probes(&mut releases, &probes, &preference);
        assert_eq!(
            releases[0].score, after_first,
            "a second pass must not accumulate the bonus"
        );
    }

    #[test]
    fn an_out_of_range_probe_index_is_ignored() {
        let preference = ReleasePreference::default();
        let mut releases = vec![release_with("only", Resolution::R1080p, ReleaseSource::WebDl, 5)];

        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(0));

        // Index 99 does not exist; this must not panic and must not score
        // release 0.
        let probes = vec![(99, probe)];

        rank_with_probes(&mut releases, &probes, &preference);

        let base = rank::score(&releases[0], &preference);
        assert_eq!(releases[0].score, base, "no probe should have applied");
    }

    #[test]
    fn releases_below_the_seeder_floor_are_dropped() {
        let preference = ReleasePreference {
            preferred_resolutions: vec![Resolution::R1080p],
            min_seeders: 10,
        };

        let mut releases = vec![
            release_with("healthy", Resolution::R1080p, ReleaseSource::WebDl, 20),
            release_with("dead", Resolution::R1080p, ReleaseSource::WebDl, 1),
        ];

        rank_with_probes(&mut releases, &[], &preference);

        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].title, "healthy");
    }

    #[test]
    fn a_probe_for_an_unknown_index_does_not_shift_a_known_one() {
        let preference = ReleasePreference::default();
        let mut releases = vec![
            release_with("a", Resolution::R1080p, ReleaseSource::WebDl, 5),
            release_with("b", Resolution::R1080p, ReleaseSource::WebDl, 5),
        ];

        let mut probe = empty_probe();
        probe.metadata = Some(resolved_metadata(0));

        // Only release 1 is probed; release 0 must keep its static score.
        let probes = vec![(1, probe)];
        rank_with_probes(&mut releases, &probes, &preference);

        let base = rank::score(
            &release_with("a", Resolution::R1080p, ReleaseSource::WebDl, 5),
            &preference,
        );
        // Both have identical static scores, so the probed one must now lead
        // and the unprobed one must still carry exactly its static score.
        assert_eq!(releases[0].title, "b");
        assert_eq!(releases[1].score, base);
    }

    #[test]
    fn ranking_with_no_probes_matches_the_static_ranking() {
        let preference = ReleasePreference::default();

        let mut probed = vec![
            release_with("low", Resolution::R480p, ReleaseSource::Hdtv, 5),
            release_with("high", Resolution::R1080p, ReleaseSource::BluRay, 50),
        ];
        let mut statically = probed.clone();

        rank_with_probes(&mut probed, &[], &preference);
        rank::rank(&mut statically, &preference);

        assert_eq!(
            probed.iter().map(|r| &r.title).collect::<Vec<_>>(),
            statically.iter().map(|r| &r.title).collect::<Vec<_>>(),
            "with no probes the two rankings must agree"
        );
        assert_eq!(probed[0].score, statically[0].score);
    }
}