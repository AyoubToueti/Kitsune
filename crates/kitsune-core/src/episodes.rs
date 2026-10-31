//! Episode lists that survive a sparse provider.
//!
//! Ported from the Svelte frontend's `src/lib/episodes.ts` and
//! `src/lib/episode.ts`, so both frontends share one set of rules. The list
//! comes from Jikan's per-episode catalogue, bridged by AniList's `idMal`.
//! AniList's own `streamingEpisodes` is NOT used here: it is a list of licensed
//! streaming links, not an episode list, and AniList attaches the whole
//! franchise's links to the first season's entry -- so using it produced a
//! season-1 list numbered 51..66 for Re:Zero.
//!
//! When Jikan has nothing, this synthesises `1..episodeCount` so the reader can
//! still reach every episode -- and, critically, so the watch page can resolve
//! an episode number to search a torrent for.
//!
//! The number is the load-bearing part. Everything downstream (the torrent
//! matcher, the file matcher) keys off "which episode is this", so every entry
//! carries an explicit number rather than relying on the title being parsed.

use once_cell::sync::Lazy;
use fancy_regex::Regex;
use serde::{Deserialize, Serialize};

use crate::types::{Anime, EpisodeInfo, NextAiringEpisode};

/// An episode as the UI renders it.
///
/// Deliberately looser than [`crate::types::StreamingEpisode`]: a synthesised
/// entry has no licensed URL to open, so `url` is optional and the card falls
/// back to selecting the episode rather than opening a link. Every entry keeps
/// an explicit `number` so the torrent and file matchers never have to recover
/// it from the title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    /// 1-based episode number, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// Air date as the provider spells it, filled in by enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aired: Option<String>,
    /// Unix seconds at which this episode airs, set ONLY on the trailing entry
    /// that represents an episode which has not aired yet.
    ///
    /// Its presence is what marks the entry as upcoming: a real episode carries
    /// its `aired` date instead, never a future timestamp. The frontend formats
    /// this, so the reader's locale and timezone decide the spelling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub airing_at: Option<i64>,
    /// Anime-original filler, which a viewer may want to skip.
    #[serde(default)]
    pub filler: bool,
    /// A recap of earlier events.
    #[serde(default)]
    pub recap: bool,
}

/// Build `1..count` synthetic episodes.
///
/// `cover_image` is used as the card background so the grid reads as part of
/// the work rather than a row of blanks. A count that is absent or non-positive
/// yields an empty list: there is nothing honest to show, and the caller
/// renders an "unknown" message instead.
pub fn synthesize_episodes(count: Option<u32>, cover_image: Option<&str>) -> Vec<Episode> {
    let Some(count) = count.filter(|count| *count > 0) else {
        return Vec::new();
    };

    (1..=count)
        .map(|number| Episode {
            number: Some(number),
            // Spelled the way `episode_number` reads back, so selection and
            // torrent matching work without a second numbering scheme.
            title: Some(format!("Episode {number}")),
            thumbnail: cover_image.map(str::to_string),
            url: None,
            site: None,
            aired: None,
            airing_at: None,
            filler: false,
            recap: false,
        })
        .collect()
}

/// Turn a provider's per-episode catalogue into display entries.
///
/// Every entry keeps its explicit `number`, so the torrent matcher never has to
/// recover it from the title.
///
/// The card image prefers the EPISODE's own still and falls back to the work's
/// poster. Kitsu supplies per-episode thumbnails; Jikan does not, so a Jikan
/// catalogue still gets the poster and looks exactly as it did before. Cloning
/// the still onto each entry is what makes the episode grid read as individual
/// episodes rather than a wall of identical posters.
pub fn episodes_from_info(info: &[EpisodeInfo], cover_image: Option<&str>) -> Vec<Episode> {
    info.iter()
        .map(|entry| Episode {
            number: Some(entry.number),
            title: Some(
                entry
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("Episode {}", entry.number)),
            ),
            thumbnail: entry
                .thumbnail
                .clone()
                .or_else(|| cover_image.map(str::to_string)),
            url: None,
            site: None,
            aired: entry.aired.clone(),
            airing_at: None,
            filler: entry.filler,
            recap: entry.recap,
        })
        .collect()
}

/// Whether a catalogue entry has already aired.
///
/// A missing or unparseable date counts as aired. The field is a bonus, and
/// withholding an episode over a malformed date would silently drop data the
/// provider did give us; only a date that parses *and* lies in the future is
/// held back. Airing schedules are also approximate, so a same-day episode is
/// shown rather than hidden for a few hours.
///
/// `now_unix` is unix seconds and is injectable so a test can pin the clock.
pub fn is_aired(aired: Option<&str>, now_unix: i64) -> bool {
    is_aired_on(aired, now_unix, false)
}

