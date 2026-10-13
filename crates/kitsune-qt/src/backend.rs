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

        // --- Metadata ---------------------------------------------------------

        /// Trending titles as a JSON array. `limit <= 0` means "use default".
        #[qinvokable]
        #[cxx_name = "getTrending"]
        fn get_trending(self: Pin<&mut Self>, limit: i32);

        /// A curated list, by intent. `filter` is the wire string
        /// ("trending", "topAiring", ...).
        #[qinvokable]
        #[cxx_name = "getList"]
        fn get_list(self: Pin<&mut Self>, filter: QString, limit: i32);

        /// Browse with filters and paging. `query` is a JSON `BrowseQuery`.
        #[qinvokable]
        #[cxx_name = "getBrowse"]
        fn get_browse(
            self: Pin<&mut Self>,
            query: QString,
            page: i32,
            per_page: i32,
        );

        /// Look up a single title. Emits JSON `null` when the id is unknown.
        #[qinvokable]
        #[cxx_name = "getAnime"]
        fn get_anime(self: Pin<&mut Self>, id: i64);

        /// A page of recommendations for one work.
        #[qinvokable]
        #[cxx_name = "getRecommendations"]
        fn get_recommendations(self: Pin<&mut Self>, id: i64, page: i32, per_page: i32);

        /// The genre names available for browsing.
        #[qinvokable]
        #[cxx_name = "getGenres"]
        fn get_genres(self: Pin<&mut Self>);

        /// The tags available for filtering, each with its grouping category.
        #[qinvokable]
        #[cxx_name = "getTags"]
        fn get_tags(self: Pin<&mut Self>);

        /// Broadcasts within a unix-second window `[from, to]`, soonest first.
        #[qinvokable]
        #[cxx_name = "getSchedule"]
        fn get_schedule(self: Pin<&mut Self>, from: i64, to: i64, limit: i32);

        /// Full episode metadata for a work, by its MyAnimeList id.
        #[qinvokable]
        #[cxx_name = "getEpisodes"]
        fn get_episodes(self: Pin<&mut Self>, mal_id: i64);

        // --- Results (one ok/err pair per command) ----------------------------

        #[qsignal]
        #[cxx_name = "trendingReady"]
        fn trending_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "trendingFailed"]
        fn trending_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "listReady"]
        fn list_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "listFailed"]
        fn list_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "browseReady"]
        fn browse_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "browseFailed"]
        fn browse_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "animeReady"]
        fn anime_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "animeFailed"]
        fn anime_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "recommendationsReady"]
        fn recommendations_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "recommendationsFailed"]
        fn recommendations_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "genresReady"]
        fn genres_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "genresFailed"]
        fn genres_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "tagsReady"]
        fn tags_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "tagsFailed"]
        fn tags_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "scheduleReady"]
        fn schedule_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "scheduleFailed"]
        fn schedule_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "episodesReady"]
        fn episodes_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "episodesFailed"]
        fn episodes_failed(self: Pin<&mut Self>, message: QString);
    }

    // Enables `self.qt_thread()` and the `queue(...)` used to hop back onto the
    // Qt event loop from a tokio worker.
    impl cxx_qt::Threading for Backend {}
}

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use kitsune_core::providers::{AniListProvider, AnimeProvider, JikanProvider};
use kitsune_core::types::{BrowseQuery, ListFilter};

/// The default page size, mirroring `commands::DEFAULT_LIMIT` in the Tauri app.
const DEFAULT_LIMIT: u32 = 20;

/// The Rust half of `Backend`.
///
/// Holds the process-wide async runtime and the provider instances, so every
/// command shares the same connection pools (the reason `lib.rs` in the Tauri
/// app built single providers too).
pub struct BackendRust {
    runtime: Arc<tokio::runtime::Runtime>,
    provider: Arc<dyn AnimeProvider>,
    episodes: Arc<JikanProvider>,
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
            episodes: Arc::new(JikanProvider::new()),
        }
    }
}

/// Resolve a caller-supplied limit the same way `commands::resolve_limit` did:
/// `None` and `0` alike mean "unset".
fn limit_or_default(limit: i32) -> u32 {
    if limit <= 0 {
        DEFAULT_LIMIT
    } else {
        limit as u32
    }
}

impl qobject::Backend {
    /// Run `work` on the tokio runtime and deliver the result back on the Qt
    /// thread.
    ///
    /// `work` receives the shared provider and returns either a JSON string or
    /// a human-readable error. `ok`/`err` are the generated signal methods, so
    /// this one helper backs every metadata command.
    fn spawn_metadata<M, Fut>(
        self: Pin<&mut Self>,
        work: M,
        ok: fn(Pin<&mut qobject::Backend>, QString),
        err: fn(Pin<&mut qobject::Backend>, QString),
    ) where
        M: FnOnce(Arc<dyn AnimeProvider>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<String, String>> + Send + 'static,
    {
        // Clone everything the task needs BEFORE leaving the Qt thread; `self`
        // is not `Send` and cannot cross into the runtime.
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);

        runtime.spawn(async move {
            match work(provider).await {
                Ok(json) => {
                    let _ = qt_thread.queue(move |qobject| ok(qobject, QString::from(&json)));
                }
                Err(message) => {
                    let _ =
                        qt_thread.queue(move |qobject| err(qobject, QString::from(&message)));
                }
            }
        });
    }

