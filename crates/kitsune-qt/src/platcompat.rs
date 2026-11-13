//! Rust binding for the C++ environment/locale helpers.
//!
//! See `cpp/platcompat.h` for why these are not plain `extern "C"` calls: the
//! CRT spellings (`setenv` vs `_putenv_s`, `LC_NUMERIC` = 1 vs 2) differ
//! between glibc and MSVC, and `qputenv`/`std::setlocale` are the portable
//! versions of both.

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("platcompat.h");

        /// Set an environment variable, overwriting any existing value.
        #[rust_name = "set_env"]
        fn kitsuneSetEnv(name: &str, value: &str);

        /// Reset `LC_NUMERIC` to "C" for C libraries that parse numbers with
        /// `strtod` and assume a dot decimal separator (libmpv).
        #[rust_name = "reset_numeric_locale"]
        fn kitsuneResetNumericLocale();

        /// Relaunch the executable and quit this instance.
        ///
        /// False when the replacement could not be started, so the caller can
        /// keep the current window open instead of closing the app.
        #[rust_name = "restart_app"]
        fn kitsuneRestartApp() -> bool;
    }
}

/// Set an environment variable before the Qt application object exists.
pub fn set_env(name: &str, value: &str) {
    ffi::set_env(name, value);
}

/// Reset the numeric locale after the Qt application object exists.
pub fn reset_numeric_locale() {
    ffi::reset_numeric_locale();
}

/// Restart the application: spawn a detached copy, then quit this one.
///
/// `false` means the new process could not be started and nothing was quit, so
/// the caller should stay running.
pub fn restart_app() -> bool {
    ffi::restart_app()
}