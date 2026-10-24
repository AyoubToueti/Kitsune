//! Resolving a player's icon into a data URL for the picker.
//!
//! The picker draws its own UI, so it cannot show an `HICON` or a themed icon
//! name -- it needs bytes. The app's CSP allows `data:` but not `file:`, so an
//! icon has to be read here, encoded, and handed over inline.
//!
//! Where an icon comes from is platform-specific:
//!
//! * Linux: a `.desktop` file names its icon (`Icon=vlc`), and that name is
//!   looked up in the icon-theme directories. GIO does NOT do this for us --
//!   `gio info` on an executable reports only a generic MIME icon -- so the
//!   lookup is done here.
//! * Other platforms: none yet. The picker falls back to its own glyph, which
//!   is a deliberate choice over shipping Win32/ObjC icon extraction for a
//!   cosmetic feature.
//!
//! Every failure is `None`. A missing icon is not worth failing a launch over,
//! and the picker treats `None` as "draw the fallback glyph".

/// Largest icon file to inline, in bytes.
///
/// A themed PNG is a few KB; this only stops a pathological file from bloating
/// the IPC payload. Anything larger is skipped rather than sent.
const MAX_ICON_BYTES: u64 = 256 * 1024;

/// The pixel sizes to prefer, largest first.
///
/// The picker draws the icon small, but a larger source scales down cleanly on
/// a HiDPI display, where a 16px source would look soft.
const PREFERRED_SIZES: &[u32] = &[256, 128, 64, 48, 32, 24, 16];

/// The extension to try for each size, in order.
///
/// PNG first because it is what every modern theme ships and what the webview
/// always renders; SVG is a common alternative and is also supported. XPM is
/// deliberately absent -- browsers cannot render it.
const EXTENSIONS: &[&str] = &["png", "svg"];

/// The icon-theme roots to search, most specific first.
///
/// Follows the freedesktop lookup: the user's own data dir, then each system
/// data dir. On NixOS the per-user profile directory is where an app's own
/// icons land, so it is added explicitly -- it is not always in `XDG_DATA_DIRS`.
pub fn icon_theme_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        roots.push(home.join(".local/share/icons"));
        roots.push(home.join(".nix-profile/share/icons"));
    }

    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for dir in std::env::split_paths(&data_dirs) {
            roots.push(dir.join("icons"));
        }
    } else {
        roots.push(std::path::PathBuf::from("/usr/share/icons"));
        roots.push(std::path::PathBuf::from("/usr/local/share/icons"));
    }

    roots.push(std::path::PathBuf::from("/run/current-system/sw/share/icons"));
    roots
}

