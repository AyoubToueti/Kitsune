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

use crate::types::{Anime, EpisodeInfo};

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
            filler: false,
            recap: false,
        })
        .collect()
}

/// Turn Jikan's per-episode catalogue into display entries.
///
/// Every entry keeps its explicit `number`, so the torrent matcher never has to
/// recover it from the title. `cover_image` is used as the card background
/// because Jikan has no per-episode stills; the episode's own title is the real
/// one, and the fallback only fires for a genuinely untitled entry.
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
            thumbnail: cover_image.map(str::to_string),
            url: None,
            site: None,
            aired: entry.aired.clone(),
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
    let Some(aired) = aired else {
        return true;
    };
    if aired.trim().is_empty() {
        return true;
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
    info.iter()
        .filter(|entry| is_aired(entry.aired.as_deref(), now_unix))
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
    if !info.is_empty() {
        let known = episodes_from_info(
            &aired_episodes(info, now_unix),
            anime.cover_image.as_deref(),
        );

        // Only a finished work is padded. Every episode of a finished work has
        // aired by definition, so a missing entry is one the provider has not
        // titled yet. On a releasing work the same gap could equally be
        // episodes that have not aired, and adding them would present
        // unwatchable episodes as ready.
        return if anime.status.as_deref() == Some("FINISHED") {
            with_missing_episodes(known, anime.episode_count, anime.cover_image.as_deref())
        } else {
            known
        };
    }
    synthesize_episodes(anime.episode_count, anime.cover_image.as_deref())
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
    fn empty_catalogue_falls_back_to_synthesis() {
        let episodes = episodes_for(&anime(Some("RELEASING"), Some(4)), &[], now_unix());
        assert_eq!(episodes.len(), 4);
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