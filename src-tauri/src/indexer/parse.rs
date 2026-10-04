//! Extract the title, episode numbers and subgroup from a release name.
//!
//! Ported from Sonarr's `Parser`/`AnimeParser`, reduced to the shapes anime
//! indexers emit. The hard part is that anime releases name an *absolute*
//! episode (`Show - 37`) while western-style releases name season and episode
//! (`Show S04E01`), and some groups give both (`Show S04E01 - 60`). All three
//! are captured, and the matcher decides which to trust.
//!
//! The patterns run against a name already passed through
//! [`super::normalize::normalize`], so separators are a single ASCII hyphen and
//! every run of spaces is one space. Pure and total.

use fancy_regex::Regex;
use once_cell::sync::Lazy;

use super::normalize::{leading_subgroup, normalize};
use crate::types::ParsedRelease;

/// Patterns that tie a season and an episode together, optionally followed by
/// an absolute episode after a dash (`S04E01 - 60`).
///
/// Tried before the bare-episode patterns, because `S04E01` also contains a
/// number that the loose patterns would otherwise grab as an absolute episode.
static SEASON_EPISODE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // S01E02, s01e02, S01.E02, and the trailing " - 60" variant.
        Regex::new(
            r"(?i)\bs(?<season>\d{1,2})[\s._-]*e(?<episode>\d{1,3})(?:[\s._-]*-[\s._-]*(?<absolute>\d{1,4}))?",
        )
        .unwrap(),
        // 1x02
        Regex::new(r"(?i)(?<season>\d{1,2})x(?<episode>\d{1,3})").unwrap(),
    ]
});

/// Patterns that name a single episode without a season.
///
/// Ordered most explicit first: an explicit "Episode 5" beats a bare trailing
/// number, which in turn beats the anime ` - 05` convention.
static ABSOLUTE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // "Episode 5", "Ep. 5", "E05"
        Regex::new(r"(?i)\b(?:episode|ep|e)[\s._-]*(?<episode>\d{1,4})(?:v\d)?").unwrap(),
        // Anime convention: "Show - 05", optionally " - 05v2".
        Regex::new(r"[\s._-]-[\s._-]*(?<episode>\d{1,4})(?:v\d)?(?![\d])").unwrap(),
        // Bare trailing number: "Show 05", optionally followed by trailing
        // bracket tags ("Show 12 [1080p]"). The tags are allowed because a
        // release almost always ends with a quality tag, and requiring the
        // number to be the very last thing would miss the common case.
        Regex::new(r"(?<episode>\d{1,4})(?:v\d)?(?:\s*\[[^\]]*\])*\s*$").unwrap(),
    ]
});

/// Multi-episode packs, e.g. "01-12" or "Batch".
///
/// These are recognised so the matcher can reject them for a single-episode
/// request: a season pack is not what "episode 3" asked for.
static RANGE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // The first number may carry an `S`/`E` prefix (`S01-12`, `E01-02`), which
    // a plain `\b\d` would miss: `S` and `0` are both word characters, so
    // there is no boundary between them.
    Regex::new(r"(?i)\b[se]?(?:\d{1,4}[\s._-]*[-~][\s._-]*\d{1,4}|\d{1,4}\s*~\s*\d{1,4})\b")
        .unwrap()
});

/// Words that mark a pack outright, whatever else the name says.
///
/// A name carrying one of these is a pack even if it also states an episode
/// number: `Show Batch 05` is still a batch, not episode 5.
static UNCONDITIONAL_PACK_WORDS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:batch|complete|\[\d{1,4}[-~]\d{1,4}\])\b").unwrap());

/// The `Season N` marker, which is weaker evidence than the words above.
///
/// A season pack is usually named `Show Season 3` with no episode, but a group
/// may equally write `Show Season 3 - 09` for a single episode of that season.
/// So this only counts as a pack when the name states no episode of its own --
/// see [`is_pack`].
static SEASON_PACK_WORDS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bseason\s*\d+\b").unwrap());

