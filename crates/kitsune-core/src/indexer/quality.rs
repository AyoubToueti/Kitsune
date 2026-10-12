//! Detect a release's resolution, source, codec and remux flag from its name.
//!
//! Ported from Sonarr's `QualityParser`, reduced to the signals anime releases
//! actually carry. The regexes use `fancy-regex` because Sonarr's originals
//! rely on lookaround and backreferences, which the `regex` crate cannot
//! express.
//!
//! Everything here is pure: parsing a name never touches the network and never
//! panics. A name that matches nothing yields a defaulted [`Quality`] rather
//! than an error, because "unknown quality" is a legitimate answer.

use fancy_regex::Regex;
use once_cell::sync::Lazy;

use crate::types::{ReleaseSource, Resolution};

/// The quality signals a release name carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Quality {
    pub resolution: Resolution,
    pub source: ReleaseSource,
    /// True when the name says `REMUX`, which means a lossless copy of a disc.
    pub remux: bool,
    /// True when the name says `PROPER` or `REPACK`, a corrected re-release.
    pub revision: bool,
}

/// Resolution patterns, most specific first.
///
/// Ordered so `2160p` is tested before `1080p`: the shorter pattern would
/// otherwise match inside the longer one only if the longer were tried first,
/// and these are anchored on word boundaries to begin with.
static RESOLUTION_PATTERNS: Lazy<Vec<(Regex, Resolution)>> = Lazy::new(|| {
    vec![
        (
            Regex::new(r"(?i)(?<![0-9])(?:2160p|4k)(?![0-9])").unwrap(),
            Resolution::R2160p,
        ),
        (
            Regex::new(r"(?i)(?<![0-9])1080p(?![0-9])").unwrap(),
            Resolution::R1080p,
        ),
        (
            Regex::new(r"(?i)(?<![0-9])720p(?![0-9])").unwrap(),
            Resolution::R720p,
        ),
        (
            Regex::new(r"(?i)(?<![0-9])576p(?![0-9])").unwrap(),
            Resolution::R576p,
        ),
        (
            Regex::new(r"(?i)(?<![0-9])540p(?![0-9])").unwrap(),
            Resolution::R540p,
        ),
        (
            Regex::new(r"(?i)(?<![0-9])480p(?![0-9])").unwrap(),
            Resolution::R480p,
        ),
        (
            Regex::new(r"(?i)(?<![0-9])360p(?![0-9])").unwrap(),
            Resolution::R360p,
        ),
    ]
});

/// Source patterns, best-known source first.
static SOURCE_PATTERNS: Lazy<Vec<(Regex, ReleaseSource)>> = Lazy::new(|| {
    vec![
        (Regex::new(r"(?i)\bremux\b").unwrap(), ReleaseSource::BluRay),
        (Regex::new(r"(?i)\bblu-?ray\b").unwrap(), ReleaseSource::BluRay),
        (Regex::new(r"(?i)\bbd-?rip\b").unwrap(), ReleaseSource::BdRip),
        (Regex::new(r"(?i)\bbr-?rip\b").unwrap(), ReleaseSource::BrRip),
        (
            Regex::new(r"(?i)\bweb-?dl\b|\bwebdl\b").unwrap(),
            ReleaseSource::WebDl,
        ),
        (
            Regex::new(r"(?i)\bweb-?rip\b|\bwebrip\b").unwrap(),
            ReleaseSource::WebRip,
        ),
        (Regex::new(r"(?i)\bhdtv\b").unwrap(), ReleaseSource::Hdtv),
        (Regex::new(r"(?i)\bdvd-?rip\b|\bdvd\b").unwrap(), ReleaseSource::Dvd),
        (Regex::new(r"(?i)\bpdtv\b").unwrap(), ReleaseSource::Pdtv),
        (Regex::new(r"(?i)\bdsr\b").unwrap(), ReleaseSource::Dsr),
        (Regex::new(r"(?i)\btv-?rip\b").unwrap(), ReleaseSource::TvRip),
        (Regex::new(r"(?i)\bsdtv\b").unwrap(), ReleaseSource::Sdtv),
    ]
});

/// `REMUX` marks a lossless copy.
static REMUX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\bremux\b").unwrap());

/// `PROPER` and `REPACK` mark a corrected re-release.
static REVISION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:proper|repack)\b").unwrap());

/// Detect the quality signals in a release name.
///
/// Resolution is taken from the first matching pattern, so a name claiming
/// `2160p` in its title but `1080p` in its tags still reads as `2160p`; the
/// patterns are ordered most-specific first rather than by position.
pub fn parse_quality(name: &str) -> Quality {
    Quality {
        resolution: detect_resolution(name),
        source: detect_source(name),
        remux: is_match(&REMUX, name),
        revision: is_match(&REVISION, name),
    }
}

