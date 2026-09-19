//! Build the queries an indexer is asked for one search.
//!
//! A title and an episode number are not enough to find a release, for three
//! reasons this module works around.
//!
//! Nyaa's search ANDs whitespace-separated terms, so a query only matches a
//! release containing every term. That makes a colon in a title actively harmful:
//! `Mushoku Tensei: Jobless Reincarnation` sends the term `Tensei:`, colon
//! attached, which no release name contains. The pre-colon segment and the
//! punctuation-stripped spelling are therefore searched too -- in *addition to*
//! the original, never instead of it.
//!
//! Releases name an episode as `S03E09`, not `09`. When a title states a season
//! it is re-expressed that way, and a title with no marker is treated as season
//! 1, because that is what a season-1 entry is called.
//!
//! A release may omit the season entirely (`Show - 09`), so a season-agnostic
//! spelling is searched alongside the explicit one.
//!
//! Everything here is pure and total: no input panics, and the output is always
//! a valid `String`.

use fancy_regex::Regex;
use once_cell::sync::Lazy;

use super::normalize::collapse_whitespace;

/// The most queries one search may send.
///
/// Every query is a separate request, so this is the latency/coverage dial. It
/// is sized for the usual case of two distinct title forms (English and romaji)
/// across four variant spellings and three episode patterns; de-duplication
/// usually brings the real count well below it.
pub const MAX_QUERIES: usize = 12;

/// The shortest pre-colon segment worth searching on its own.
///
/// Guards the degenerate case: `Re:Zero` would otherwise also query `Re`, which
/// matches an enormous amount of unrelated traffic.
const MIN_SHORT_TITLE: usize = 4;

/// A trailing season marker: `Season 3` or `3rd Season`.
///
/// Deliberately narrow. Roman numerals (`Season III`) and `Part 2` are not
/// recognised, because `Gundam III` is a film rather than a third season, and
/// guessing wrong there is worse than not guessing at all.
static SEASON_SUFFIX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\s*[-,:]?\s*(?:season\s*(?<plain>\d{1,2})|(?<ordinal>\d{1,2})\s*(?:st|nd|rd|th)\s+season)\s*$",
    )
    .unwrap()
});

/// Rewrite a title into the spelling a search engine is more likely to match.
///
/// `:`, `,`, `-` and `_` become spaces, and runs of whitespace collapse. Case is
/// preserved, and nothing is deleted: `Re:Zero` becomes `Re Zero` so a release
/// named with a space can be found, while the original is still searched so a
/// release named with the colon can be found too.
pub fn sanitize_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());

    for ch in title.chars() {
        match ch {
            ':' | ',' | '-' | '_' => out.push(' '),
            // The dash-like glyphs a title may carry instead of an ASCII hyphen.
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => out.push(' '),
            other => out.push(other),
        }
    }

    collapse_whitespace(&out)
}

/// Split a trailing `Season N` off a title, returning the base and the number.
///
/// `Mushoku Tensei: Jobless Reincarnation Season 3` yields the base and `3`, so
/// the query can say `S03E09` instead of `Season 3 09`.
///
/// A title that is *only* a season marker (`Season 3`) keeps its original text
/// and reports no season: stripping it would leave an empty base, and a query of
/// bare `S03E09` matches nothing useful.
pub fn split_season(title: &str) -> (String, Option<u32>) {
    let trimmed = title.trim();

    let Some(captures) = SEASON_SUFFIX.captures(trimmed).ok().flatten() else {
        return (trimmed.to_string(), None);
    };

    let number = captures
        .name("plain")
        .or_else(|| captures.name("ordinal"))
        .and_then(|m| m.as_str().parse::<u32>().ok());

    let Some(number) = number else {
        return (trimmed.to_string(), None);
    };

    let base = collapse_whitespace(trimmed[..captures.get(0).unwrap().start()].trim());
    if base.is_empty() {
        return (trimmed.to_string(), None);
    }

    (base, Some(number))
}

/// The part of a title before its first colon, when it is worth searching.
///
/// Releases often drop a subtitle: `Mushoku Tensei: Jobless Reincarnation` is
/// uploaded as `Mushoku Tensei`. Searching the short form as well is what finds
/// those, since Nyaa would otherwise require the terms `Jobless` and
/// `Reincarnation` to appear in the release name.
///
/// `None` when there is no colon, when the segment is shorter than
/// [`MIN_SHORT_TITLE`], or when it is the whole title.
pub fn short_title(title: &str) -> Option<String> {
    let (head, _) = title.split_once(':')?;
    let candidate = sanitize_title(head);

    if candidate.chars().count() < MIN_SHORT_TITLE || candidate == title.trim() {
        return None;
    }

    Some(candidate)
}