/// A bare `S01`/`S1` season marker with no episode or range after it.
///
/// Groups name a whole season `Show S01 1080p`, with no `E##` for the
/// season/episode patterns to latch onto. The negative lookaheads keep this
/// from firing on a single episode (`S01E05`) or a range (`S01-12`), both of
/// which state their own episode and are handled elsewhere.
static BARE_SEASON_MARKER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?<![a-z0-9])s\d{1,2}(?![\s._-]*e\d)(?![\s._-]*[-~]\s*\d)(?!\d)").unwrap()
});

/// Parse a release name into its title, numbers and subgroup.
///
/// `name` is normalised internally, so callers pass the raw indexer title and
/// do not need to pre-normalise.
pub fn parse_release(name: &str) -> ParsedRelease {
    let normalized = normalize(name);
    let subgroup = leading_subgroup(&normalized).map(str::to_string);

    let (season, episode, absolute, title_end) = match_season_episode(&normalized)
        .or_else(|| match_absolute(&normalized))
        .unwrap_or((None, None, None, normalized.len()));

    let title = extract_title(&normalized, title_end);

    ParsedRelease {
        title,
        season,
        episode,
        absolute_episode: absolute,
        subgroup,
    }
}

/// Try the season+episode patterns. Returns numbers and where the title ends.
fn match_season_episode(name: &str) -> Option<(Option<u32>, Option<u32>, Option<u32>, usize)> {
    for pattern in SEASON_EPISODE_PATTERNS.iter() {
        let Ok(Some(caps)) = pattern.captures(name) else {
            continue;
        };

        let season = caps.name("season").and_then(|m| m.as_str().parse().ok());
        let episode = caps.name("episode").and_then(|m| m.as_str().parse().ok());
        let absolute = caps.name("absolute").and_then(|m| m.as_str().parse().ok());
        let end = caps.get(0).map(|m| m.start()).unwrap_or(name.len());

        return Some((season, episode, absolute, end));
    }
    None
}

/// Try the single-episode patterns. Returns numbers and where the title ends.
fn match_absolute(name: &str) -> Option<(Option<u32>, Option<u32>, Option<u32>, usize)> {
    for pattern in ABSOLUTE_PATTERNS.iter() {
        let Ok(Some(caps)) = pattern.captures(name) else {
            continue;
        };

        let Some(episode_match) = caps.name("episode") else {
            continue;
        };
        let episode: u32 = match episode_match.as_str().parse() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let end = caps.get(0).map(|m| m.start()).unwrap_or(name.len());

        // A bare trailing number is the weakest signal: only accept it when
        // what precedes looks like a title rather than a resolution tag, so
        // "Show 1080p" does not read as episode 1080.
        if pattern.as_str().contains(r"\s*$") && looks_like_quality_only(&name[..end]) {
            continue;
        }

        return Some((None, Some(episode), Some(episode), end));
    }
    None
}

/// Whether the text before a candidate number is only tags, not a title.
///
/// Guards the bare-number fallback: "[G] 1080p" is a truncated name, not an
/// episode.
fn looks_like_quality_only(prefix: &str) -> bool {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return true;
    }
    // Only brackets, punctuation and known tags; no letters forming a word.
    !trimmed.chars().any(|c| c.is_alphabetic())
}

/// The title portion of a normalised name, up to `end`.
///
/// A leading `[Group]` bracket is dropped, since it names the encoder rather
/// than the work. Returns `None` when nothing usable is left, which is honest:
/// a name that is only a group tag has no title.
fn extract_title(name: &str, end: usize) -> Option<String> {
    let head = name[..end.min(name.len())].trim();

    // Drop a leading bracket tag: "[Group] Show" -> "Show".
    let without_group = if let Some(rest) = head.strip_prefix('[') {
        match rest.find(']') {
            Some(close) => rest[close + 1..].trim(),
            None => head,
        }
    } else {
        head
    };

    // Trailing hyphens are left behind by "Show - " once the number is cut.
    let cleaned = without_group.trim_end_matches(['-', ' ']).trim();

    (!cleaned.is_empty()).then(|| cleaned.to_string())
}

