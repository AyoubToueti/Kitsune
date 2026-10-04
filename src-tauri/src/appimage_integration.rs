//! Desktop integration for the AppImage build.
//!
//! An AppImage is one file with no installer, so nothing registers it with the
//! desktop environment: the window shows a generic icon and there is no menu
//! entry. Tools like `appimaged` or AppImageLauncher fix this, but not every
//! machine runs one, so the app does it itself on first launch -- writing the
//! desktop entry and icons into the user's `~/.local/share`, which is where the
//! shell looks.
//!
//! A no-op everywhere else: it only acts when `APPIMAGE` is set, which the
//! AppImage runtime exports. The deb and rpm packages are installed properly by
//! the package manager and need none of this.

use std::fs;
use std::path::Path;
use std::process::Command;

/// The GTK application id. Matches `app.enableGTKAppId` and the file name the
/// shell looks up, so the running window is associated with this entry.
const APP_ID: &str = "com.kitsune.app";
const APP_NAME: &str = "Kitsune";

/// Icon sizes and their PNG bytes, largest first.
///
/// Embedded rather than read from the AppImage: `include_bytes!` cannot fail at
/// runtime, and the set is a few tens of kilobytes.
static ICONS: &[(u32, &[u8])] = &[
    (256, include_bytes!("../icons/128x128@2x.png")),
    (128, include_bytes!("../icons/128x128.png")),
    (64, include_bytes!("../icons/64x64.png")),
    (32, include_bytes!("../icons/32x32.png")),
];

/// Register the AppImage with the desktop environment, if running as one.
pub fn integrate() {
    let Some(appimage) = std::env::var_os("APPIMAGE") else {
        return;
    };
    let appimage = appimage.to_string_lossy().into_owned();

    let Some(data_home) = dirs::data_dir() else {
        return;
    };

    let apps_dir = data_home.join("applications");
    let icons_root = data_home.join("icons/hicolor");

    let mut changed = false;

    let desktop_path = apps_dir.join(format!("{APP_ID}.desktop"));
    if write_if_changed(&desktop_path, desktop_entry(&appimage).as_bytes()) {
        changed = true;
    }

    for (size, bytes) in ICONS {
        let path = icons_root
            .join(format!("{size}x{size}/apps"))
            .join(format!("{APP_ID}.png"));
        if write_if_changed(&path, bytes) {
            changed = true;
        }
    }

    // Only refresh the caches when something actually changed, so an ordinary
    // launch does no work. Both commands are best-effort: a desktop without
    // `desktop-file-utils` still reads `~/.local/share/applications` directly.
    if changed {
        let _ = Command::new("update-desktop-database").arg(&apps_dir).status();
        let _ = Command::new("gtk-update-icon-cache")
            .arg("-q")
            .arg("-t")
            .arg(&icons_root)
            .status();
    }
}

/// The desktop entry, pointing at this AppImage.
fn desktop_entry(appimage: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={APP_NAME}\n\
         Comment=Anime streaming app\n\
         Exec=\"{appimage}\" %U\n\
         Icon={APP_ID}\n\
         Terminal=false\n\
         Categories=AudioVideo;Video;\n\
         StartupWMClass={APP_ID}\n\
         MimeType=x-scheme-handler/kitsune;\n"
    )
}

/// Write `contents` to `path`, creating parent directories; true when changed.
fn write_if_changed(path: &Path, contents: &[u8]) -> bool {
    if let Ok(existing) = fs::read(path) {
        if existing == contents {
            return false;
        }
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    fs::write(path, contents).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_points_at_the_appimage() {
        let entry = desktop_entry("/home/u/Apps/Kitsune.AppImage");
        assert!(entry.contains("Exec=\"/home/u/Apps/Kitsune.AppImage\" %U"));
        assert!(entry.contains("Icon=com.kitsune.app"));
        assert!(entry.contains("StartupWMClass=com.kitsune.app"));
    }

    #[test]
    fn write_if_changed_reports_only_real_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/file.txt");
        assert!(write_if_changed(&path, b"one"));
        assert!(!write_if_changed(&path, b"one"));
        assert!(write_if_changed(&path, b"two"));
        assert_eq!(fs::read(&path).unwrap(), b"two");
    }
}