/// [`is_aired`], with the releasing case made explicit.
///
/// `releasing` changes what an ABSENT date means. For a finished work a missing
/// date is just a gap in the provider's data, and the episode has aired. For a
/// work that is still airing it means the opposite: Kitsu creates a record for
/// every episode of the cour up front and fills `airdate` in only as each one
/// airs, so a record with no date is an episode that has NOT aired yet.
///
/// Getting this wrong is what drew a full 12-card grid for a show four weeks
/// into its run -- Kitsu reported twelve records, all undated, and every one of
/// them counted as watchable.
pub fn is_aired_on(aired: Option<&str>, now_unix: i64, releasing: bool) -> bool {
    let Some(aired) = aired else {
        return !releasing;
    };
    if aired.trim().is_empty() {
        return !releasing;
    }

    match parse_date_unix(aired) {
        Some(at) => at <= now_unix,
        // Unparseable counts as aired.
        None => true,
    }
}

/// The catalogue entries that have aired, in catalogue order.
///
/// An unaired episode is filtered out of the list rather than shown disabled:
/// the reader cannot watch it, and a row that does nothing when clicked reads
/// as broken.
pub fn aired_episodes(info: &[EpisodeInfo], now_unix: i64) -> Vec<EpisodeInfo> {
    aired_episodes_on(info, now_unix, false)
}

/// [`aired_episodes`], with the releasing case made explicit.
///
/// See [`is_aired_on`] for why an undated record means "not out yet" on a work
/// that is still airing and "already out" on one that has finished.
pub fn aired_episodes_on(
    info: &[EpisodeInfo],
    now_unix: i64,
    releasing: bool,
) -> Vec<EpisodeInfo> {
    info.iter()
        .filter(|entry| is_aired_on(entry.aired.as_deref(), now_unix, releasing))
        .cloned()
        .collect()
}

/// The episodes to show for a work.
///
/// The catalogue wins when it has entries, because it is the real per-episode
/// data: correct numbering, real titles and air dates. Only when it is empty
/// does this fall back to a synthesised `1..episodeCount` stand-in, so a work
/// with no MAL link (or a MAL entry with no episode list) is still navigable.
///
/// Entries that have not aired yet are dropped. A catalogue that is entirely
/// unaired therefore yields an empty list, which is the honest answer -- it must
/// not fall back to synthesis, because that would invent viewable episodes.
///
/// A catalogue that is merely *short* is padded up to the announced count by
/// [`with_missing_episodes`], but only for a finished work.
///
/// `now_unix` defaults to the current time; see [`now_unix`].
pub fn episodes_for(anime: &Anime, info: &[EpisodeInfo], now_unix: i64) -> Vec<Episode> {
    episodes_for_facts(
        anime.episode_count,
        anime.status.as_deref(),
        anime.cover_image.as_deref(),
        info,
        anime.aired_episodes,
        anime.next_airing,
        now_unix,
    )
}

/// The scalar-facts form of [`episodes_for`].
///
/// Lets a frontend that only has the numbers (the Qt app crosses the FFI with
/// scalars, not a whole `Anime`) get the same rules. [`episodes_for`] delegates
/// here, so there is exactly one implementation.
pub fn episodes_for_facts(
    episode_count: Option<u32>,
    status: Option<&str>,
    cover_image: Option<&str>,
    info: &[EpisodeInfo],
    aired_through: Option<u32>,
    next_airing: Option<NextAiringEpisode>,
    now_unix: i64,
) -> Vec<Episode> {
    // The schedule IS an aired count: `next_airing.episode` is the one ABOUT
    // to air, so one less have already aired.
    //
    // It is combined with the catalogue's count by taking the MINIMUM, not by
    // preferring one. Both can overshoot:
    //
    // - Kitsu's `meta.count` includes episodes it has PLANNED but not aired,
    //   so One Piece reported 1410 against ~1181 actually out.
    // - A schedule is the provider's plan, and can run ahead of what shipped.
    //
    // Whichever is lower is the safer claim: it can only under-offer an
    // episode, never present an unbroadcast one as watchable.
    let schedule_aired = next_airing.map(|next| next.episode.saturating_sub(1));
    let aired_through = match (aired_through, schedule_aired) {
        (Some(catalogue), Some(schedule)) => Some(catalogue.min(schedule)),
        (catalogue, schedule) => catalogue.or(schedule),
    };

    let mut episodes = episodes_without_upcoming(
        episode_count,
        status,
        cover_image,
        info,
        aired_through,
        now_unix,
    );

    // The upcoming entry goes LAST, after the aired list, because that is where
    // it belongs in time. Appending rather than merging keeps every existing
    // rule -- padding, capping, the catalogue winning -- untouched.
    if let Some(next) = next_airing {
        append_upcoming(&mut episodes, next, cover_image, now_unix);
    }

    episodes
}

