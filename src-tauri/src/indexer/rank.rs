//! Order candidate releases so the best pick is first.
//!
//! Ported from Sonarr's `DownloadDecisionComparer` chain, adapted to what an
//! anime search actually knows: resolution, source, remux, trusted uploader and
//! seeder count. Sonarr compares releases pairwise with a chain of comparers;
//! this collapses the same preferences into a single score, which is easier to
//! test and lets the UI show *why* a release is first.
//!
//! The score is only ever compared for order, never interpreted as a quantity,
//! so its absolute value is not meaningful -- only its ranking is.
//!
//! Pure: no network, no clock, no panics.

use crate::types::{Release, ReleasePreference};

/// Score one release against the viewer's preferences.
///
/// Higher is better. The weights are ordered so a resolution match always
/// outranks any accumulation of the smaller signals, which is what makes the
/// preferred resolution feel like a preference rather than a tiebreaker.
pub fn score(release: &Release, preference: &ReleasePreference) -> i64 {
    let mut total: i64 = 0;

    // Resolution: the top preference dominates, and each subsequent one is
    // worth progressively less, so "1080p then 720p" cannot be beaten by a
    // pile of seeders.
    if let Some(rank) = resolution_rank(release, preference) {
        // 1_000_000, 900_000, ... leaving room for everything below.
        total += 1_000_000 - (rank as i64) * 100_000;
    }

    // Source quality: up to ~10_000, which is deliberately less than one step
    // of resolution preference.
    total += release.source.rank() as i64 * 1_000;

    // Remux is a genuine quality bump on top of its source.
    if release.remux {
        total += 2_000;
    }

    // Trusted uploaders are worth a little more, but not enough to beat a
    // better source.
    if release.trusted {
        total += 1_000;
    }

    // Seeders matter for a stream, so log-compress them: 100 seeders should
    // not outweigh a resolution choice, but should beat 1 seeder.
    if let Some(seeders) = release.seeders {
        total += seeders_log_bonus(seeders);
    }

    total
}

/// The 0-based rank of a release's resolution in the preference list.
///
/// `None` when the release states no resolution or the preference does not
/// list it, meaning "no resolution credit" rather than a penalty.
fn resolution_rank(release: &Release, preference: &ReleasePreference) -> Option<usize> {
    if release.resolution == crate::types::Resolution::Unknown {
        return None;
    }
    preference
        .preferred_resolutions
        .iter()
        .position(|&candidate| candidate == release.resolution)
}

/// A logarithmic bonus for a seeder count, clamped so it cannot dominate.
///
/// Ported from Sonarr's `log10(seeders)` peers comparer. `log10` keeps the
/// difference between 10 and 100 seeders (one step) the same as between 100
/// and 1000, which matches how little extra help the additional peers give.
fn seeders_log_bonus(seeders: u32) -> i64 {
    if seeders == 0 {
        return 0;
    }
    // log10 * 100, capped: 1 seeder -> 0, 10 -> 100, 100 -> 200, 1000 -> 300.
    let bonus = (seeders as f64).log10() * 100.0;
    (bonus.round() as i64).clamp(0, 500)
}

/// Reject releases below the seeder floor, when one is set.
///
/// A floor of zero keeps everything. When a floor is set, releases that state
/// no seeder count are kept: "unknown" is not the same as "too few", and the
/// indexer may simply not have reported it.
pub fn passes_seeder_floor(release: &Release, preference: &ReleasePreference) -> bool {
    match release.seeders {
        Some(seeders) => seeders >= preference.min_seeders,
        None => true,
    }
}

/// Sort releases best-first, returning new indices in that order.
///
/// Ties are broken by title so the order is deterministic and a re-render does
/// not shuffle equal releases -- which would look like the app changing its
/// mind.
pub fn ranked_order(releases: &[Release], preference: &ReleasePreference) -> Vec<usize> {
    let mut scored: Vec<(usize, i64)> = releases
        .iter()
        .enumerate()
        .filter(|(_, release)| passes_seeder_floor(release, preference))
        .map(|(index, release)| (index, score(release, preference)))
        .collect();

    scored.sort_by(|(idx_a, score_a), (idx_b, score_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| releases[*idx_a].title.cmp(&releases[*idx_b].title))
    });

    scored.into_iter().map(|(index, _)| index).collect()
}