/// One spelling of a title, and whether it still carries its season marker.
///
/// The flag matters because a variant that already says `Season 3` must not have
/// `S03E09` appended to it: that would ask for a release name carrying both, and
/// no release is named that way.
struct Variant {
    text: String,
    states_season: bool,
}

/// The spellings of one title, most useful first.
fn variants_of(title: &str) -> Vec<Variant> {
    let trimmed = title.trim();
    let (base, _) = split_season(trimmed);
    let sanitized = sanitize_title(&base);

    let mut variants: Vec<Variant> = Vec::new();

    // The sanitized base leads, because a colon is the single most damaging
    // character in a Nyaa query: it ANDs terms, and `Tensei:` appears in no
    // release name.
    if !sanitized.is_empty() {
        variants.push(Variant {
            text: sanitized.clone(),
            states_season: false,
        });
    }

    if let Some(short) = short_title(&base) {
        variants.push(Variant {
            text: short,
            states_season: false,
        });
    }

    // The base with its punctuation intact, for a release that kept the colon.
    let raw_base = base.trim();
    if !raw_base.is_empty() && raw_base != sanitized {
        variants.push(Variant {
            text: raw_base.to_string(),
            states_season: false,
        });
    }

    // The untouched title, kept so a release literally named with its season
    // marker is still reachable.
    variants.push(Variant {
        text: trimmed.to_string(),
        states_season: split_season(trimmed).1.is_some(),
    });

    variants
}

/// How a title variant is combined with a season and episode number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pattern {
    /// `Title S03E09` -- how a release with a season marker is named.
    SeasonEpisode,
    /// `Title 09` -- a release that omits the season.
    PaddedEpisode,
    /// `Title 9` -- the unpadded spelling.
    BareEpisode,
    /// `Title S03` -- a title-level search scoped to one season.
    SeasonOnly,
    /// `Title` -- a film or a whole-work search.
    TitleOnly,
}

/// Combine one variant with one pattern, or `None` when they do not belong.
fn apply(
    pattern: Pattern,
    variant: &Variant,
    season: Option<u32>,
    episode: Option<u32>,
) -> Option<String> {
    // A variant that already states its own season must not be given a second.
    if variant.states_season
        && matches!(pattern, Pattern::SeasonEpisode | Pattern::SeasonOnly)
    {
        return None;
    }

    let text = variant.text.trim();
    if text.is_empty() {
        return None;
    }

    let query = match pattern {
        Pattern::SeasonEpisode => {
            let (season, episode) = (season?, episode?);
            format!("{text} S{season:02}E{episode:02}")
        }
        Pattern::PaddedEpisode => {
            let episode = episode?;
            format!("{text} {episode:02}")
        }
        Pattern::BareEpisode => {
            let episode = episode?;
            format!("{text} {episode}")
        }
        Pattern::SeasonOnly => {
            let season = season?;
            format!("{text} S{season:02}")
        }
        Pattern::TitleOnly => text.to_string(),
    };

    Some(collapse_whitespace(&query))
}

/// The patterns to try, strongest first, for a given request.
fn patterns_for(season: Option<u32>, episode: Option<u32>) -> Vec<Pattern> {
    match (season, episode) {
        (Some(_), Some(_)) => vec![
            Pattern::SeasonEpisode,
            Pattern::PaddedEpisode,
            Pattern::BareEpisode,
        ],
        (None, Some(_)) => vec![Pattern::PaddedEpisode, Pattern::BareEpisode],
        (Some(_), None) => vec![Pattern::SeasonOnly],
        (None, None) => vec![Pattern::TitleOnly],
    }
}