/// Whether a release name looks like a multi-episode or whole-season pack.
///
/// Used by the matcher to reject packs for a single-episode request. A name
/// that says "Batch" or spans "01-12" is not the one episode asked for.
/// A `Season N` marker is weaker evidence than a batch word, because a group
/// may write `Show Season 3 - 09` for a single episode.
///
/// The marker has to be removed before *any* other check, for two reasons. The
/// bare-trailing-number pattern reads its own `3` as an episode, and the range
/// pattern reads `3 - 09` as the span "3-09". Either would misclassify
/// `Show Season 3 - 09` as a pack. Once the marker is gone, whatever remains is
/// the name's real evidence about whether it is one episode.
pub fn is_pack(name: &str) -> bool {
    let normalized = normalize(name);

    if is_match(&UNCONDITIONAL_PACK_WORDS, &normalized) {
        return true;
    }

    if is_match(&SEASON_PACK_WORDS, &normalized) {
        let without_season = SEASON_PACK_WORDS.replace_all(&normalized, " ");
        return is_match(&RANGE_PATTERN, &without_season)
            || !states_an_episode(&without_season);
    }

    // A bare `S01` with no episode of its own is a season pack. The pattern
    // already excludes `S01E05` and `S01-12`, so a match here means the name
    // carries a season and nothing more.
    if is_match(&BARE_SEASON_MARKER, &normalized) {
        return true;
    }

    is_match(&RANGE_PATTERN, &normalized)
}

/// Whether a normalised name states an episode number of its own.
///
/// Used only by [`is_pack`], on a name that has already had its `Season N`
/// marker removed. Both the season/episode patterns and the single-episode
/// patterns count, since `S03E09` and ` - 09` are equally good evidence that
/// the release is one episode rather than a pack.
fn states_an_episode(name: &str) -> bool {
    match_season_episode(name).is_some() || match_absolute(name).is_some()
}

