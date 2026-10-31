//! Matching a torrent's files against an episode number.
//!
//! Ported from the Svelte frontend's `src/lib/torrent-files.ts`, so both
//! frontends agree on which file an episode is.
//!
//! A torrent ships more than the episode: subtitles, cover art, sample clips
//! and sometimes a whole batch. The reader selects an episode and expects the
//! right video, so this is the rule that turns "episode 7" into one file -- or
//! into nothing, when the naming genuinely does not line up and the reader must
//! pick by hand.

use once_cell::sync::Lazy;
use fancy_regex::Regex;
use serde::{Deserialize, Serialize};

use crate::player::state::TorrentFile;

/// What a file is, for the type icon and label in a file row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Video,
    Subtitle,
    Image,
    Other,
}

/// Whether a file's name carries `number` as a delimited token.
///
/// The number is matched as a delimited token so "03" does not match "1080p" or
/// "2023". Split out so the relative and absolute spellings share one rule.
pub fn match_number(files: &[TorrentFile], number: u32) -> Option<&TorrentFile> {
    // `0*` lets the same pattern match a zero-padded number, so "03" and "3"
    // both resolve. `(?:^|[^0-9]) ... (?:[^0-9]|$)` is the token boundary.
    let pattern = Regex::new(&format!(r"(?:^|[^0-9])0*{number}(?:[^0-9]|$)")).ok()?;

    let direct = files
        .iter()
        .find(|file| pattern.is_match(&file.name).unwrap_or(false));
    if direct.is_some() {
        return direct;
    }

    // Fall back to the zero-padded spelling before giving up, since some
    // releases only ever write "03" and a bare "3" would not match.
    let token = format!("{number:02}");
    files.iter().find(|file| file.name.contains(&token))
}

/// The file that best matches an episode number, or `None` when nothing does.
///
/// `offset` bridges a later cour's numbering: its releases are named with the
/// running total (`Show - 13`) even though its own list starts at 1. It is 0 for
/// an ordinary 1..N season, in which case the absolute attempt would repeat the
/// relative one and is skipped.
pub fn file_for_episode(
    files: &[TorrentFile],
    number: u32,
    offset: u32,
) -> Option<&TorrentFile> {
    let direct = match_number(files, number);
    if direct.is_some() {
        return direct;
    }

    let absolute = number + offset;
    if absolute == number {
        return None;
    }

    match_number(files, absolute)
}

/// Whether a file looks like something a video player can open.
pub fn is_playable(name: &str) -> bool {
    static PLAYABLE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\.(mkv|mp4|avi|webm|mov|m4v)$").unwrap());
    PLAYABLE.is_match(name).unwrap_or(false)
}

/// How many playable videos a torrent holds.
///
/// Deliberately counts only playable files: a release that ships the episode
/// beside a subtitle and cover art is one video, not three.
pub fn playable_count(files: &[TorrentFile]) -> usize {
    files.iter().filter(|file| is_playable(&file.name)).count()
}

/// The largest playable file, since a full-length episode dwarfs the sample
/// clips and cover art some releases ship beside it.
///
/// Falls back to the largest file of any kind, so a torrent whose naming is
/// opaque still plays something rather than nothing.
pub fn best_effort_file(files: &[TorrentFile]) -> Option<&TorrentFile> {
    let playable: Vec<&TorrentFile> = files
        .iter()
        .filter(|file| is_playable(&file.name))
        .collect();

    let pool = if playable.is_empty() {
        files.iter().collect::<Vec<_>>()
    } else {
        playable
    };

    pool.into_iter().max_by_key(|file| file.length_bytes)
}

/// The file to start automatically, or `None` to wait for the reader's pick.
///
/// The rule that keeps a batch from downloading an arbitrary guess: whenever
/// there is more than one video to choose from, NOTHING is started -- the reader
/// picks, and the torrent is held paused until they do. A single video always
/// starts, since there is no choice to make.
pub fn first_target_file(files: &[TorrentFile]) -> Option<&TorrentFile> {
    if playable_count(files) > 1 {
        return None;
    }
    best_effort_file(files)
}