/// The aired entries for a work, before the upcoming one is appended.
fn episodes_without_upcoming(
    episode_count: Option<u32>,
    status: Option<&str>,
    cover_image: Option<&str>,
    info: &[EpisodeInfo],
    aired_through: Option<u32>,
    now_unix: i64,
) -> Vec<Episode> {
    if !info.is_empty() {
        // A releasing work's undated records are episodes that have not aired,
        // not episodes with a missing date. See `is_aired_on`.
        let releasing = status == Some("RELEASING");
        let known =
            episodes_from_info(&aired_episodes_on(info, now_unix, releasing), cover_image);

        // A catalogue that filtered down to NOTHING is not an answer. Kitsu
        // returns a record per planned episode with `airdate: null` until each
        // airs, so an airing cour can arrive as twelve undated rows and no
        // watchable ones. Falling through to synthesis below draws the episodes
        // that HAVE aired -- the aired count is known even when the per-episode
        // dates are not.
        if !known.is_empty() {
            // A finished work is padded to its announced total: every episode
            // has aired, so a missing entry is one the provider has not titled.
            if status == Some("FINISHED") {
                return with_missing_episodes(known, episode_count, cover_image);
            }

            // A RELEASING work is padded to what has AIRED, never to the
            // announced total -- the difference is the whole point.
            //
            // The case this exists for is a long runner. Paging stops at a
            // hundred records, so One Piece arrived with a hundred aired
            // entries and an aired count of 1181: the grid showed 1..100 and
            // then jumped to the next-airing card. Filling 101..1181 with bare
            // numbered entries is what Naruto and Bleach already look like,
            // and it costs no requests.
            //
            // `with_missing_episodes` refuses a list that does not start at 1,
            // so a later cour (13..24) is left alone rather than padded against
            // a franchise total it does not share.
            if releasing {
                if let Some(aired) = aired_through {
                    return with_missing_episodes(known, Some(aired), cover_image);
                }
            }

            return known;
        }
    }

    // No catalogue to show, so the list has to be synthesised. The count to
    // synthesise from is the crux of the "unaired episodes" bug:
    //
    // - `episode_count` is the ANNOUNCED total, which for a releasing work
    //   includes episodes that have not aired yet.
    // - `aired_through` is how many have ACTUALLY aired.
    //
    // A releasing work must be capped at the aired count, otherwise the grid
    // offers episodes the reader cannot watch. A finished work ignores the cap
    // (everything has aired), and a work with no announced total (One Piece)
    // uses the aired count as its only source of length.
    let ceiling = match (status, aired_through) {
        // Nothing has aired, so nothing is watchable. Without this arm the
        // announced total leaked through `_ => episode_count` and an unreleased
        // work offered a full grid of episodes that do not exist yet.
        (Some("NOT_YET_RELEASED"), _) => Some(0),
        // A releasing work is capped at what has aired. `aired_through` is
        // derived from the schedule when the catalogue did not supply it, which
        // is what stops the whole announced cour being drawn when the episode
        // fetch fails.
        //
        // An unknown aired count yields NOTHING rather than the announced
        // total. Drawing 26 episodes for a cour three weeks in offers the
        // reader two dozen episodes they cannot watch, which is the exact bug
        // this arm exists to prevent -- a short grid is honest, a long one is
        // not.
        (Some("RELEASING"), aired) => match (episode_count, aired) {
            (Some(total), Some(aired)) => Some(total.min(aired)),
            (None, Some(aired)) => Some(aired),
            (_, None) => Some(0),
        },
        // No status, but an aired count and no announced total: the aired count
        // is still better than nothing.
        (_, Some(aired)) if episode_count.is_none() => Some(aired),
        _ => episode_count,
    };

    synthesize_episodes(ceiling, cover_image)
}

/// Append the not-yet-aired episode as a trailing entry.
///
/// Two guards, both about not lying to the reader:
///
/// - An entry with that number already present means the provider disagrees
///   with its own schedule, so the real entry wins and nothing is added.
/// - A timestamp in the past means the schedule is stale (the airing already
///   happened but the pull predates it), so the entry would claim a future
///   broadcast that is not in the future.
fn append_upcoming(
    episodes: &mut Vec<Episode>,
    next: NextAiringEpisode,
    cover_image: Option<&str>,
    now_unix: i64,
) {
    if episodes
        .iter()
        .any(|episode| episode.number == Some(next.episode))
    {
        return;
    }
    // `now_unix` is a parameter, not a call, for the same reason everywhere
    // else in this module: a test must be able to pin the clock.
    if next.airing_at <= now_unix {
        return;
    }

    episodes.push(Episode {
        number: Some(next.episode),
        // Deliberately no title: the provider has not published one for an
        // episode that has not aired, and inventing "Episode 7" would read as
        // a real title once the grid sorts by it.
        title: None,
        thumbnail: cover_image.map(str::to_string),
        url: None,
        site: None,
        aired: None,
        airing_at: Some(next.airing_at),
        filler: false,
        recap: false,
    });
}

