//! The Rust <-> QML bridge.
//!
//! `Backend` is the single QObject QML talks to. Every command the old Svelte
//! frontend made with `invoke(...)` becomes either a `#[qinvokable]` method or
//! a signal on this object.
//!
//! # Async commands
//!
//! Commands that hit the network must not block the Qt event loop, so they
//! follow one pattern:
//!
//! 1. The `#[qinvokable]` clones a `CxxQtThread` handle and the shared
//!    tokio runtime, then returns immediately.
//! 2. The async work runs on the runtime.
//! 3. When it finishes, the result is queued back onto the Qt thread with
//!    `qt_thread.queue(...)`, which emits a `#[qsignal]`.
//!
//! QML never blocks: it calls `getTrending(20)` and listens for
//! `trendingReady(json)`.
//!
//! Results cross the boundary as JSON strings, exactly the camelCase shape the
//! Svelte frontend already consumed. QML does `JSON.parse(...)`. This keeps the
//! existing Rust `serde` contracts untouched and avoids hand-mapping every
//! field into Qt types.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type Backend = super::BackendRust;

        /// Trending titles as a JSON array, for the home screen.
        ///
        /// `limit <= 0` means "use the default". The result arrives via
        /// [`Backend::trending_ready`] or [`Backend::trending_failed`].
        #[qinvokable]
        #[cxx_name = "getTrending"]
        fn get_trending(self: Pin<&mut Self>, limit: i32);

        /// A JSON array of trending titles, in the shape the Svelte frontend
        /// consumed (`src/lib/types.ts`).
        #[qsignal]
        #[cxx_name = "trendingReady"]
        fn trending_ready(self: Pin<&mut Self>, json: QString);

        /// The human-readable reason the trending fetch failed.
        #[qsignal]
        #[cxx_name = "trendingFailed"]
        fn trending_failed(self: Pin<&mut Self>, message: QString);
    }

    // Enables `self.qt_thread()` and the `queue(...)` used to hop back onto the
    // Qt event loop from a tokio worker.
    impl cxx_qt::Threading for Backend {}
}

use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use kitsune_core::providers::{AniListProvider, AnimeProvider};

/// The default page size, mirroring `commands::DEFAULT_LIMIT` in the Tauri app.
const DEFAULT_LIMIT: u32 = 20;

/// The Rust half of `Backend`.
///
/// Holds the process-wide async runtime and the one provider instance, so every
/// command shares the same connection pool (the reason `lib.rs` in the Tauri
/// app built a single provider too).
pub struct BackendRust {
    runtime: Arc<tokio::runtime::Runtime>,
    provider: Arc<dyn AnimeProvider>,
}

impl Default for BackendRust {
    fn default() -> Self {
        // A multi-threaded runtime so a slow request cannot stall another
        // command's future. `enable_all` turns on the IO + time drivers reqwest
        // needs.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build the tokio runtime");

        Self {
            runtime: Arc::new(runtime),
            provider: Arc::new(AniListProvider::new()),
        }
    }
}

impl qobject::Backend {
    fn get_trending(self: Pin<&mut Self>, limit: i32) {
        // Grab everything the async task needs BEFORE returning, since `self`
        // is not `Send` and cannot cross into the runtime.
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);

        runtime.spawn(async move {
            let limit = if limit <= 0 {
                DEFAULT_LIMIT
            } else {
                limit as u32
            };

            match provider.trending(limit).await {
                Ok(items) => {
                    // `unwrap_or` rather than `?`: a serialisation failure is
                    // not worth dropping the whole response over, and `[]` is
                    // a safe fallback the UI already handles.
                    let json =
                        serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.trending_ready(QString::from(&json));
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.trending_failed(QString::from(&message));
                    });
                }
            }
        });
    }
}