/// Build the query strings for a search, most promising first.
///
/// `titles` are the work's title forms, best first. `episode` is the episode to
/// find, or `None` for a film or a whole-work search. The result is capped at
/// `max` and holds no case-insensitive duplicates.
///
/// The ordering is pattern-major: every title spelling gets its strongest
/// pattern before any spelling gets a weaker one. That way a truncated list
/// still contains the `S03E09` spelling of every form, which is the one most
/// releases use.
///
/// A season-less title is searched as season 1 when an episode is selected. That
/// is what a season-1 entry is called, and without it `... 01` finds nothing --
/// Nyaa's results are all named `S01E01`.
pub fn build_queries(titles: &[String], episode: Option<u32>, max: usize) -> Vec<String> {
    let forms: Vec<&String> = titles.iter().filter(|t| !t.trim().is_empty()).collect();
    if forms.is_empty() || max == 0 {
        return Vec::new();
    }

    let stated = forms.first().and_then(|t| split_season(t).1);
    // Only default a season once an episode is in play: a film search must not
    // become `S01`.
    let season = match (episode, stated) {
        (Some(_), None) => Some(1),
        (_, season) => season,
    };

    let mut queries: Vec<String> = Vec::new();

    for pattern in patterns_for(season, episode) {
        for form in &forms {
            for variant in variants_of(form) {
                let Some(candidate) = apply(pattern, &variant, season, episode) else {
                    continue;
                };
                push_unique(&mut queries, candidate);
            }
        }
    }

    queries.truncate(max);
    queries
}