/// Fill the gaps between a catalogue and the count the provider announced.
///
/// A bare entry is deliberately generic: it carries a number and nothing else.
/// What it buys is reachability: the reader can select it, and the watch page
/// can search for it.
///
/// Only a 1-based list is filled. A later cour is numbered 13..24 while its own
/// list runs 1..12, and padding that to the franchise total would invent the
/// previous cour's episodes. Requiring the lowest number to be 1 keeps the two
/// cases apart.
fn with_missing_episodes(
    known: Vec<Episode>,
    count: Option<u32>,
    cover_image: Option<&str>,
) -> Vec<Episode> {
    let Some(count) = count.filter(|count| *count > 0) else {
        return known;
    };

    let numbers: Vec<u32> = known.iter().filter_map(|episode| episode.number).collect();

    // Without numbers there is nothing to compare against, and a guess would
    // put entries in the wrong slots.
    if numbers.is_empty() {
        return known;
    }

    let lowest = *numbers.iter().min().unwrap();
    let highest = *numbers.iter().max().unwrap();

    // `lowest != 1` means this is not a 1-based cour; `highest > count` means
    // the catalogue is already numbered beyond what was announced.
    if lowest != 1 || highest > count || known.len() as u32 >= count {
        return known;
    }

    let present: std::collections::HashSet<u32> = numbers.into_iter().collect();
    let mut filled = known;

    for number in 1..=count {
        if present.contains(&number) {
            continue;
        }
        filled.push(Episode {
            number: Some(number),
            title: Some(format!("Episode {number}")),
            thumbnail: cover_image.map(str::to_string),
            url: None,
            site: None,
            aired: None,
            airing_at: None,
            filler: false,
            recap: false,
        });
    }

    filled.sort_by_key(|episode| episode.number.unwrap_or(0));
    filled
}

/// Whether a work has any episode information at all.
///
/// False means neither a provider list nor a count, so the caller should say so
/// rather than render an empty grid that looks broken.
pub fn has_episode_data(anime: &Anime) -> bool {
    !anime.streaming_episodes.is_empty() || anime.episode_count.unwrap_or(0) > 0
}

// --- episode numbering ------------------------------------------------------

/// `Episode 12`, `Ep. 12`, or `episode-12` inside a URL.
static EPISODE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:episode|ep)\.?\s*[-–:]?\s*(\d+)").unwrap());
/// `episode-12` with a separator, catching URL spellings the first misses.
static EPISODE_SEP_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:episode|ep)[-_](\d+)").unwrap());
/// A bare trailing number.
static TRAILING_NUMBER: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)\s*$").unwrap());
/// A title that already announces its own episode number.
static TITLE_HAS_NUMBER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*(?:ep(?:isode)?\.?\s*|e)\d+").unwrap());

/// The episode number an entry represents, when one can be told.
///
/// An explicit `number` wins outright. Otherwise the title is tried before the
/// URL because it is written for humans and so carries the number most
/// reliably; the URL patterns catch entries whose title is a site name or blank.
pub fn episode_number(ep: &Episode) -> Option<u32> {
    if let Some(number) = ep.number {
        return Some(number);
    }

    for source in [ep.title.as_deref(), ep.url.as_deref()] {
        let Some(source) = source else {
            continue;
        };
        for pattern in [&*EPISODE_PATTERN, &*EPISODE_SEP_PATTERN, &*TRAILING_NUMBER] {
            if let Ok(Some(captures)) = pattern.captures(source) {
                if let Some(found) = captures.get(1) {
                    if let Ok(number) = found.as_str().parse::<u32>() {
                        return Some(number);
                    }
                }
            }
        }
    }
    None
}

/// Whether a title already announces its own episode number.
///
/// Some titles carry the number themselves: a padded catalogue entry is titled
/// `Episode 7`. Prefixing those again would read "Episode 7 - Episode 7", so
/// the caller leaves them as they are. Anchored at the start on purpose: a
/// number appearing later is part of a name rather than an announcement.
pub fn title_has_episode_number(title: &str) -> bool {
    TITLE_HAS_NUMBER.is_match(title).unwrap_or(false)
}

