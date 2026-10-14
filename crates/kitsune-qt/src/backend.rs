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

        /// A curated list, with a caller-supplied `tag` echoed back on the
        /// answer. The home page fires four list requests at once and needs to
        /// tell which block each reply belongs to; the untagged `getList`
        /// cannot distinguish them.
        #[qinvokable]
        #[cxx_name = "getListTagged"]
        fn get_list_tagged(self: Pin<&mut Self>, tag: QString, filter: QString, limit: i32);

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

        /// Build the episode list to display for a work from its scalar facts
        /// plus the (possibly empty) Jikan catalogue JSON. `status` is the
        /// provider's status string (e.g. "FINISHED"). Pure and synchronous, so
        /// QML gets the shared core rules without re-deriving.
        #[qinvokable]
        #[cxx_name = "buildEpisodes"]
        fn build_episodes(
            self: Pin<&mut Self>,
            episode_count: i32,
            status: QString,
            cover_image: QString,
            info_json: QString,
        ) -> QString;

        /// The title forms worth searching an indexer for, as a JSON array
        /// (best first). Synchronous.
        #[qinvokable]
        #[cxx_name = "titleForms"]
        fn title_forms(
            self: Pin<&mut Self>,
            user_preferred: QString,
            english: QString,
            romaji: QString,
        ) -> QString;

        /// Choose the file to play from a torrent's files: the episode match,
        /// or the sole video, else `-1` to wait for the reader's pick.
        /// `files_json` is a JSON `TorrentFile[]`. Synchronous.
        #[qinvokable]
        #[cxx_name = "pickFile"]
        fn pick_file(
            self: Pin<&mut Self>,
            files_json: QString,
            episode: i32,
            offset: i32,
        ) -> i32;

        /// Search the indexers for a work's releases. `titles` is a JSON array
        /// of title forms; `mode` is the search-mode wire string.
        #[qinvokable]
        #[cxx_name = "searchReleases"]
        fn search_releases(
            self: Pin<&mut Self>,
            titles: QString,
            episode: i32,
            absolute_episode: i32,
            mode: QString,
        );

        /// Add a magnet and return the files it resolves to.
        #[qinvokable]
        #[cxx_name = "addMagnet"]
        fn add_magnet(self: Pin<&mut Self>, magnet_uri: QString);

        /// Drop a torrent from the session.
        #[qinvokable]
        #[cxx_name = "removeTorrent"]
        fn remove_torrent(self: Pin<&mut Self>, torrent_id: i64);

        /// Restrict a torrent's download to a single file index.
        #[qinvokable]
        #[cxx_name = "setOnlyFiles"]
        fn set_only_files(self: Pin<&mut Self>, torrent_id: i64, file_idx: i64);

        /// The loopback URL that streams one file of one torrent.
        #[qinvokable]
        #[cxx_name = "getStreamUrl"]
        fn get_stream_url(self: Pin<&mut Self>, torrent_id: i64, file_idx: i64);

        /// Pause a torrent's transfer.
        #[qinvokable]
        #[cxx_name = "pauseTorrent"]
        fn pause_torrent(self: Pin<&mut Self>, torrent_id: i64);

        /// Resume a paused torrent's transfer.
        #[qinvokable]
        #[cxx_name = "resumeTorrent"]
        fn resume_torrent(self: Pin<&mut Self>, torrent_id: i64);

        /// A download-progress snapshot, or JSON `null` when unknown.
        #[qinvokable]
        #[cxx_name = "getTorrentStats"]
        fn get_torrent_stats(self: Pin<&mut Self>, torrent_id: i64);

        /// Take a hold on a torrent while the embedded player reads it.
        #[qinvokable]
        #[cxx_name = "holdTorrent"]
        fn hold_torrent(self: Pin<&mut Self>, torrent_id: i64);

        /// Release a hold, removing the torrent when it was the last one.
        #[qinvokable]
        #[cxx_name = "releaseTorrent"]
        fn release_torrent(self: Pin<&mut Self>, torrent_id: i64);

        // --- Account (AniList list) ------------------------------------------
        //
        // NOTE: the interactive sign-in flow (open the authorise URL, catch the
        // `kitsune://` deep link) is NOT ported yet -- it needs OS scheme
        // registration. These commands work against a token already stored on
        // disk (e.g. written by the Tauri build), so a signed-in reader sees
        // their list without re-authenticating.

        /// The URL to open so the reader can authorise Kitsune.
        #[qinvokable]
        #[cxx_name = "beginLogin"]
        fn begin_login(self: Pin<&mut Self>) -> QString;

        /// Whether a token is stored.
        #[qinvokable]
        #[cxx_name = "authStatus"]
        fn auth_status(self: Pin<&mut Self>);

        /// Forget the stored token.
        #[qinvokable]
        #[cxx_name = "logout"]
        fn logout(self: Pin<&mut Self>);

        /// The works the reader is currently watching, most recent first.
        #[qinvokable]
        #[cxx_name = "continueWatching"]
        fn continue_watching(self: Pin<&mut Self>, limit: i32);

        /// The reader's whole list, grouped-ready.
        #[qinvokable]
        #[cxx_name = "userList"]
        fn user_list(self: Pin<&mut Self>);

        /// Put a work on the reader's list / move it between lists.
        /// `status` is the wire string ("current", "completed", ...).
        #[qinvokable]
        #[cxx_name = "setListEntry"]
        fn set_list_entry(
            self: Pin<&mut Self>,
            media_id: i64,
            status: QString,
            progress: i32,
        );

        /// Remove a work from the reader's list.
        #[qinvokable]
        #[cxx_name = "deleteListEntry"]
        fn delete_list_entry(self: Pin<&mut Self>, entry_id: i64);

        /// Where a work sits on the reader's list, or JSON `null`.
        #[qinvokable]
        #[cxx_name = "getListEntry"]
        fn get_list_entry(self: Pin<&mut Self>, media_id: i64);

        // --- Settings --------------------------------------------------------

        /// The reader's stored settings as JSON.
        #[qinvokable]
        #[cxx_name = "getSettings"]
        fn get_settings(self: Pin<&mut Self>);

        /// Replace the stored settings. `json` is a full `Settings` object.
        #[qinvokable]
        #[cxx_name = "setSettings"]
        fn set_settings(self: Pin<&mut Self>, json: QString);

        // --- Settings results -------------------------------------------------

        #[qsignal]
        #[cxx_name = "settingsReady"]
        fn settings_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "settingsFailed"]
        fn settings_failed(self: Pin<&mut Self>, message: QString);

        // --- Account results -------------------------------------------------

        #[qsignal]
        #[cxx_name = "authStatusReady"]
        fn auth_status_ready(self: Pin<&mut Self>, signed_in: bool);

        #[qsignal]
        #[cxx_name = "continueWatchingReady"]
        fn continue_watching_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "continueWatchingFailed"]
        fn continue_watching_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "userListReady"]
        fn user_list_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "userListFailed"]
        fn user_list_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "listEntryReady"]
        fn list_entry_ready(self: Pin<&mut Self>, json: QString);

        /// Emitted after any successful list write, so views can re-read.
        #[qsignal]
        #[cxx_name = "listChanged"]
        fn list_changed(self: Pin<&mut Self>);

        #[qsignal]
        #[cxx_name = "authError"]
        fn auth_error(self: Pin<&mut Self>, message: QString);

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

        /// The tagged variant: `tag` is the caller's own label, echoed back.
        #[qsignal]
        #[cxx_name = "taggedListReady"]
        fn tagged_list_ready(self: Pin<&mut Self>, tag: QString, json: QString);
        #[qsignal]
        #[cxx_name = "taggedListFailed"]
        fn tagged_list_failed(self: Pin<&mut Self>, tag: QString, message: QString);

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

        // --- Release / torrent results ---------------------------------------

        #[qsignal]
        #[cxx_name = "releasesReady"]
        fn releases_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "releasesFailed"]
        fn releases_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "torrentAdded"]
        fn torrent_added(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "torrentAddFailed"]
        fn torrent_add_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "streamUrlReady"]
        fn stream_url_ready(self: Pin<&mut Self>, torrent_id: i64, file_idx: i64, url: QString);
        #[qsignal]
        #[cxx_name = "streamUrlFailed"]
        fn stream_url_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "torrentStatsReady"]
        fn torrent_stats_ready(self: Pin<&mut Self>, json: QString);
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