/// Reorder releases in place, best-first, stamping each with its score.
///
/// The score is written back onto the release so the UI can render a ranking
/// without recomputing it, and so the list the frontend receives is already
/// sorted rather than relying on the frontend to sort identically.
pub fn rank(releases: &mut Vec<Release>, preference: &ReleasePreference) {
    for release in releases.iter_mut() {
        release.score = score(release, preference);
    }

    let order = ranked_order(releases, preference);
    let mut reordered: Vec<Release> = Vec::with_capacity(order.len());
    // `order` holds indices into `releases`; drain by index instead of cloning.
    let mut slots: Vec<Option<Release>> = releases.drain(..).map(Some).collect();
    for index in order {
        if let Some(release) = slots[index].take() {
            reordered.push(release);
        }
    }

    *releases = reordered;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProviderId, ReleaseSource, Resolution};

    fn release_with(resolution: Resolution, source: ReleaseSource, seeders: u32) -> Release {
        Release {
            title: format!("release-{resolution:?}-{source:?}-{seeders}"),
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


    #[test]
    fn preferred_resolution_outranks_everything_else() {
        let preference = ReleasePreference::default();

        let preferred = release_with(Resolution::R1080p, ReleaseSource::Unknown, 1);
        let popular = release_with(Resolution::R720p, ReleaseSource::BluRay, 10_000);

        assert!(
            score(&preferred, &preference) > score(&popular, &preference),
            "1080p preference should beat a 720p BluRay with many seeders"
        );
    }

    #[test]
    fn a_better_source_raises_the_score() {
        let preference = ReleasePreference::default();

        let bluray = release_with(Resolution::R1080p, ReleaseSource::BluRay, 5);
        let hdtv = release_with(Resolution::R1080p, ReleaseSource::Hdtv, 5);

        assert!(score(&bluray, &preference) > score(&hdtv, &preference));
    }

    #[test]
    fn more_seeders_score_higher() {
        let preference = ReleasePreference::default();

        let many = release_with(Resolution::R1080p, ReleaseSource::WebDl, 500);
        let few = release_with(Resolution::R1080p, ReleaseSource::WebDl, 2);

        assert!(score(&many, &preference) > score(&few, &preference));
    }

    #[test]
    fn remux_is_worth_a_bump() {
        let preference = ReleasePreference::default();
        let mut plain = release_with(Resolution::R1080p, ReleaseSource::BluRay, 5);
        let mut remux = release_with(Resolution::R1080p, ReleaseSource::BluRay, 5);
        remux.remux = true;

        // Sanity: the two are otherwise identical.
        plain.score = 0;
        assert!(score(&remux, &preference) > score(&plain, &preference));
    }

    #[test]
    fn trusted_beats_untrusted_at_equal_quality() {
        let preference = ReleasePreference::default();
        let plain = release_with(Resolution::R1080p, ReleaseSource::WebDl, 5);
        let mut trusted = release_with(Resolution::R1080p, ReleaseSource::WebDl, 5);
        trusted.trusted = true;

        assert!(score(&trusted, &preference) > score(&plain, &preference));
    }

    #[test]
    fn seeders_do_not_outweigh_a_resolution_step() {
        let preference = ReleasePreference::default();

        // One preference step apart: 1080p (rank 0) vs 720p (rank 1). The gap
        // is 100_000, so even 100_000 seeders cannot close it.
        let preferred = release_with(Resolution::R1080p, ReleaseSource::Unknown, 0);
        let other = release_with(Resolution::R720p, ReleaseSource::Unknown, u32::MAX);

        assert!(score(&preferred, &preference) > score(&other, &preference));
    }

    #[test]
    fn unknown_resolution_gets_no_credit() {
        let preference = ReleasePreference::default();
        let unknown = release_with(Resolution::Unknown, ReleaseSource::BluRay, 100);
        let known = release_with(Resolution::R720p, ReleaseSource::Unknown, 0);

        assert!(score(&known, &preference) > score(&unknown, &preference));
    }

    #[test]
    fn seeder_floor_drops_low_releases() {
        let preference = ReleasePreference {
            preferred_resolutions: vec![Resolution::R1080p],
            min_seeders: 5,
        };

        let healthy = release_with(Resolution::R1080p, ReleaseSource::WebDl, 10);
        let dead = release_with(Resolution::R1080p, ReleaseSource::WebDl, 1);

        assert!(passes_seeder_floor(&healthy, &preference));
        assert!(!passes_seeder_floor(&dead, &preference));
    }

    #[test]
    fn unknown_seeders_survive_the_floor() {
        let preference = ReleasePreference {
            preferred_resolutions: vec![Resolution::R1080p],
            min_seeders: 5,
        };
        let mut release = release_with(Resolution::R1080p, ReleaseSource::WebDl, 0);
        release.seeders = None;

        assert!(passes_seeder_floor(&release, &preference));
    }

    #[test]
    fn ranked_order_puts_the_best_first() {
        let preference = ReleasePreference::default();
        let releases = vec![
            release_with(Resolution::R480p, ReleaseSource::Hdtv, 5),
            release_with(Resolution::R1080p, ReleaseSource::BluRay, 50),
            release_with(Resolution::R720p, ReleaseSource::WebDl, 20),
        ];

        let order = ranked_order(&releases, &preference);

        assert_eq!(order[0], 1, "1080p BluRay should be first");
        assert_eq!(order[1], 2, "720p WebDL should be second");
        assert_eq!(order[2], 0, "480p HDTV should be last");
    }

    #[test]
    fn rank_reorders_and_stamps_scores() {
        let preference = ReleasePreference::default();
        let mut releases = vec![
            release_with(Resolution::R480p, ReleaseSource::Hdtv, 5),
            release_with(Resolution::R1080p, ReleaseSource::BluRay, 50),
        ];

        rank(&mut releases, &preference);

        assert_eq!(releases[0].resolution, Resolution::R1080p);
        assert!(releases[0].score > releases[1].score);
    }

    #[test]
    fn equal_releases_order_by_title() {
        let preference = ReleasePreference::default();
        let mut a = release_with(Resolution::R1080p, ReleaseSource::BluRay, 5);
        let mut b = release_with(Resolution::R1080p, ReleaseSource::BluRay, 5);
        a.title = "Alpha".into();
        b.title = "Beta".into();

        let releases = vec![b, a];
        let order = ranked_order(&releases, &preference);

        assert_eq!(releases[order[0]].title, "Alpha");
    }
}