/// Index of the entry for `number`, or `None` when nothing matches.
pub fn index_of_episode(episodes: &[Episode], number: u32) -> Option<usize> {
    episodes
        .iter()
        .position(|ep| episode_number(ep) == Some(number))
}

/// How much to add to a list position to get the work's absolute episode number.
///
/// A later cour is often numbered 13..24 while its releases are named
/// `Show - 13`. The offset bridges the two: it is the lowest number in the list
/// minus one, but only when that lowest number is greater than the list length.
/// That guard keeps an ordinary 1..N season at zero.
pub fn absolute_offset(episodes: &[Episode]) -> u32 {
    let numbers: Vec<u32> = episodes.iter().filter_map(episode_number).collect();
    if numbers.is_empty() {
        return 0;
    }

    let lowest = *numbers.iter().min().unwrap();
    if lowest > episodes.len() as u32 {
        lowest - 1
    } else {
        0
    }
}

// --- time helpers -----------------------------------------------------------

/// The current time in unix seconds.
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Parse the leading `YYYY-MM-DD` of a date string to unix seconds (UTC).
///
/// A full ISO-8601 timestamp is accepted; only the date portion is used and the
/// time-of-day and zone are ignored, because an air date is a day, not an
/// instant. Returns `None` when no date can be read, which the caller treats as
/// "aired".
fn parse_date_unix(s: &str) -> Option<i64> {
    let trimmed = s.trim();
    if trimmed.len() < 10 {
        return None;
    }
    let head = &trimmed[..10];

    let mut parts = head.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    Some(days_from_civil(year, month, day) * 86_400)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// algorithm). Kept local so the crate needs no date dependency.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProviderId, Title};

    fn info(number: u32, aired: Option<&str>) -> EpisodeInfo {
        EpisodeInfo {
            number,
            title: None,
            aired: aired.map(str::to_string),
            thumbnail: None,
            filler: false,
            recap: false,
        }
    }

    fn anime(status: Option<&str>, count: Option<u32>) -> Anime {
        Anime {
            id: 1,
            provider: ProviderId::AniList,
            title: Title::default(),
            id_mal: Some(1),
            cover_image: None,
            banner_image: None,
            description: None,
            episode_count: count,
            aired_episodes: None,
            next_airing: None,
            duration_minutes: None,
            format: None,
            genres: Vec::new(),
            average_score: None,
            popularity: None,
            status: status.map(str::to_string),
            season: None,
            season_year: None,
            streaming_episodes: Vec::new(),
            relations: Vec::new(),
            recommendations: Vec::new(),
            trailer: None,
        }
    }

    #[test]
    fn synthesize_builds_one_to_count() {
        let episodes = synthesize_episodes(Some(3), Some("cover.jpg"));
        assert_eq!(episodes.len(), 3);
        assert_eq!(episodes[0].number, Some(1));
        assert_eq!(episodes[2].number, Some(3));
        assert_eq!(episodes[0].title.as_deref(), Some("Episode 1"));
        assert_eq!(episodes[0].thumbnail.as_deref(), Some("cover.jpg"));
    }

    #[test]
    fn synthesize_is_empty_without_a_count() {
        assert!(synthesize_episodes(None, None).is_empty());
        assert!(synthesize_episodes(Some(0), None).is_empty());
    }

    #[test]
    fn dates_in_the_future_are_not_aired() {
        // 2100-01-01 is comfortably in the future.
        assert!(!is_aired(Some("2100-01-01"), now_unix()));
    }

    #[test]
    fn past_and_missing_dates_are_aired() {
        assert!(is_aired(Some("2000-01-01"), now_unix()));
        assert!(is_aired(None, now_unix()));
        assert!(is_aired(Some("not a date"), now_unix()));
        assert!(is_aired(Some(""), now_unix()));
    }

    #[test]
    fn an_undated_record_on_a_releasing_work_has_not_aired() {
        // THE BUG THIS GUARDS: Kitsu returns a record for EVERY planned
        // episode, with `airdate: null` until each airs. Counting those as
        // aired drew a full 12-card grid for a cour four weeks into its run.
        assert!(!is_aired_on(None, now_unix(), true));
        assert!(!is_aired_on(Some(""), now_unix(), true));
        // A real past date is still aired on a releasing work.
        assert!(is_aired_on(Some("2000-01-01"), now_unix(), true));
        // A real future date is still not.
        assert!(!is_aired_on(Some("2100-01-01"), now_unix(), true));
    }

    #[test]
    fn an_undated_record_on_a_finished_work_is_aired() {
        // The old behaviour is correct here: a finished work has aired
        // everything, so a gap in the provider's dates is not evidence of an
        // unbroadcast episode.
        assert!(is_aired_on(None, now_unix(), false));
    }

    #[test]
    fn a_releasing_catalogue_of_undated_records_falls_back_to_the_aired_count() {
        // THE REPORTED SHAPE: twelve records, none with an `airdate`, four
        // having actually aired. The undated rows are correctly dropped, and
        // the aired count draws the four that are watchable -- not twelve, and
        // not zero.
        let catalogue: Vec<EpisodeInfo> = (1..=12)
            .map(|number| info(number, None))
            .collect();

        let next = NextAiringEpisode {
            episode: 5,
            airing_at: now_unix() + 86_400,
        };
        let episodes = episodes_for_facts(
            Some(12),
            Some("RELEASING"),
            None,
            &catalogue,
            None,
            Some(next),
            now_unix(),
        );

        assert_eq!(episodes.len(), 5, "4 aired plus the upcoming entry");
        let numbers: Vec<_> = episodes.iter().filter_map(|e| e.number).collect();
        assert_eq!(numbers, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn the_same_catalogue_on_a_finished_work_still_shows() {
        let catalogue: Vec<EpisodeInfo> = (1..=12)
            .map(|number| info(number, None))
            .collect();

        let episodes = episodes_for(
            &anime(Some("FINISHED"), Some(12)),
            &catalogue,
            now_unix(),
        );

        assert_eq!(episodes.len(), 12);
    }

    #[test]
    fn finished_works_are_padded_to_the_announced_count() {
        let catalogue = vec![info(1, Some("2000-01-01"))];
        let episodes = episodes_for(&anime(Some("FINISHED"), Some(3)), &catalogue, now_unix());
        assert_eq!(episodes.len(), 3);
        assert_eq!(episodes[1].number, Some(2));
    }

    #[test]
    fn releasing_works_are_not_padded() {
        let catalogue = vec![info(1, Some("2000-01-01"))];
        let episodes = episodes_for(&anime(Some("RELEASING"), Some(3)), &catalogue, now_unix());
        assert_eq!(episodes.len(), 1);
    }

    #[test]
    fn a_releasing_work_with_no_aired_info_shows_nothing() {
        // WAS `empty_catalogue_falls_back_to_synthesis`, which asserted 4.
        // That encoded the behaviour where a releasing work with no aired
        // count drew its whole announced cour -- 26 watchable-looking cards for
        // a show three weeks into its run. There is nothing to cap against, so
        // the honest answer is an empty grid rather than an over-long one.
        let episodes = episodes_for(&anime(Some("RELEASING"), Some(4)), &[], now_unix());
        assert!(episodes.is_empty());
    }

    #[test]
    fn a_finished_work_still_falls_back_to_synthesis() {
        // The guard above is scoped to RELEASING: a finished work has aired
        // everything, so the announced total IS the real list.
        let episodes = episodes_for(&anime(Some("FINISHED"), Some(4)), &[], now_unix());
        assert_eq!(episodes.len(), 4);
    }

    #[test]
    fn an_unreleased_work_has_no_episodes() {
        // THE BUG THIS GUARDS: the ceiling match had no unreleased arm, so the
        // announced count fell through `_ => episode_count` and an unreleased
        // work offered a full grid of episodes that do not exist yet.
        let episodes = episodes_for(&anime(Some("NOT_YET_RELEASED"), Some(12)), &[], now_unix());
        assert!(
            episodes.is_empty(),
            "an unreleased work must not offer a synthesised grid"
        );
    }

    #[test]
    fn an_unreleased_work_ignores_a_catalogue_too() {
        // Even a stale catalogue entry cannot make an unreleased work playable:
        // the entries are dropped as unaired, and synthesis is capped at zero.
        let catalogue = vec![info(1, Some("2100-01-01"))];
        let episodes = episodes_for(
            &anime(Some("NOT_YET_RELEASED"), Some(12)),
            &catalogue,
            now_unix(),
        );
        assert!(episodes.is_empty());
    }

    #[test]
    fn a_releasing_work_is_capped_by_its_schedule_alone() {
        // THE BUG THIS GUARDS: the catalogue fetch had failed, so there was no
        // `aired_through`. The match fell through to the announced total and
        // drew all 14 episodes. The schedule says ep 5 is next, so 4 have
        // aired -- that is the cap.
        let next = NextAiringEpisode {
            episode: 5,
            airing_at: now_unix() + 86_400,
        };
        let episodes = episodes_for_facts(
            Some(14),
            Some("RELEASING"),
            None,
            &[],
            None,
            Some(next),
            now_unix(),
        );

        assert_eq!(episodes.len(), 5, "4 aired plus the upcoming entry");
        let numbers: Vec<_> = episodes.iter().filter_map(|e| e.number).collect();
        assert_eq!(numbers, vec![1, 2, 3, 4, 5]);
        assert!(episodes[3].airing_at.is_none(), "ep 4 has aired");
        assert_eq!(
            episodes[4].airing_at,
            Some(next.airing_at),
            "ep 5 is the upcoming one"
        );
    }

    #[test]
    fn an_explicit_aired_count_beats_the_schedule() {
        // The catalogue's own count is authoritative when present; the schedule
        // is only a fallback for when it is missing.
        let next = NextAiringEpisode {
            episode: 5,
            airing_at: now_unix() + 86_400,
        };
        let episodes = episodes_for_facts(
            Some(14),
            Some("RELEASING"),
            None,
            &[],
            Some(3),
            Some(next),
            now_unix(),
        );
        // 3 aired, and ep 4 is now skipped as already past... but ep 5 is still
        // upcoming, so 3 aired entries plus the trailing one.
        assert_eq!(episodes.len(), 4);
        assert_eq!(episodes[3].number, Some(5));
    }

    #[test]
    fn a_truncated_long_runner_fills_its_tail() {
        // THE REPORTED CASE: One Piece. Paging stopped at 100 records while the
        // aired count is 1181, so the grid showed 1..100 and then jumped to the
        // next-airing card. The tail must be synthesised, not left as a gap.
        let catalogue: Vec<EpisodeInfo> = (1..=100)
            .map(|number| info(number, Some("2000-01-01")))
            .collect();

        let episodes = episodes_for_facts(
            Some(1181),
            Some("RELEASING"),
            None,
            &catalogue,
            Some(1181),
            None,
            now_unix(),
        );

        assert_eq!(episodes.len(), 1181, "the tail must be filled");
        // The first hundred keep their real catalogue entries.
        assert_eq!(episodes[0].title.as_deref(), Some("Episode 1"));
        // The tail is bare but numbered, so every episode is reachable.
        let numbers: Vec<u32> = episodes.iter().filter_map(|e| e.number).collect();
        assert_eq!(numbers.first(), Some(&1));
        assert_eq!(numbers.last(), Some(&1181));
    }

    #[test]
    fn the_lower_of_the_two_aired_counts_wins() {
        // THE REPORTED CASE: Kitsu's `meta.count` is 1410 because it counts
        // PLANNED records, while the schedule says episode 1182 is next, so
        // 1181 have aired. Trusting the catalogue overshot by 229 episodes --
        // the grid offered episodes that do not exist.
        let catalogue: Vec<EpisodeInfo> = (1..=100)
            .map(|number| info(number, Some("2000-01-01")))
            .collect();
        let next = NextAiringEpisode {
            episode: 1182,
            airing_at: now_unix() + 86_400,
        };

        let episodes = episodes_for_facts(
            Some(1410),
            Some("RELEASING"),
            None,
            &catalogue,
            Some(1410),
            Some(next),
            now_unix(),
        );

        // 1181 aired plus the upcoming entry.
        assert_eq!(episodes.len(), 1182);
        let numbers: Vec<u32> = episodes.iter().filter_map(|e| e.number).collect();
        assert_eq!(numbers.last(), Some(&1182));
    }

    #[test]
    fn a_later_cour_is_never_padded() {
        // 13..24 with a franchise total would invent the previous cour.
        let catalogue: Vec<EpisodeInfo> = (13..=24)
            .map(|number| info(number, Some("2000-01-01")))
            .collect();

        let episodes = episodes_for_facts(
            Some(1181),
            Some("RELEASING"),
            None,
            &catalogue,
            Some(1181),
            None,
            now_unix(),
        );

        assert_eq!(episodes.len(), 12, "a non-1-based cour is left alone");
    }

    #[test]
    fn a_releasing_work_appends_the_next_episode_last() {
        let catalogue = vec![info(1, Some("2000-01-01")), info(2, Some("2000-01-08"))];
        let next = NextAiringEpisode {
            episode: 3,
            airing_at: now_unix() + 86_400,
        };
        let episodes = episodes_for_facts(
            Some(12),
            Some("RELEASING"),
            None,
            &catalogue,
            Some(2),
            Some(next),
            now_unix(),
        );

        assert_eq!(episodes.len(), 3);
        let last = episodes.last().expect("an upcoming entry");
        assert_eq!(last.number, Some(3));
        assert_eq!(last.airing_at, Some(next.airing_at));
        assert!(last.title.is_none(), "no title has been published yet");
    }

    #[test]
    fn a_finished_work_gets_no_upcoming_entry() {
        // A finished work reports no `nextAiringEpisode`, so the field is None
        // and the list is exactly what it was before.
        let episodes = episodes_for(&anime(Some("FINISHED"), Some(3)), &[], now_unix());
        assert_eq!(episodes.len(), 3);
        assert!(episodes.iter().all(|episode| episode.airing_at.is_none()));
    }

    #[test]
    fn a_stale_schedule_is_not_shown_as_upcoming() {
        // The airing already happened but the payload predates it: claiming a
        // future broadcast that is in the past would be a lie.
        //
        // The clock is PINNED here. With the system clock the assertion would
        // pass for the wrong reason, and would start failing the moment the
        // test ran before the timestamp.
        let pinned = 1_700_000_000i64;
        let next = NextAiringEpisode {
            episode: 4,
            airing_at: pinned - 60,
        };
        let episodes = episodes_for_facts(
            Some(12),
            Some("RELEASING"),
            None,
            &[info(1, Some("2000-01-01"))],
            Some(3),
            Some(next),
            pinned,
        );
        assert!(episodes.iter().all(|episode| episode.airing_at.is_none()));
    }

    #[test]
    fn a_future_schedule_is_shown_against_the_pinned_clock() {
        // The mirror of the test above: same pinned clock, timestamp ahead of
        // it. This is what proves the comparison uses the injected value
        // rather than whatever the machine thinks the time is.
        let pinned = 1_700_000_000i64;
        let next = NextAiringEpisode {
            episode: 4,
            airing_at: pinned + 60,
        };
        let episodes = episodes_for_facts(
            Some(12),
            Some("RELEASING"),
            None,
            &[info(1, Some("2000-01-01"))],
            Some(3),
            Some(next),
            pinned,
        );
        assert_eq!(
            episodes.last().and_then(|episode| episode.airing_at),
            Some(pinned + 60)
        );
    }

    #[test]
    fn an_already_listed_episode_is_not_duplicated() {
        // The provider disagreeing with its own schedule must not produce two
        // cards for one number.
        let next = NextAiringEpisode {
            episode: 2,
            airing_at: now_unix() + 86_400,
        };
        let episodes = episodes_for_facts(
            Some(12),
            Some("RELEASING"),
            None,
            &[info(1, Some("2000-01-01")), info(2, Some("2000-01-08"))],
            Some(2),
            Some(next),
            now_unix(),
        );
        assert_eq!(episodes.len(), 2);
    }

    #[test]
    fn a_later_cour_numbering_is_not_padded() {
        // 13..14 with an announced total of 24: lowest != 1, so untouched.
        let catalogue = vec![
            info(13, Some("2000-01-01")),
            info(14, Some("2000-01-01")),
        ];
        let episodes = episodes_for(&anime(Some("FINISHED"), Some(24)), &catalogue, now_unix());
        assert_eq!(episodes.len(), 2);
    }

    #[test]
    fn number_is_read_from_explicit_field_first() {
        let ep = Episode {
            number: Some(7),
            title: Some("Episode 99".into()),
            url: None,
            site: None,
            thumbnail: None,
            aired: None,
            airing_at: None,
            filler: false,
            recap: false,
        };
        assert_eq!(episode_number(&ep), Some(7));
    }

    #[test]
    fn number_is_read_from_the_title() {
        let ep = Episode {
            number: None,
            title: Some("Ep. 12 - The Name".into()),
            url: None,
            site: None,
            thumbnail: None,
            aired: None,
            airing_at: None,
            filler: false,
            recap: false,
        };
        assert_eq!(episode_number(&ep), Some(12));
    }

    #[test]
    fn title_that_announces_a_number_is_recognised() {
        assert!(title_has_episode_number("Episode 7"));
        assert!(title_has_episode_number("E07"));
        assert!(!title_has_episode_number("The Journey's End 2"));
    }

    #[test]
    fn offset_is_zero_for_a_first_cour() {
        let episodes = synthesize_episodes(Some(12), None);
        assert_eq!(absolute_offset(&episodes), 0);
    }

    #[test]
    fn offset_bridges_a_later_cour() {
        let episodes: Vec<Episode> = (13..=24)
            .map(|number| Episode {
                number: Some(number),
                title: None,
                url: None,
                site: None,
                thumbnail: None,
                aired: None,
                airing_at: None,
                filler: false,
                recap: false,
            })
            .collect();
        assert_eq!(absolute_offset(&episodes), 12);
    }

    #[test]
    fn index_of_episode_finds_the_entry() {
        let episodes = synthesize_episodes(Some(5), None);
        assert_eq!(index_of_episode(&episodes, 3), Some(2));
        assert_eq!(index_of_episode(&episodes, 9), None);
    }

    #[test]
    fn has_episode_data_reads_count_and_streaming() {
        assert!(has_episode_data(&anime(None, Some(1))));
        assert!(!has_episode_data(&anime(None, None)));
    }
}