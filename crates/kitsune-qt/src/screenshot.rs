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
    }
}

/// Schedule a screenshot of the app window and a quit, `delay_ms` from now.
pub fn save_after(path: &str, delay_ms: i32) {
    ffi::save_after(path, delay_ms);
}