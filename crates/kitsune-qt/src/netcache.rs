//! Rust binding for the C++ HTTP disk cache shim.
//!
//! Kept in its own `#[cxx::bridge]` like `screenshot.rs`, because the shim is a
//! plain free function rather than a QObject. `QQmlApplicationEngine` is
//! DECLARED here without a `#[qobject]` attribute: CXX allows the same C++ type
//! to be named by several bridges as long as only one of them gives it
//! behaviour, and cxx-qt-lib owns the real declaration. This bridge only needs
//! to forward the reference through.

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("netcache.h");
        include!("cxx-qt-lib/qqmlapplicationengine.h");

        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;

        /// Attach a `QNetworkDiskCache` to every network manager the engine
        /// creates. Must run before `load()`, or the first image request goes
        /// out on a manager with no cache.
        #[rust_name = "install_network_disk_cache"]
        fn kitsuneInstallNetworkDiskCache(
            engine: Pin<&mut QQmlApplicationEngine>,
            max_bytes: i32,
        );
    }
}

/// How much image data to keep on disk. Covers, banners and episode stills are
/// small individually but a long-running show is a hundred thumbnails, so a
/// small cap would thrash.
const MAX_CACHE_BYTES: i32 = 512 * 1024 * 1024;

/// Install the disk HTTP cache on `engine`.
///
/// The directory is resolved on the C++ side from
/// `QStandardPaths::CacheLocation`, so there is no Rust-side XDG lookup and the
/// path matches whatever Qt already uses for this app.
pub fn install(engine: core::pin::Pin<&mut cxx_qt_lib::QQmlApplicationEngine>) {
    ffi::install_network_disk_cache(engine, MAX_CACHE_BYTES);
}