use kitsune_core::auth::{TokenStore, flow};
use kitsune_core::indexer::{search_all, IndexerRegistry, ReleaseRequest, SearchMode};
use kitsune_core::player::state::PlayerState;
use kitsune_core::providers::{AniListProvider, AnimeProvider, JikanProvider};
use kitsune_core::settings::{Settings, SettingsStore};
use kitsune_core::types::{BrowseQuery, EpisodeInfo, ListFilter, ListStatus, ReleasePreference, Title};

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
    /// The concrete AniList handle, for account operations that are not on the
    /// `AnimeProvider` trait (token, list writes). Points at the SAME instance
    /// as `provider`, so a token set here is used by every metadata call.
    anilist: Arc<AniListProvider>,
    episodes: Arc<JikanProvider>,
    /// The AniList token, read from disk at startup so a signed-in reader
    /// stays signed in across restarts.
    tokens: Arc<TokenStore>,
    /// The reader's preferences, read and written by the Qt app.
    settings: Arc<SettingsStore>,
    /// The indexers a release search fans out to.
    indexers: Arc<IndexerRegistry>,
    /// The torrent engine + stream bridge + player holds, started lazily on
    /// the first playback. Shares the settings store so the reader's download
    /// directory and player preferences are honoured.
    player: Arc<PlayerState>,
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

        let anilist = Arc::new(AniListProvider::new());
        let tokens = Arc::new(TokenStore::new());

        // Restore any stored token into the provider, so a reader who signed in
        // previously is authenticated from the first request.
        if let Some(token) = tokens.read() {
            anilist.set_token(Some(token));
        }

        let settings = Arc::new(SettingsStore::new());

        Self {
            runtime: Arc::new(runtime),
            provider: anilist.clone(),
            anilist,
            episodes: Arc::new(JikanProvider::new()),
            tokens,
            player: Arc::new(PlayerState::with_settings(settings.clone())),
            settings,
            indexers: Arc::new(IndexerRegistry::new()),
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

    /// The tagged variant of `get_list`. Same work, but the answer carries the
    /// caller's `tag` back so concurrent requests can be told apart.
    fn get_list_tagged(self: Pin<&mut Self>, tag: QString, filter: QString, limit: i32) {
        let tag = tag.to_string();
        let filter = filter.to_string();
        let limit = limit_or_default(limit);

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);

        runtime.spawn(async move {
            let parsed: Result<ListFilter, String> =
                serde_json::from_value(serde_json::Value::String(filter))
                    .map_err(|err| format!("unknown list filter: {err}"));

            match parsed {
                Ok(filter) => match provider.list(filter, limit).await {
                    Ok(items) => {
                        let json = json_or_empty(&items);
                        let _ = qt_thread.queue(move |qobject| {
                            qobject.tagged_list_ready(QString::from(&tag), QString::from(&json))
                        });
                    }
                    Err(err) => {
                        let message = err.to_string();
                        let _ = qt_thread.queue(move |qobject| {
                            qobject.tagged_list_failed(
                                QString::from(&tag),
                                QString::from(&message),
                            )
                        });
                    }
                },
                Err(message) => {
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.tagged_list_failed(
                            QString::from(&tag),
                            QString::from(&message),
                        )
                    });
                }
            }
        });
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

    /// Build the display episode list from a work's scalars and its (optional)
    /// Jikan catalogue. Pure: the shared `kitsune_core::episodes` rules do the
    /// work, so the Qt app and the web app cannot disagree.
    fn build_episodes(
        self: Pin<&mut Self>,
        episode_count: i32,
        status: QString,
        cover_image: QString,
        info_json: QString,
    ) -> QString {
        let info_json = info_json.to_string();
        let info: Vec<EpisodeInfo> = serde_json::from_str(&info_json).unwrap_or_default();

        let cover = cover_image.to_string();
        let status = status.to_string();
        let episodes = kitsune_core::episodes::episodes_for_facts(
            (episode_count > 0).then_some(episode_count as u32),
            (!status.is_empty()).then_some(status.as_str()),
            (!cover.is_empty()).then_some(cover.as_str()),
            &info,
            kitsune_core::episodes::now_unix(),
        );
        let json = serde_json::to_string(&episodes).unwrap_or_else(|_| "[]".into());
        QString::from(&json)
    }

    /// The title forms worth searching an indexer for. Pure.
    fn title_forms(
        self: Pin<&mut Self>,
        user_preferred: QString,
        english: QString,
        romaji: QString,
    ) -> QString {
        let title = Title {
            user_preferred: non_empty(user_preferred),
            english: non_empty(english),
            romaji: non_empty(romaji),
            native: None,
        };
        let json = serde_json::to_string(&title.title_forms()).unwrap_or_else(|_| "[]".into());
        QString::from(&json)
    }

    /// Choose the file to play from a torrent's files. Pure.
    ///
    /// The episode match wins; otherwise a sole video starts automatically; a
    /// genuine choice returns `-1` so the reader picks.
    fn pick_file(self: Pin<&mut Self>, files_json: QString, episode: i32, offset: i32) -> i32 {
        use kitsune_core::player::state::TorrentFile;
        use kitsune_core::torrent_files;

        let files_json = files_json.to_string();
        let files: Vec<TorrentFile> = match serde_json::from_str(&files_json) {
            Ok(files) => files,
            Err(_) => return -1,
        };

        if episode > 0 {
            if let Some(file) =
                torrent_files::file_for_episode(&files, episode as u32, offset.max(0) as u32)
            {
                return file.idx as i32;
            }
        }

        match torrent_files::first_target_file(&files) {
            Some(file) => file.idx as i32,
            None => -1,
        }
    }

    // --- Releases & torrents ----------------------------------------------

    fn search_releases(
        self: Pin<&mut Self>,
        titles: QString,
        episode: i32,
        absolute_episode: i32,
        mode: QString,
    ) {
        let titles = titles.to_string();
        let mode = mode.to_string();

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let indexers = Arc::clone(&self.rust().indexers);
        let settings = Arc::clone(&self.rust().settings);

        runtime.spawn(async move {
            let titles: Vec<String> = match serde_json::from_str(&titles) {
                Ok(titles) => titles,
                Err(err) => {
                    let message = format!("bad titles payload: {err}");
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.releases_failed(QString::from(&message))
                    });
                    return;
                }
            };

            // `<= 0` means "no episode" (a film or a title-level search).
            let episode = (episode > 0).then_some(episode as u32);
            let absolute_episode = (absolute_episode > 0).then_some(absolute_episode as u32);

            let mode: SearchMode = match serde_json::from_value(serde_json::Value::String(mode)) {
                Ok(mode) => mode,
                Err(err) => {
                    let message = format!("unknown search mode: {err}");
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.releases_failed(QString::from(&message))
                    });
                    return;
                }
            };

            let request = ReleaseRequest {
                titles,
                episode,
                absolute_episode,
                mode,
            };

            // The stored preference steers ranking, so the settings page takes
            // effect without every search passing it.
            let preference = {
                let stored = settings.read();
                ReleasePreference {
                    preferred_resolutions: stored.preferred_resolutions,
                    min_seeders: stored.min_seeders,
                }
            };

            let found = search_all(indexers.indexers(), &request, &preference).await;
            let json = serde_json::to_string(&found).unwrap_or_else(|_| "[]".into());
            let _ = qt_thread.queue(move |qobject| qobject.releases_ready(QString::from(&json)));
        });
    }

    fn add_magnet(self: Pin<&mut Self>, magnet_uri: QString) {
        let magnet_uri = magnet_uri.to_string();

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            match player.add_magnet(&magnet_uri).await {
                Ok(handle) => {
                    let json = serde_json::to_string(&handle).unwrap_or_else(|_| "null".into());
                    let _ = qt_thread
                        .queue(move |qobject| qobject.torrent_added(QString::from(&json)));
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread
                        .queue(move |qobject| qobject.torrent_add_failed(QString::from(&message)));
                }
            }
        });
    }

    fn remove_torrent(self: Pin<&mut Self>, torrent_id: i64) {
        let id = torrent_id as usize;
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            if let Err(err) = player.remove_torrent(id).await {
                tracing::warn!("failed to remove torrent {id}: {err:#}");
            }
        });
    }

    fn set_only_files(self: Pin<&mut Self>, torrent_id: i64, file_idx: i64) {
        let id = torrent_id as usize;
        let file = file_idx as usize;
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            // Non-fatal: librqbit rejects this while a torrent is still
            // initializing, and playback can proceed with the default order.
            if let Err(err) = player.set_only_files(id, vec![file]).await {
                tracing::warn!("set_only_files for torrent {id} failed: {err:#}");
            }
        });
    }

    fn get_stream_url(self: Pin<&mut Self>, torrent_id: i64, file_idx: i64) {
        let id = torrent_id as usize;
        let file = file_idx as usize;
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            match player.stream_url(id, file).await {
                Ok(url) => {
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.stream_url_ready(id as i64, file as i64, QString::from(&url))
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread
                        .queue(move |qobject| qobject.stream_url_failed(QString::from(&message)));
                }
            }
        });
    }

    fn pause_torrent(self: Pin<&mut Self>, torrent_id: i64) {
        let id = torrent_id as usize;
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            if let Err(err) = player.pause_torrent(id).await {
                tracing::warn!("failed to pause torrent {id}: {err:#}");
            }
        });
    }

    fn resume_torrent(self: Pin<&mut Self>, torrent_id: i64) {
        let id = torrent_id as usize;
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            if let Err(err) = player.resume_torrent(id).await {
                tracing::warn!("failed to resume torrent {id}: {err:#}");
            }
        });
    }

    fn get_torrent_stats(self: Pin<&mut Self>, torrent_id: i64) {
        // Synchronous: a snapshot read from the live session, no await.
        let progress = self.rust().player.torrent_progress(torrent_id as usize);
        let json = serde_json::to_string(&progress).unwrap_or_else(|_| "null".into());
        self.torrent_stats_ready(QString::from(&json));
    }

    fn hold_torrent(self: Pin<&mut Self>, torrent_id: i64) {
        // Synchronous and cheap: takes the hold before any removal can race it.
        self.rust().player.hold_torrent(torrent_id as usize);
    }

    fn release_torrent(self: Pin<&mut Self>, torrent_id: i64) {
        let id = torrent_id as usize;
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            if let Err(err) = player.release_torrent(id).await {
                tracing::warn!("failed to release torrent {id}: {err:#}");
            }
        });
    }

    // --- Account -----------------------------------------------------------

    /// The URL to open in a browser to authorise Kitsune.
    ///
    /// Synchronous: it only builds a string, no network.
    fn begin_login(self: Pin<&mut Self>) -> QString {
        QString::from(flow::authorize_url())
    }

    fn auth_status(self: Pin<&mut Self>) {
        let signed_in = self.rust().tokens.has_token();
        self.auth_status_ready(signed_in);
    }

    fn logout(self: Pin<&mut Self>) {
        // Clear the store, then the live provider, so signing out takes effect
        // on the next request rather than at the next restart.
        if let Err(err) = self.rust().tokens.clear() {
            let message = err.to_string();
            self.auth_error(QString::from(&message));
            return;
        }
        self.rust().anilist.clear_token();
        self.auth_status_ready(false);
    }

    /// Run a concrete-`AniListProvider` task on the runtime and deliver the
    /// result back on the Qt thread. The counterpart of `spawn_metadata` for
    /// account operations that are not on the `AnimeProvider` trait.
    fn spawn_anilist<M, Fut>(
        self: Pin<&mut Self>,
        work: M,
        ok: fn(Pin<&mut qobject::Backend>, QString),
        err: fn(Pin<&mut qobject::Backend>, QString),
    ) where
        M: FnOnce(Arc<AniListProvider>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<String, String>> + Send + 'static,
    {
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let anilist = Arc::clone(&self.rust().anilist);

        runtime.spawn(async move {
            match work(anilist).await {
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

    fn continue_watching(self: Pin<&mut Self>, limit: i32) {
        let limit = limit_or_default(limit);
        self.spawn_anilist(
            move |anilist| async move {
                anilist
                    .continue_watching(limit)
                    .await
                    .map(|items| json_or_empty(&items))
                    .map_err(|err| err.to_string())
            },
            Self::continue_watching_ready,
            Self::continue_watching_failed,
        );
    }

    fn user_list(self: Pin<&mut Self>) {
        self.spawn_anilist(
            move |anilist| async move {
                anilist
                    .user_list()
                    .await
                    .map(|entries| json_or_empty(&entries))
                    .map_err(|err| err.to_string())
            },
            Self::user_list_ready,
            Self::user_list_failed,
        );
    }

    fn set_list_entry(self: Pin<&mut Self>, media_id: i64, status: QString, progress: i32) {
        let status = status.to_string();
        // `progress <= 0` means "leave progress alone" (the Tauri command's
        // `Option<u32>` None); sending 0 would reset a tracked position.
        let progress = if progress > 0 { Some(progress as u32) } else { None };

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let anilist = Arc::clone(&self.rust().anilist);

        runtime.spawn(async move {
            let parsed: ListStatus = match serde_json::from_value(serde_json::Value::String(
                status,
            )) {
                Ok(s) => s,
                Err(err) => {
                    let message = format!("unknown list status: {err}");
                    let _ = qt_thread
                        .queue(move |qobject| qobject.auth_error(QString::from(&message)));
                    return;
                }
            };

            match anilist.save_list_entry(media_id, parsed, progress).await {
                Ok(()) => {
                    // Only after the write succeeded: a failed save did not
                    // change the list, so telling the UI to re-read would lie.
                    let _ = qt_thread.queue(move |qobject| qobject.list_changed());
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread
                        .queue(move |qobject| qobject.auth_error(QString::from(&message)));
                }
            }
        });
    }

    fn delete_list_entry(self: Pin<&mut Self>, entry_id: i64) {
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let anilist = Arc::clone(&self.rust().anilist);

        runtime.spawn(async move {
            match anilist.delete_list_entry(entry_id).await {
                Ok(()) => {
                    let _ = qt_thread.queue(move |qobject| qobject.list_changed());
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread
                        .queue(move |qobject| qobject.auth_error(QString::from(&message)));
                }
            }
        });
    }

    fn get_list_entry(self: Pin<&mut Self>, media_id: i64) {
        self.spawn_anilist(
            move |anilist| async move {
                anilist
                    .list_entry(media_id)
                    .await
                    // `None` -> JSON `null`, which QML reads as "not on list".
                    .map(|entry| serde_json::to_string(&entry).unwrap_or_else(|_| "null".into()))
                    .map_err(|err| err.to_string())
            },
            Self::list_entry_ready,
            Self::auth_error,
        );
    }

    // --- Settings ----------------------------------------------------------

    /// Both settings commands are synchronous file reads/writes, so they run
    /// on the Qt thread directly rather than through the runtime.
    fn get_settings(self: Pin<&mut Self>) {
        let settings = self.rust().settings.read();
        let json = serde_json::to_string(&settings).unwrap_or_else(|_| "null".to_string());
        self.settings_ready(QString::from(&json));
    }

    fn set_settings(self: Pin<&mut Self>, json: QString) {
        let json = json.to_string();
        let parsed: Settings = match serde_json::from_str(&json) {
            Ok(s) => s,
            Err(err) => {
                let message = format!("bad settings payload: {err}");
                self.settings_failed(QString::from(&message));
                return;
            }
        };

        if let Err(err) = self.rust().settings.write(&parsed) {
            let message = err.to_string();
            self.settings_failed(QString::from(&message));
            return;
        }

        // Emit what was actually persisted, so a clamped value (the auto-launch
        // threshold) is visible without a second read.
        let stored = self.rust().settings.read();
        let out = serde_json::to_string(&stored).unwrap_or_else(|_| "null".to_string());
        self.settings_ready(QString::from(&out));
    }
}

/// Serialise to a JSON array, falling back to `[]` so a serialisation failure
/// never drops the whole response (the UI already handles an empty list).
fn json_or_empty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}

/// A non-blank `QString` as an `Option<String>`, so an empty FFI string reads
/// as "absent" rather than as a present-but-empty value.
fn non_empty(value: QString) -> Option<String> {
    let value = value.to_string();
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}