fn is_match(pattern: &Regex, input: &str) -> bool {
    pattern.is_match(input).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_anime_dash_convention() {
        let parsed = parse_release("[SubsPlease] Show - 05 (1080p)");
        assert_eq!(parsed.title.as_deref(), Some("Show"));
        assert_eq!(parsed.absolute_episode, Some(5));
        assert_eq!(parsed.episode, Some(5));
        assert_eq!(parsed.season, None);
        assert_eq!(parsed.subgroup.as_deref(), Some("SubsPlease"));
    }

    #[test]
    fn parses_a_season_episode_name() {
        let parsed = parse_release("[Group] Show S04E01 [1080p]");
        assert_eq!(parsed.season, Some(4));
        assert_eq!(parsed.episode, Some(1));
        assert_eq!(parsed.absolute_episode, None);
    }

    #[test]
    fn parses_season_episode_with_an_absolute_tail() {
        // "S04E01 - 60": both meanings are captured, and the matcher picks.
        let parsed = parse_release("[Group] Show S04E01 - 60 [1080p]");
        assert_eq!(parsed.season, Some(4));
        assert_eq!(parsed.episode, Some(1));
        assert_eq!(parsed.absolute_episode, Some(60));
    }

    #[test]
    fn parses_a_bare_trailing_number() {
        let parsed = parse_release("[Group] Show 12 [1080p]");
        assert_eq!(parsed.absolute_episode, Some(12));
        assert_eq!(parsed.title.as_deref(), Some("Show"));
    }

    #[test]
    fn parses_the_word_episode() {
        let parsed = parse_release("Show Episode 7 [720p]");
        assert_eq!(parsed.absolute_episode, Some(7));
    }

    #[test]
    fn parses_a_v2_revision() {
        let parsed = parse_release("[Group] Show - 05v2 (1080p)");
        assert_eq!(parsed.absolute_episode, Some(5));
    }

    #[test]
    fn title_drops_the_group_tag() {
        let parsed = parse_release("[Erai-raws] Great Show - 01 [1080p]");
        assert_eq!(parsed.title.as_deref(), Some("Great Show"));
        assert_eq!(parsed.subgroup.as_deref(), Some("Erai-raws"));
    }

    #[test]
    fn handles_a_name_without_a_group_tag() {
        let parsed = parse_release("Great Show - 01 [1080p]");
        assert_eq!(parsed.title.as_deref(), Some("Great Show"));
        assert_eq!(parsed.subgroup, None);
    }

    #[test]
    fn does_not_treat_a_resolution_as_an_episode() {
        // The bare-number fallback must not read "1080p" as episode 1080.
        let parsed = parse_release("[Group] Show [1080p]");
        assert_ne!(parsed.absolute_episode, Some(1080));
    }

    #[test]
    fn detects_a_multi_episode_range() {
        assert!(is_pack("[Group] Show 01-12 [1080p]"));
        assert!(is_pack("[Group] Show 01~12 [1080p]"));
    }

    #[test]
    fn detects_a_named_batch() {
        assert!(is_pack("[Group] Show Batch [1080p]"));
        assert!(is_pack("[Group] Show Complete [1080p]"));
    }

    #[test]
    fn a_single_episode_is_not_a_pack() {
        assert!(!is_pack("[Group] Show - 05 [1080p]"));
    }

    #[test]
    fn a_bare_season_marker_is_a_pack() {
        // `Show Season 3` states a season but no episode, so it is the pack.
        assert!(is_pack("[Group] Show Season 3 [1080p]"));
    }

    #[test]
    fn a_bare_sxx_marker_is_a_pack() {
        // The naming the real GTO upload uses: a season and nothing else.
        assert!(is_pack("GTO Great Teacher Onizuka S01 1080p NF WEB-DL -VARYG"));
        assert!(is_pack("[Group] Show S02 [1080p]"));
        assert!(is_pack("[Group] Show S1 [720p]"));
    }

    #[test]
    fn a_bare_sxx_with_an_episode_is_not_a_pack() {
        assert!(!is_pack("[Group] Show S01E05 [1080p]"));
    }

    #[test]
    fn a_bare_sxx_range_is_still_a_pack() {
        // `S01-12` is a range, which the range rule already recognises.
        assert!(is_pack("[Group] Show S01-12 [1080p]"));
    }

    #[test]
    fn a_season_marker_with_an_episode_is_not_a_pack() {
        // The case the old rule got wrong: episode 9 of season 3 is not a pack.
        assert!(!is_pack("[Group] Show Season 3 - 09 [1080p]"));
        assert!(!is_pack("[Group] Show Season 3 Episode 09 [1080p]"));
        assert!(!is_pack("[Group] Show Season 3 S03E09 [1080p]"));
    }

    #[test]
    fn a_season_marker_with_a_range_is_still_a_pack() {
        assert!(is_pack("[Group] Show Season 3 [01-12] [1080p]"));
        assert!(is_pack("[Group] Show Season 3 01-12 [1080p]"));
    }

    #[test]
    fn a_season_marker_with_a_batch_word_is_still_a_pack() {
        assert!(is_pack("[Group] Show Season 3 Batch [1080p]"));
        assert!(is_pack("[Group] Show Season 3 Complete [1080p]"));
    }

    #[test]
    fn the_season_marker_does_not_count_as_its_own_episode() {
        // `parse_release` alone reads the `3` in `Season 3` as episode 3, because
        // the bare-trailing-number pattern cannot tell a season from an episode.
        // That is exactly why `is_pack` strips the marker before deciding: the
        // pack verdict, not the parse, is what stops this being taken as episode 3.
        assert_eq!(parse_release("[Group] Show Season 3 [1080p]").episode, Some(3));
        assert!(is_pack("[Group] Show Season 3 [1080p]"));
    }

    #[test]
    fn parses_a_name_that_only_has_a_group() {
        // Nothing usable is left after the group tag is dropped.
        let parsed = parse_release("[Group]");
        assert_eq!(parsed.title, None);
        assert_eq!(parsed.subgroup.as_deref(), Some("Group"));
    }

    #[test]
    fn underscore_separated_numbers_parse() {
        // normalise turns "Show_08" into "Show 08" before parsing.
        let parsed = parse_release("[Group] Show_08_[1080p]");
        assert_eq!(parsed.absolute_episode, Some(8));
    }
}