/// Add `candidate` unless an equal spelling is already present.
///
/// Compared case-insensitively: Nyaa does not distinguish case, so `show S01E01`
/// and `Show S01E01` would return the same set and only cost an extra request.
fn push_unique(queries: &mut Vec<String>, candidate: String) {
    let key = candidate.to_lowercase();
    if !queries.iter().any(|existing| existing.to_lowercase() == key) {
        queries.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(forms: &[&str]) -> Vec<String> {
        forms.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sanitize_replaces_the_harmful_characters() {
        assert_eq!(sanitize_title("Re:Zero"), "Re Zero");
        assert_eq!(sanitize_title("K-On!"), "K On!");
        assert_eq!(sanitize_title("Show_Name"), "Show Name");
        assert_eq!(sanitize_title("Show - Sub"), "Show Sub");
        assert_eq!(sanitize_title("Show, Sub"), "Show Sub");
    }

    #[test]
    fn sanitize_keeps_case_and_other_punctuation() {
        assert_eq!(sanitize_title("Fate/stay night"), "Fate/stay night");
        assert_eq!(sanitize_title("Steins;Gate"), "Steins;Gate");
    }

    #[test]
    fn sanitize_is_idempotent() {
        let once = sanitize_title("Re:Zero - Starting Life");
        assert_eq!(sanitize_title(&once), once);
    }

    #[test]
    fn sanitize_of_a_blank_title_is_empty() {
        assert_eq!(sanitize_title("   "), "");
        assert_eq!(sanitize_title("---"), "");
    }

    #[test]
    fn split_season_reads_a_plain_marker() {
        let (base, season) = split_season("Mushoku Tensei: Jobless Reincarnation Season 3");
        assert_eq!(base, "Mushoku Tensei: Jobless Reincarnation");
        assert_eq!(season, Some(3));
    }

    #[test]
    fn split_season_reads_an_ordinal_marker() {
        let (base, season) = split_season("Jujutsu Kaisen 2nd Season");
        assert_eq!(base, "Jujutsu Kaisen");
        assert_eq!(season, Some(2));
    }

    #[test]
    fn split_season_leaves_a_title_without_one_alone() {
        assert_eq!(
            split_season("Attack on Titan"),
            ("Attack on Titan".to_string(), None)
        );
    }

    #[test]
    fn split_season_keeps_a_bare_marker_title_intact() {
        // Nothing would be left to search, so the title is returned untouched.
        assert_eq!(split_season("Season 3"), ("Season 3".to_string(), None));
    }

    #[test]
    fn split_season_ignores_a_marker_that_is_not_trailing() {
        // Out of scope by decision: only a trailing marker is recognised.
        assert_eq!(
            split_season("Show Season 3 Part 2").1,
            None
        );
    }

    #[test]
    fn short_title_reads_the_segment_before_the_colon() {
        assert_eq!(
            short_title("Mushoku Tensei: Jobless Reincarnation").as_deref(),
            Some("Mushoku Tensei")
        );
    }

    #[test]
    fn short_title_is_none_without_a_colon() {
        assert_eq!(short_title("Attack on Titan"), None);
    }

    #[test]
    fn short_title_rejects_a_too_short_segment() {
        // `Re:Zero` must not also query `Re`.
        assert_eq!(short_title("Re:Zero"), None);
    }

    #[test]
    fn the_first_query_is_the_season_episode_spelling() {
        let built = build_queries(
            &titles(&["Mushoku Tensei: Jobless Reincarnation Season 3"]),
            Some(9),
            MAX_QUERIES,
        );

        assert_eq!(
            built.first().map(String::as_str),
            Some("Mushoku Tensei Jobless Reincarnation S03E09"),
            "got {built:?}"
        );
    }

    #[test]
    fn a_season_less_title_is_searched_as_season_one() {
        let built = build_queries(&titles(&["Mushoku Tensei"]), Some(1), MAX_QUERIES);
        assert!(
            built.contains(&"Mushoku Tensei S01E01".to_string()),
            "got {built:?}"
        );
    }

    #[test]
    fn a_season_agnostic_spelling_is_also_searched() {
        let built = build_queries(&titles(&["Attack on Titan"]), Some(5), MAX_QUERIES);
        assert!(built.contains(&"Attack on Titan 05".to_string()), "got {built:?}");
    }

    #[test]
    fn the_original_title_is_searched_with_its_season_marker() {
        let built = build_queries(
            &titles(&["Mushoku Tensei: Jobless Reincarnation Season 3"]),
            Some(9),
            MAX_QUERIES,
        );
        // The un-stripped form keeps `Season 3` and only takes the episode, so
        // it can match an upload named that way.
        assert!(
            built.contains(&"Mushoku Tensei: Jobless Reincarnation Season 3 09".to_string()),
            "got {built:?}"
        );
    }

    #[test]
    fn a_title_stating_a_season_is_never_given_a_second_one() {
        let built = build_queries(
            &titles(&["Mushoku Tensei: Jobless Reincarnation Season 3"]),
            Some(9),
            MAX_QUERIES,
        );
        assert!(
            !built.iter().any(|q| q.contains("Season 3 S03E09")),
            "got {built:?}"
        );
    }

    #[test]
    fn a_film_search_never_forces_a_season() {
        let built = build_queries(&titles(&["Some Movie"]), None, MAX_QUERIES);
        assert_eq!(built, vec!["Some Movie".to_string()]);
    }

    #[test]
    fn a_title_level_search_scopes_to_the_stated_season() {
        let built = build_queries(
            &titles(&["Show Season 3"]),
            None,
            MAX_QUERIES,
        );
        assert!(built.contains(&"Show S03".to_string()), "got {built:?}");
    }

    #[test]
    fn every_title_form_is_searched() {
        let built = build_queries(
            &titles(&[
                "Attack on Titan",
                "Shingeki no Kyojin",
            ]),
            Some(5),
            MAX_QUERIES,
        );

        assert!(built.contains(&"Attack on Titan S01E05".to_string()));
        assert!(built.contains(&"Shingeki no Kyojin S01E05".to_string()));
    }

    #[test]
    fn the_cap_is_respected() {
        let built = build_queries(
            &titles(&[
                "Mushoku Tensei: Jobless Reincarnation Season 3",
                "Mushoku Tensei II: Isekai Ittara Honki Dasu",
                "Mushoku Tensei III: Isekai Ittara Honki Dasu",
            ]),
            Some(9),
            4,
        );
        assert_eq!(built.len(), 4);
    }

    #[test]
    fn duplicates_are_collapsed_case_insensitively() {
        let built = build_queries(
            &titles(&["Show", "show", "SHOW"]),
            Some(1),
            MAX_QUERIES,
        );

        // The three forms are the same work spelled three ways, so they collapse
        // to one spelling per pattern -- three queries, not nine.
        assert_eq!(
            built,
            vec![
                "Show S01E01".to_string(),
                "Show 01".to_string(),
                "Show 1".to_string(),
            ]
        );
    }

    #[test]
    fn the_sanitized_and_original_spellings_both_survive() {
        let built = build_queries(&titles(&["Re:Zero"]), Some(1), MAX_QUERIES);
        assert!(built.contains(&"Re:Zero S01E01".to_string()), "got {built:?}");
        assert!(built.contains(&"Re Zero S01E01".to_string()), "got {built:?}");
    }

    #[test]
    fn an_empty_title_list_yields_no_queries() {
        assert!(build_queries(&[], Some(1), MAX_QUERIES).is_empty());
        assert!(build_queries(&titles(&["  "]), Some(1), MAX_QUERIES).is_empty());
    }

    #[test]
    fn a_zero_cap_yields_no_queries() {
        assert!(build_queries(&titles(&["Show"]), Some(1), 0).is_empty());
    }
}