/// The resolution a name states, or [`Resolution::Unknown`].
pub fn detect_resolution(name: &str) -> Resolution {
    for (pattern, resolution) in RESOLUTION_PATTERNS.iter() {
        if is_match(pattern, name) {
            return *resolution;
        }
    }
    Resolution::Unknown
}

/// The source a name states, or [`ReleaseSource::Unknown`].
pub fn detect_source(name: &str) -> ReleaseSource {
    for (pattern, source) in SOURCE_PATTERNS.iter() {
        if is_match(pattern, name) {
            return *source;
        }
    }
    ReleaseSource::Unknown
}

/// Whether a regex matches, treating a regex error as "no match".
///
/// `fancy-regex` returns `Result<bool>` because a pattern with catastrophic
/// backtracking can exceed its budget. A failure is not a reason to abort a
/// search, so it is treated as a miss; the patterns here are all anchored and
/// short enough that this should never fire.
fn is_match(pattern: &Regex, input: &str) -> bool {
    pattern.is_match(input).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_resolutions() {
        assert_eq!(detect_resolution("[G] Show - 01 [1080p]"), Resolution::R1080p);
        assert_eq!(detect_resolution("[G] Show - 01 [720p]"), Resolution::R720p);
        assert_eq!(detect_resolution("[G] Show - 01 [480p]"), Resolution::R480p);
        assert_eq!(detect_resolution("[G] Show - 01 [360p]"), Resolution::R360p);
    }

    #[test]
    fn detects_four_k_spellings() {
        assert_eq!(detect_resolution("Show [2160p]"), Resolution::R2160p);
        assert_eq!(detect_resolution("Show [4K]"), Resolution::R2160p);
    }

    #[test]
    fn unknown_resolution_is_not_a_low_one() {
        // A name with no resolution claim is Unknown, distinct from 360p.
        assert_eq!(detect_resolution("[G] Show - 01"), Resolution::Unknown);
    }

    #[test]
    fn does_not_read_a_resolution_out_of_a_number() {
        // "1080" inside a longer number must not count as 1080p.
        assert_eq!(detect_resolution("Show 10801"), Resolution::Unknown);
    }

    #[test]
    fn detects_bluray_and_remux() {
        assert_eq!(detect_source("Show BluRay"), ReleaseSource::BluRay);
        assert_eq!(detect_source("Show BDRip"), ReleaseSource::BdRip);
        assert_eq!(detect_source("Show BD-Rip"), ReleaseSource::BdRip);
    }

    #[test]
    fn detects_streaming_sources() {
        assert_eq!(detect_source("Show WEB-DL"), ReleaseSource::WebDl);
        assert_eq!(detect_source("Show WEBRip"), ReleaseSource::WebRip);
        assert_eq!(detect_source("Show WEB"), ReleaseSource::Unknown);
    }

    #[test]
    fn detects_broadcast_and_disc_sources() {
        assert_eq!(detect_source("Show HDTV"), ReleaseSource::Hdtv);
        assert_eq!(detect_source("Show DVD"), ReleaseSource::Dvd);
        assert_eq!(detect_source("Show TVRip"), ReleaseSource::TvRip);
    }

    #[test]
    fn unknown_source_is_reported_as_unknown() {
        assert_eq!(detect_source("[G] Show - 01 [1080p]"), ReleaseSource::Unknown);
    }

    #[test]
    fn remux_is_flagged() {
        let quality = parse_quality("Show.2020.REMUX.1080p.BluRay");
        assert!(quality.remux);
        assert_eq!(quality.source, ReleaseSource::BluRay);
    }

    #[test]
    fn proper_and_repack_are_revisions() {
        assert!(parse_quality("Show PROPER 1080p").revision);
        assert!(parse_quality("Show REPACK 1080p").revision);
        assert!(!parse_quality("Show 1080p").revision);
    }

    #[test]
    fn parse_quality_combines_every_signal() {
        let quality = parse_quality("[SubsPlease] Show - 12 (1080p) [WEB-DL]");
        assert_eq!(quality.resolution, Resolution::R1080p);
        assert_eq!(quality.source, ReleaseSource::WebDl);
        assert!(!quality.remux);
    }

    #[test]
    fn resolution_and_source_are_case_insensitive() {
        let quality = parse_quality("SHOW 1080P BLURAY");
        assert_eq!(quality.resolution, Resolution::R1080p);
        assert_eq!(quality.source, ReleaseSource::BluRay);
    }
}