    fn get_trending(self: Pin<&mut Self>, limit: i32) {
        let limit = limit_or_default(limit);
        self.spawn_metadata(
            move |provider| async move {
                provider
                    .trending(limit)
                    .await
                    .map(|items| json_or_empty(&items))
                    .map_err(|err| err.to_string())
            },
            Self::trending_ready,
            Self::trending_failed,
        );
    }

    fn get_list(self: Pin<&mut Self>, filter: QString, limit: i32) {
        let filter = filter.to_string();
        let limit = limit_or_default(limit);
        self.spawn_metadata(
            move |provider| async move {
                let filter: ListFilter = serde_json::from_value(serde_json::Value::String(
                    filter,
                ))
                .map_err(|err| format!("unknown list filter: {err}"))?;
                provider
                    .list(filter, limit)
                    .await
                    .map(|items| json_or_empty(&items))
                    .map_err(|err| err.to_string())
            },
            Self::list_ready,
            Self::list_failed,
        );
    }

    fn get_browse(self: Pin<&mut Self>, query: QString, page: i32, per_page: i32) {
        let query = query.to_string();
        let page = page.max(1) as u32;
        let per_page = limit_or_default(per_page);
        self.spawn_metadata(
            move |provider| async move {
                let query: BrowseQuery = serde_json::from_str(&query)
                    .map_err(|err| format!("bad browse query: {err}"))?;
                provider
                    .browse(query, page, per_page)
                    .await
                    .map(|page| json_or_empty(&page))
                    .map_err(|err| err.to_string())
            },
            Self::browse_ready,
            Self::browse_failed,
        );
    }

    fn get_anime(self: Pin<&mut Self>, id: i64) {
        self.spawn_metadata(
            move |provider| async move {
                provider
                    .by_id(id)
                    .await
                    // `None` serialises to JSON `null`, which QML already
                    // handles as "not found".
                    .map(|anime| serde_json::to_string(&anime).unwrap_or_else(|_| "null".into()))
                    .map_err(|err| err.to_string())
            },
            Self::anime_ready,
            Self::anime_failed,
        );
    }

    fn get_recommendations(self: Pin<&mut Self>, id: i64, page: i32, per_page: i32) {
        let page = page.max(1) as u32;
        let per_page = limit_or_default(per_page);
        self.spawn_metadata(
            move |provider| async move {
                provider
                    .recommendations(id, page, per_page)
                    .await
                    .map(|page| json_or_empty(&page))
                    .map_err(|err| err.to_string())
            },
            Self::recommendations_ready,
            Self::recommendations_failed,
        );
    }

    fn get_genres(self: Pin<&mut Self>) {
        self.spawn_metadata(
            move |provider| async move {
                provider
                    .genres()
                    .await
                    .map(|genres| json_or_empty(&genres))
                    .map_err(|err| err.to_string())
            },
            Self::genres_ready,
            Self::genres_failed,
        );
    }

    fn get_tags(self: Pin<&mut Self>) {
        self.spawn_metadata(
            move |provider| async move {
                provider
                    .tags()
                    .await
                    .map(|tags| json_or_empty(&tags))
                    .map_err(|err| err.to_string())
            },
            Self::tags_ready,
            Self::tags_failed,
        );
    }

    fn get_schedule(self: Pin<&mut Self>, from: i64, to: i64, limit: i32) {
        let limit = limit_or_default(limit);
        self.spawn_metadata(
            move |provider| async move {
                provider
                    .schedule(from, to, limit)
                    .await
                    .map(|items| json_or_empty(&items))
                    .map_err(|err| err.to_string())
            },
            Self::schedule_ready,
            Self::schedule_failed,
        );
    }

    fn get_episodes(self: Pin<&mut Self>, mal_id: i64) {
        // Episodes come from Jikan, not the AnimeProvider, so this does not go
        // through `spawn_metadata`; the plumbing is otherwise identical.
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().episodes);

        runtime.spawn(async move {
            match provider.episodes(mal_id).await {
                Ok(items) => {
                    let json = json_or_empty(&items);
                    let _ = qt_thread
                        .queue(move |qobject| qobject.episodes_ready(QString::from(&json)));
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread
                        .queue(move |qobject| qobject.episodes_failed(QString::from(&message)));
                }
            }
        });
    }
}

/// Serialise to a JSON array, falling back to `[]` so a serialisation failure
/// never drops the whole response (the UI already handles an empty list).
fn json_or_empty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}