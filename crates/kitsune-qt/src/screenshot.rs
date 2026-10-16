//! Rust binding for the C++ screenshot helper.
//!
//! Kept in its own `#[cxx::bridge]` (not the CXX-Qt one) because the helper is
//! a plain free function, not a QObject. It takes `&str`, which crosses as
//! `rust::Str`; the QString is built on the C++ side.

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("screenshot.h");

        /// Save the first top-level window to `path` after `delay_ms`, then
        /// quit. No-op unless the caller opts in.
        #[rust_name = "save_after"]
        fn kitsuneSaveScreenshot(path: &str, delay_ms: i32);

        /// Install the Qt message filter that drops known-harmless startup
        /// warnings. Call once, before QML loads.
        #[rust_name = "install_log_filter"]
        fn kitsuneInstallLogFilter();

        /// Declare which desktop entry the window belongs to, e.g.
        /// "com.kitsune.app". On Wayland this becomes the window's app_id, which
        /// the compositor matches against an installed `.desktop` file to pick
        /// the taskbar icon.
        #[rust_name = "set_desktop_file_name"]
        fn kitsuneSetDesktopFileName(name: &str);

        /// Set the application window icon from a Qt resource path, so the
        /// compositor shows the app's own icon in the taskbar.
        #[rust_name = "set_window_icon"]
        fn kitsuneSetWindowIcon(resource_path: &str);
    }
}

/// Schedule a screenshot of the app window and a quit, `delay_ms` from now.
pub fn save_after(path: &str, delay_ms: i32) {
    ffi::save_after(path, delay_ms);
}

/// Install the Qt message filter that drops known-harmless startup warnings.
pub fn install_log_filter() {
    ffi::install_log_filter();
}

/// Declare which desktop entry the window belongs to, so the desktop
/// environment shows the app's own taskbar icon rather than a generic one.
pub fn set_desktop_file_name(name: &str) {
    ffi::set_desktop_file_name(name);
}

/// Set the application window icon from a Qt resource path.
pub fn set_window_icon(resource_path: &str) {
    ffi::set_window_icon(resource_path);
}