/// Pull the `Icon=` value out of a `.desktop` file's text.
///
/// Kept pure so the parsing is tested without a real file. Only the `[Desktop
/// Entry]` group is read, so an `Icon=` in a later group (a `[Desktop Action]`,
/// say) is ignored. A blank value is treated as absent.
pub fn icon_name_from_desktop(text: &str) -> Option<String> {
    let mut in_desktop_entry = false;

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some(value) = line.strip_prefix("Icon=") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

/// Find an icon file for `name` under the given theme roots.
///
/// `name` may be an absolute path (some entries store one) -- that is checked
/// first. Otherwise it is looked up as `hicolor/<size>x<size>/apps/<name>.<ext>`
/// across every root, trying the largest size first. Pure in its inputs so the
/// ordering is tested with a fake directory.
pub fn resolve_icon_path(
    name: &str,
    roots: &[std::path::PathBuf],
) -> Option<std::path::PathBuf> {
    // An absolute path in the entry is used as-is; there is nothing to look up.
    let as_path = std::path::Path::new(name);
    if as_path.is_absolute() {
        return as_path.is_file().then(|| as_path.to_path_buf());
    }

    for root in roots {
        for size in PREFERRED_SIZES {
            for ext in EXTENSIONS {
                let candidate = root
                    .join("hicolor")
                    .join(format!("{size}x{size}"))
                    .join("apps")
                    .join(format!("{name}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

/// Read an icon file and encode it as a `data:` URL the webview can render.
///
/// `None` when the file is missing, empty, or larger than [`MAX_ICON_BYTES`] --
/// all of which the picker renders as its fallback glyph. The MIME type is
/// derived from the extension so an SVG is sent as `image/svg+xml`, without
/// which the webview would refuse it.
pub fn icon_data_url(path: &std::path::Path) -> Option<String> {
    use base64::Engine;

    let meta = std::fs::metadata(path).ok()?;
    if meta.len() == 0 || meta.len() > MAX_ICON_BYTES {
        return None;
    }

    let bytes = std::fs::read(path).ok()?;
    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("svg") => "image/svg+xml",
        _ => "image/png",
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

    Some(format!("data:{mime};base64,{encoded}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_name_is_read_from_the_desktop_entry_group() {
        let text = "[Desktop Entry]\nName=VLC\nIcon=vlc\nExec=vlc %U\n";
        assert_eq!(icon_name_from_desktop(text), Some("vlc".to_string()));
    }

    #[test]
    fn icon_name_ignores_a_later_group() {
        // The action group has its own Icon=; only [Desktop Entry] counts.
        let text = "[Desktop Entry]\nName=VLC\n\n[Desktop Action x]\nIcon=other\n";
        assert_eq!(icon_name_from_desktop(text), None);
    }

    #[test]
    fn blank_icon_name_is_absent() {
        assert_eq!(icon_name_from_desktop("[Desktop Entry]\nIcon=\n"), None);
        assert_eq!(icon_name_from_desktop("[Desktop Entry]\nIcon=   \n"), None);
    }

    #[test]
    fn missing_desktop_entry_group_has_no_icon() {
        assert_eq!(icon_name_from_desktop("Name=VLC\nIcon=vlc\n"), None);
    }

    #[test]
    fn resolve_prefers_the_largest_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let apps = dir.path().join("hicolor").join("128x128").join("apps");
        std::fs::create_dir_all(&apps).expect("mkdir");
        std::fs::write(apps.join("vlc.png"), b"x").expect("write");
        let small = dir.path().join("hicolor").join("16x16").join("apps");
        std::fs::create_dir_all(&small).expect("mkdir");
        std::fs::write(small.join("vlc.png"), b"x").expect("write");

        let found = resolve_icon_path("vlc", &[dir.path().to_path_buf()]).expect("icon");
        assert!(found.to_string_lossy().contains("128x128"));
    }

    #[test]
    fn resolve_falls_back_to_a_smaller_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let apps = dir.path().join("hicolor").join("16x16").join("apps");
        std::fs::create_dir_all(&apps).expect("mkdir");
        std::fs::write(apps.join("mpv.png"), b"x").expect("write");

        let found = resolve_icon_path("mpv", &[dir.path().to_path_buf()]).expect("icon");
        assert!(found.to_string_lossy().contains("16x16"));
    }

    #[test]
    fn resolve_has_no_icon_when_the_theme_lacks_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(resolve_icon_path("nope", &[dir.path().to_path_buf()]), None);
    }

    #[test]
    fn resolve_uses_an_absolute_name_directly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let icon = dir.path().join("custom.png");
        std::fs::write(&icon, b"x").expect("write");
        // No theme roots: only the absolute path can answer.
        let found = resolve_icon_path(&icon.to_string_lossy(), &[]).expect("icon");
        assert_eq!(found, icon);
    }

    #[test]
    fn data_url_carries_the_png_mime_and_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let icon = dir.path().join("vlc.png");
        std::fs::write(&icon, b"PNGDATA").expect("write");

        let url = icon_data_url(&icon).expect("data url");
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn data_url_uses_the_svg_mime_for_svg() {
        let dir = tempfile::tempdir().expect("tempdir");
        let icon = dir.path().join("mpv.svg");
        std::fs::write(&icon, b"<svg/>").expect("write");

        let url = icon_data_url(&icon).expect("data url");
        assert!(url.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn data_url_is_none_for_a_missing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(icon_data_url(&dir.path().join("nope.png")), None);
    }
}