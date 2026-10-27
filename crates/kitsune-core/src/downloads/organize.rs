//! File organization for downloaded episodes.
//!
//! Builds the final path inside the library directory, sanitizing filenames so
//! the result is safe on Linux, macOS, and Windows. The layout is
//! `{library_dir}/{Anime Title}/Season {N}/{Anime Title} - S{N}E{NN}.ext`;
//! when season/episode metadata is missing it falls back to the original
//! torrent file name under `{library_dir}/{Anime Title}/`.
//!
//! The layout is written inline rather than as an indented block because an
//! indented block in a module doc is a rustdoc CODE BLOCK, and rustdoc would
//! then try to compile that path as Rust and fail the doc-test run.

use std::path::{Path, PathBuf};

/// Characters forbidden in filenames on at least one major OS.
///
/// Windows forbids `< > : " / \ | ? *` plus control characters. Linux and
/// macOS only forbid `/` and `\0`, but replacing the Windows set everywhere
/// keeps the library portable across filesystems.
const FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Sanitize a filename component.
///
/// Replaces forbidden characters with `_`, collapses consecutive whitespace
/// into a single space, trims leading/trailing whitespace and dots (Windows
/// refuses trailing dots), and falls back to `"_"` for an empty result.
fn sanitize(component: &str) -> String {
    let mut out = String::with_capacity(component.len());
    for ch in component.chars() {
        if FORBIDDEN.contains(&ch) || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    // Collapse runs of whitespace.
    let collapsed: String = out
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    // Windows refuses names that end in `.` or space.
    let trimmed = collapsed.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build the final path for a downloaded episode inside `library_dir`.
///
/// When `season` and `episode` are both known, the layout is
/// `{library_dir}/{Anime}/Season {N}/{Anime} - S{N}E{NN}.{ext}`. Otherwise
/// the original torrent `file_name` is preserved under
/// `{library_dir}/{Anime}/{file_name}`.
///
/// `ext` is the file extension (without the leading dot), e.g. `"mkv"`.
pub fn library_path(
    library_dir: &Path,
    anime_title: &str,
    season: Option<u32>,
    episode: Option<u32>,
    file_name: &str,
    ext: &str,
) -> PathBuf {
    let anime = sanitize(anime_title);
    let ext_clean = ext.trim_start_matches('.');

    match (season, episode) {
        (Some(s), Some(e)) => {
            let season_dir = format!("Season {}", s);
            let stem = format!("{} - S{:02}E{:02}", anime, s, e);
            let filename = if ext_clean.is_empty() {
                stem
            } else {
                format!("{}.{}", stem, ext_clean)
            };
            library_dir.join(&anime).join(season_dir).join(filename)
        }
        _ => {
            // Fall back to the original file name, sanitized.
            let filename = sanitize(file_name);
            library_dir.join(&anime).join(filename)
        }
    }
}

/// Extract the file extension from a filename, without the leading dot.
///
/// Returns an empty string when the filename has no extension.
pub fn extension_of(file_name: &str) -> String {
    Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_forbidden() {
        assert_eq!(sanitize("A:B<C>D"), "A_B_C_D");
        assert_eq!(sanitize(r"path\to/file"), "path_to_file");
        assert_eq!(sanitize("a|b?c*d"), "a_b_c_d");
    }

    #[test]
    fn sanitize_collapses_whitespace() {
        assert_eq!(sanitize("  hello   world  "), "hello world");
    }

    #[test]
    fn sanitize_trims_trailing_dots() {
        assert_eq!(sanitize("name..."), "name");
    }

    #[test]
    fn sanitize_empty_falls_back() {
        assert_eq!(sanitize(""), "_");
        assert_eq!(sanitize("..."), "_");
        assert_eq!(sanitize("   "), "_");
    }

    #[test]
    fn library_path_with_season_and_episode() {
        let dir = Path::new("/library");
        let path = library_path(dir, "Attack on Titan", Some(1), Some(5), "ep.mkv", "mkv");
        assert_eq!(
            path,
            PathBuf::from("/library/Attack on Titan/Season 1/Attack on Titan - S01E05.mkv")
        );
    }

    #[test]
    fn library_path_zero_pads_episode() {
        let dir = Path::new("/library");
        let path = library_path(dir, "Show", Some(2), Some(3), "x.mkv", "mkv");
        assert_eq!(
            path,
            PathBuf::from("/library/Show/Season 2/Show - S02E03.mkv")
        );
    }

    #[test]
    fn library_path_falls_back_without_season() {
        let dir = Path::new("/library");
        let path = library_path(dir, "Show", None, Some(5), "Show - 05.mkv", "mkv");
        assert_eq!(path, PathBuf::from("/library/Show/Show - 05.mkv"));
    }

    #[test]
    fn library_path_falls_back_without_episode() {
        let dir = Path::new("/library");
        let path = library_path(dir, "Show", Some(1), None, "special.mkv", "mkv");
        assert_eq!(path, PathBuf::from("/library/Show/special.mkv"));
    }

    #[test]
    fn library_path_sanitizes_anime_title() {
        let dir = Path::new("/library");
        let path = library_path(dir, "A:B<C>", Some(1), Some(1), "ep.mkv", "mkv");
        assert_eq!(
            path,
            PathBuf::from("/library/A_B_C_/Season 1/A_B_C_ - S01E01.mkv")
        );
    }

    #[test]
    fn library_path_handles_missing_extension() {
        let dir = Path::new("/library");
        let path = library_path(dir, "Show", Some(1), Some(1), "ep", "");
        assert_eq!(
            path,
            PathBuf::from("/library/Show/Season 1/Show - S01E01")
        );
    }

    #[test]
    fn extension_of_extracts_cleanly() {
        assert_eq!(extension_of("file.mkv"), "mkv");
        assert_eq!(extension_of("file.MKV"), "MKV");
        assert_eq!(extension_of("noext"), "");
        assert_eq!(extension_of("multi.tar.gz"), "gz");
    }
}