/// Whether a file's name contains `query`, case-insensitively.
///
/// The file list's filter box. A plain substring match rather than a token
/// search: the reader is usually typing a fragment of a name. An empty query
/// matches everything.
pub fn matches_file_query(file: &TorrentFile, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    file.name.to_lowercase().contains(&needle)
}

/// Classify a file by its extension, for the row's icon and label.
pub fn file_kind(name: &str) -> FileKind {
    let lower = name.to_lowercase();
    if Regex::new(r"\.(mkv|mp4|avi|webm|mov|m4v|ts|m2ts)$")
        .map(|re| re.is_match(&lower).unwrap_or(false))
        .unwrap_or(false)
    {
        return FileKind::Video;
    }
    if Regex::new(r"\.(ass|srt|ssa|vtt|sub)$")
        .map(|re| re.is_match(&lower).unwrap_or(false))
        .unwrap_or(false)
    {
        return FileKind::Subtitle;
    }
    if Regex::new(r"\.(jpg|jpeg|png|gif|webp|bmp)$")
        .map(|re| re.is_match(&lower).unwrap_or(false))
        .unwrap_or(false)
    {
        return FileKind::Image;
    }
    FileKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(idx: usize, name: &str, length_bytes: u64) -> TorrentFile {
        TorrentFile {
            idx,
            name: name.to_string(),
            length_bytes,
        }
    }

    #[test]
    fn matches_a_zero_padded_number() {
        let files = vec![file(0, "Show - 03 [1080p].mkv", 100)];
        assert!(match_number(&files, 3).is_some());
    }

    #[test]
    fn matches_the_right_episode_among_several() {
        let files = vec![
            file(0, "Show - 05 [1080p].mkv", 100),
            file(1, "Show - 06 [1080p].mkv", 100),
        ];
        assert_eq!(match_number(&files, 6).map(|f| f.idx), Some(1));
    }

    #[test]
    fn a_number_absent_from_the_name_does_not_match() {
        let files = vec![file(0, "Show - 05 [1080p].mkv", 100)];
        // 99 appears nowhere, not even as a substring.
        assert!(match_number(&files, 99).is_none());
    }

    #[test]
    fn file_for_episode_uses_the_offset() {
        let files = vec![file(0, "Show - 13.mkv", 100)];
        assert!(file_for_episode(&files, 1, 12).is_some());
        assert!(file_for_episode(&files, 1, 0).is_none());
    }

    #[test]
    fn a_batch_waits_for_a_pick() {
        let files = vec![
            file(0, "Show - 01.mkv", 100),
            file(1, "Show - 02.mkv", 100),
        ];
        assert!(first_target_file(&files).is_none());
    }

    #[test]
    fn a_single_video_starts_automatically() {
        let files = vec![
            file(0, "Show - 01.mkv", 100),
            file(1, "Show - 01.srt", 1),
        ];
        assert_eq!(first_target_file(&files).map(|f| f.idx), Some(0));
    }

    #[test]
    fn best_effort_prefers_the_largest_video() {
        let files = vec![
            file(0, "sample.mkv", 10),
            file(1, "episode.mkv", 900),
            file(2, "cover.jpg", 500),
        ];
        assert_eq!(best_effort_file(&files).map(|f| f.idx), Some(1));
    }

    #[test]
    fn query_matches_case_insensitively() {
        let f = file(0, "Show - 01 [SubsPlease].mkv", 100);
        assert!(matches_file_query(&f, "subsplease"));
        assert!(matches_file_query(&f, ""));
        assert!(!matches_file_query(&f, "nope"));
    }

    #[test]
    fn kind_is_classified_by_extension() {
        assert_eq!(file_kind("a.mkv"), FileKind::Video);
        assert_eq!(file_kind("a.srt"), FileKind::Subtitle);
        assert_eq!(file_kind("a.jpg"), FileKind::Image);
        assert_eq!(file_kind("a.txt"), FileKind::Other);
    }
}