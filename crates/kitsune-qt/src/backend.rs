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

        /// One page of a curated list, as a JSON `AnimePage`
        /// (`{ items, pageInfo }`). Backs the scrolling "Top 100" view: the
        /// `pageInfo` says whether another page exists. Emits `listReady`.
        #[qinvokable]
        #[cxx_name = "getListPaged"]
        fn get_list_paged(self: Pin<&mut Self>, filter: QString, page: i32, per_page: i32);

        /// Every home shelf (trending + the five curated blocks) in ONE
        /// request. Emits `homeReady(json)` with a `HomeShelves` object, or
        /// `homeFailed(message)`.
        #[qinvokable]
        #[cxx_name = "getHome"]
        fn get_home(self: Pin<&mut Self>);

        /// Browse with filters and paging. `query` is a JSON `BrowseQuery`.
        #[qinvokable]
        #[cxx_name = "getBrowse"]
        fn get_browse(self: Pin<&mut Self>, query: QString, page: i32, per_page: i32);

        /// Look up a single title. Emits JSON `null` when the id is unknown.
        /// One work's full detail.
        ///
        /// `provider` is the id's NAMESPACE ("anilist" / "kitsu"), taken from
        /// the card the reader clicked. Ids are per-source, so a bare number is
        /// ambiguous and the detail MUST be fetched from the source that
        /// issued it -- asking AniList for a Kitsu id returns the wrong work.
        #[qinvokable]
        #[cxx_name = "getAnime"]
        fn get_anime(self: Pin<&mut Self>, id: i64, provider: QString);

        /// A page of recommendations for one work.
        #[qinvokable]
        #[cxx_name = "getRecommendations"]
        fn get_recommendations(self: Pin<&mut Self>, id: i64, page: i32, per_page: i32);

        /// Cast the reader's vote on a recommendation, returning the new
        /// tally. `rating` is the serde form of `RecommendationRating`
        /// ("rateUp" | "rateDown" | "noRating") -- NOT AniList's GraphQL
        /// literal, which the enum produces on the way out. AniList keys a
        /// recommendation by the (base, recommended) PAIR, so both ids travel.
        /// Emits `recommendationRated` (success) or `recommendationRateFailed`.
        #[qinvokable]
        #[cxx_name = "rateRecommendation"]
        fn rate_recommendation(
            self: Pin<&mut Self>,
            media_id: i64,
            recommended_id: i64,
            rating: QString,
        );

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

        /// Like `getSchedule`, but echoes `tag` back on the answer. The home
        /// page's schedule widget uses this: the home page never unmounts, so
        /// an untagged reply from a standalone SchedulePage would overwrite it.
        #[qinvokable]
        #[cxx_name = "getScheduleTagged"]
        fn get_schedule_tagged(self: Pin<&mut Self>, tag: QString, from: i64, to: i64, limit: i32);

        /// Full episode metadata for a work.
        ///
        /// `provider` is the `Anime.provider` string ("anilist", "kitsu"):
        /// ids live in per-provider namespaces, so an id alone is ambiguous.
        /// Passing "kitsu" uses it directly; anything else is treated as an
        /// AniList id and bridged through Kitsu's mapping table.
        #[qinvokable]
        #[cxx_name = "getEpisodes"]
        fn get_episodes(self: Pin<&mut Self>, id: i64, provider: QString);

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
        fn pick_file(self: Pin<&mut Self>, files_json: QString, episode: i32, offset: i32) -> i32;

        /// Measure latency and download throughput against Cloudflare. Runs
        /// asynchronously; progress arrives on `speedTestProgress`, the final
        /// numbers on `speedTestReady`. A second call while one is running is
        /// rejected on `speedTestFailed`.
        #[qinvokable]
        #[cxx_name = "runSpeedTest"]
        fn run_speed_test(self: Pin<&mut Self>);

        /// Search the indexers for a work's releases. `titles` is a JSON array
        /// of title forms; `mode` is the search-mode wire string.
        ///
        /// `tag` is echoed on `releasesReady` / `releasesFailed`, so a slow
        /// answer for a previous open can be ignored rather than painted into
        /// the modal. `force` skips the cache and searches again -- the Retry
        /// button passes it.
        #[qinvokable]
        #[cxx_name = "searchReleases"]
        fn search_releases(
            self: Pin<&mut Self>,
            tag: QString,
            titles: QString,
            episode: i32,
            absolute_episode: i32,
            mode: QString,
            force: bool,
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

        /// Select `file_idx`, wait until the selection is in force, warm both
        /// ends of the file, and return the URL to play.
        ///
        /// This is the play path: it exists because selecting the file and
        /// resolving the URL have to happen IN ORDER. Fired as two commands
        /// the selection lost its race with the torrent's initial hash and was
        /// silently dropped, so the engine downloaded its own first file while
        /// the reader watched a progress bar climb towards an episode that had
        /// not been fetched at all.
        #[qinvokable]
        #[cxx_name = "prepareStream"]
        fn prepare_stream(self: Pin<&mut Self>, torrent_id: i64, file_idx: i64);

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

        /// Probe a set of releases for swarm health.
        ///
        /// `releases_json` is a JSON `Release[]`, the exact list a search just
        /// returned. One `probeResult` fires per release, in completion order,
        /// so the list can re-rank while slower probes are still running; a
        /// final `probeDone` marks the end. Nothing is returned: the answers
        /// are the signals.
        #[qinvokable]
        #[cxx_name = "probeReleases"]
        fn probe_releases(self: Pin<&mut Self>, tag: QString, releases_json: QString);

        /// Probe one release on demand and return its full health report.
        #[qinvokable]
        #[cxx_name = "probeRelease"]
        fn probe_release(self: Pin<&mut Self>, tag: QString, release_json: QString);

        /// Fetch a `.torrent` from an indexer and write it to `path`.
        ///
        /// The fetch happens here rather than in QML: Nyaa cannot be reached
        /// from a web context (CORS), and a native request writes the bytes
        /// straight to disk.
        #[qinvokable]
        #[cxx_name = "downloadTorrent"]
        fn download_torrent(self: Pin<&mut Self>, url: QString, path: QString);

        // --- Permanent episode downloads -------------------------------------

        /// Start downloading ONE episode for keeps, from a release.
        ///
        /// `release_json` is the `Release` the reader picked; `file_idx` the
        /// file inside it to keep. `season` and `episode` may be `0` when
        /// unknown, which keeps the original torrent file name instead of
        /// building an `SxxEyy` name from numbers we do not have.
        ///
        /// Emits `downloadStarted(json)` with the stored record, or
        /// `downloadFailed(id, message)`.
        ///
        /// `file_indices_json` is a JSON array of file indices. A season pack
        /// passes every file the reader ticked in the picker, which produces
        /// one download record per file so each episode shows up separately on
        /// the Downloads page.
        #[qinvokable]
        #[cxx_name = "startEpisodeDownload"]
        fn start_episode_download(
            self: Pin<&mut Self>,
            anime_id: i64,
            provider: QString,
            episode: i32,
            season: i32,
            title: QString,
            episode_title: QString,
            cover_url: QString,
            release_json: QString,
            file_indices_json: QString,
        );

        /// Resolve a release's file list WITHOUT starting a download.
        ///
        /// Needed by the include/exclude picker: a torrent's file list is only
        /// known once metadata resolves, and the reader must choose from it
        /// before anything is written. Emits `releaseFilesReady(token, json)`.
        ///
        /// The torrent is dropped again as soon as the list is read, so merely
        /// inspecting a release never leaves a transfer running.
        #[qinvokable]
        #[cxx_name = "listReleaseFiles"]
        fn list_release_files(self: Pin<&mut Self>, token: QString, release_json: QString);

        /// Re-read the store and emit `downloadsReady(json)`.
        #[qinvokable]
        #[cxx_name = "getDownloads"]
        fn get_downloads(self: Pin<&mut Self>);

        /// Reattach every incomplete download left from a previous run.
        ///
        /// Runs once per process. Each torrent is restored PAUSED, using cached
        /// `.torrent` metadata when available and the magnet otherwise, so a
        /// restart never silently resumes transfers. Stale process-local torrent
        /// IDs are replaced with the new engine IDs; a group that cannot be
        /// restored is marked Failed with the reason.
        #[qinvokable]
        #[cxx_name = "restoreDownloads"]
        fn restore_downloads(self: Pin<&mut Self>);

        /// List every file in the torrent behind `group_key`.
        ///
        /// Emits `downloadFilesReady(token, json)` where `json` is
        /// `{ groupKey, name, files: [{ idx, name, lengthBytes, selected }] }`.
        /// A group with no live torrent is reattached temporarily; it is dropped
        /// again before the reply, so inspecting files never starts a transfer.
        #[qinvokable]
        #[cxx_name = "listDownloadFiles"]
        fn list_download_files(self: Pin<&mut Self>, token: QString, group_key: QString);

        /// Replace the selected files of the torrent behind `group_key`.
        ///
        /// `indices_json` is a JSON array of file indices. Existing rows for
        /// files no longer selected are removed (their completed library files
        /// are left alone), and newly selected files become rows. Emits
        /// `downloadFilesReady` with the refreshed list, or
        /// `downloadFilesFailed(token, message)`.
        #[qinvokable]
        #[cxx_name = "setDownloadFiles"]
        fn set_download_files(
            self: Pin<&mut Self>,
            token: QString,
            group_key: QString,
            indices_json: QString,
        );

        /// Poll every in-flight download once and emit `downloadProgress(json)`.
        ///
        /// Driven by a QML timer at ~1 Hz; each call also completes any
        /// download whose file has fully arrived.
        #[qinvokable]
        #[cxx_name = "pollDownloads"]
        fn poll_downloads(self: Pin<&mut Self>);

        /// Forget a download, optionally deleting the file from disk.
        #[qinvokable]
        #[cxx_name = "removeDownload"]
        fn remove_download(self: Pin<&mut Self>, id: QString, delete_file: bool);

        /// Pause a running download.
        #[qinvokable]
        #[cxx_name = "pauseDownload"]
        fn pause_download(self: Pin<&mut Self>, id: QString);

        /// Resume a paused download.
        #[qinvokable]
        #[cxx_name = "resumeDownload"]
        fn resume_download(self: Pin<&mut Self>, id: QString);

        /// Change one download's queue priority: "high", "normal", or "low".
        #[qinvokable]
        #[cxx_name = "setDownloadPriority"]
        fn set_download_priority(self: Pin<&mut Self>, id: QString, priority: QString);

        /// The library directory downloads are saved into.
        #[qinvokable]
        #[cxx_name = "libraryDir"]
        fn library_dir(self: Pin<&mut Self>) -> QString;

        /// Absolute path of a completed download, or an empty string.
        ///
        /// QML plays it with the same embedded `MpvObject` the watch page
        /// uses, so a download needs no second player binary.
        #[qinvokable]
        #[cxx_name = "downloadPath"]
        fn download_path(self: Pin<&mut Self>, id: QString) -> QString;

        /// Completed downloads of one work, whose files are still on disk.
        #[qinvokable]
        #[cxx_name = "downloadedForAnime"]
        fn downloaded_for_anime(self: Pin<&mut Self>, anime_id: i64, provider: QString) -> QString;

        /// Open a completed download's folder in the file manager.
        #[qinvokable]
        #[cxx_name = "openDownloadFolder"]
        fn open_download_folder(self: Pin<&mut Self>, id: QString);

        /// Open the library directory itself.
        #[qinvokable]
        #[cxx_name = "openLibraryFolder"]
        fn open_library_folder(self: Pin<&mut Self>);

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
        ///
        /// Deliberately does NOT start a sync. Three pages call this on
        /// startup and the detail page calls it on every `listChanged`, so a
        /// sync here turned one pull into an unbounded chain of them.
        #[qinvokable]
        #[cxx_name = "authStatus"]
        fn auth_status(self: Pin<&mut Self>);

        /// Reconcile the local list with AniList, once, in the background.
        ///
        /// Called at startup and after the reader signs in. Every list WRITE
        /// already spawns its own sync, so this is not needed per edit.
        #[qinvokable]
        #[cxx_name = "syncMediaList"]
        fn sync_media_list(self: Pin<&mut Self>);

        /// Apply a `kitsune://auth#access_token=...` redirect URL.
        ///
        /// Returns whether a token was applied, so a caller can tell "not our
        /// URL" from "applied".
        #[qinvokable]
        #[cxx_name = "applyAuthUrl"]
        fn apply_auth_url(self: Pin<&mut Self>, url: QString) -> bool;

        /// Apply a sign-in that a SECOND launch forwarded over D-Bus.
        ///
        /// The bus service cannot reach this object (it is created by QML, not
        /// by `main`), so it parks the URL in a process-wide slot and this
        /// drains it. QML calls it when the window becomes active, which is
        /// exactly when the reader returns from the browser.
        ///
        /// Returns whether a token was applied. A no-op when nothing is
        /// pending, so it is cheap to call on every activation.
        #[qinvokable]
        #[cxx_name = "takePendingAuthUrl"]
        fn take_pending_auth_url(self: Pin<&mut Self>) -> bool;

        /// Register the D-Bus notification callback.
        ///
        /// Called once from QML at startup. After this, when a deep link
        /// arrives via D-Bus, the bus service invokes the callback which
        /// queues `pendingAuthArrived` on the Qt thread, auto-raising the
        /// window without requiring manual focus.
        #[qinvokable]
        #[cxx_name = "initDeepLinkNotify"]
        fn init_deep_link_notify(self: Pin<&mut Self>);

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
        fn set_list_entry(self: Pin<&mut Self>, media_id: i64, status: QString, progress: i32);

        /// Remove a work from the reader's list.
        #[qinvokable]
        #[cxx_name = "deleteListEntry"]
        fn delete_list_entry(self: Pin<&mut Self>, entry_id: i64);

        /// Where a work sits on the reader's list, or JSON `null`.
        #[qinvokable]
        #[cxx_name = "getListEntry"]
        fn get_list_entry(self: Pin<&mut Self>, media_id: i64);

        // --- Local resume state ----------------------------------------------

        /// Record that `anime_id` was opened, with the namespace it came from.
        ///
        /// `provider` is stored alongside the id: a resume must ask the source
        /// that issued it, and a bare id cannot say which that is.
        #[qinvokable]
        #[cxx_name = "recordLastPlayed"]
        fn record_last_played(self: Pin<&mut Self>, anime_id: i64, episode: i32, provider: QString);

        /// The most recently opened work as JSON, or `null`.
        #[qinvokable]
        #[cxx_name = "getLastPlayed"]
        fn get_last_played(self: Pin<&mut Self>);

        /// Record where the reader stopped in an episode.
        #[qinvokable]
        #[cxx_name = "recordWatchPosition"]
        fn record_watch_position(
            self: Pin<&mut Self>,
            anime_id: i64,
            episode: i32,
            position_seconds: f64,
            duration_seconds: f64,
        );

        /// Every saved watch position as a JSON object keyed by anime id.
        #[qinvokable]
        #[cxx_name = "getWatchPositions"]
        fn get_watch_positions(self: Pin<&mut Self>);

        /// Remember the release last played for a work, so a resume can
        /// reselect it. `file_idx` is the torrent file index that played.
        #[qinvokable]
        #[cxx_name = "recordLastRelease"]
        fn record_last_release(
            self: Pin<&mut Self>,
            anime_id: i64,
            magnet: QString,
            file_idx: i32,
            name: QString,
        );

        /// The remembered release for a work as JSON, or `null`.
        #[qinvokable]
        #[cxx_name = "getLastRelease"]
        fn get_last_release(self: Pin<&mut Self>, anime_id: i64);

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

        /// Emitted when a deep link arrives via D-Bus while the app is running.
        ///
        /// QML connects to this signal to auto-raise the window and drain the
        /// pending URL, so the user doesn't need to manually focus the app.
        #[qsignal]
        #[cxx_name = "pendingAuthArrived"]
        fn pending_auth_arrived(self: Pin<&mut Self>);

        #[qsignal]
        #[cxx_name = "continueWatchingReady"]
        fn continue_watching_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "continueWatchingFailed"]
        fn continue_watching_failed(self: Pin<&mut Self>, message: QString);

        /// The last-played record as JSON, or `"null"`.
        #[qsignal]
        #[cxx_name = "lastPlayedReady"]
        fn last_played_ready(self: Pin<&mut Self>, json: QString);

        /// Every saved watch position as a JSON object keyed by anime id.
        #[qsignal]
        #[cxx_name = "watchPositionsReady"]
        fn watch_positions_ready(self: Pin<&mut Self>, json: QString);

        /// The remembered release for a work as JSON, or `"null"`.
        #[qsignal]
        #[cxx_name = "lastReleaseReady"]
        fn last_release_ready(self: Pin<&mut Self>, json: QString);

        /// Emitted after any local resume-state write (last-played, position,
        /// or release), so the disc and the Continue Watching row re-read
        /// live instead of only on startup.
        #[qsignal]
        #[cxx_name = "libraryChanged"]
        fn library_changed(self: Pin<&mut Self>);

        #[qsignal]
        #[cxx_name = "userListReady"]
        fn user_list_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "userListFailed"]
        fn user_list_failed(self: Pin<&mut Self>, message: QString);

        /// `media_id` echoes the request, so a hidden DetailPage does not adopt
        /// another work's entry -- that silently moved the "Resume Ep N" CTA
        /// and the list-status chip onto the wrong work.
        #[qsignal]
        #[cxx_name = "listEntryReady"]
        fn list_entry_ready(self: Pin<&mut Self>, media_id: i64, json: QString);

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

        /// The home screen's shelves, as a JSON `HomeShelves`. One signal for
        /// the whole screen, since it is one request.
        #[qsignal]
        #[cxx_name = "homeReady"]
        fn home_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "homeFailed"]
        fn home_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "browseReady"]
        fn browse_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "browseFailed"]
        fn browse_failed(self: Pin<&mut Self>, message: QString);

        #[qsignal]
        #[cxx_name = "animeReady"]
        fn anime_ready(self: Pin<&mut Self>, json: QString);
        /// Echoes the request identity for the same reason as
        /// `episodesFailed`: a hidden DetailPage must not adopt another
        /// work's failure and paint an error over a healthy page.
        #[qsignal]
        #[cxx_name = "animeFailed"]
        fn anime_failed(self: Pin<&mut Self>, id: i64, provider: QString, message: QString);

        /// A page of recommendations. `id` echoes the requested work so a
        /// reply for a work the reader has navigated away from is ignored.
        #[qsignal]
        #[cxx_name = "recommendationsReady"]
        fn recommendations_ready(self: Pin<&mut Self>, id: i64, json: QString);
        #[qsignal]
        #[cxx_name = "recommendationsFailed"]
        fn recommendations_failed(self: Pin<&mut Self>, id: i64, message: QString);

        /// The new vote tally after a successful `rateRecommendation`. The
        /// (base, recommended) pair identifies which card to update.
        #[qsignal]
        #[cxx_name = "recommendationRated"]
        fn recommendation_rated(
            self: Pin<&mut Self>,
            media_id: i64,
            recommended_id: i64,
            rating: i32,
        );
        #[qsignal]
        #[cxx_name = "recommendationRateFailed"]
        fn recommendation_rate_failed(self: Pin<&mut Self>, message: QString);

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

        /// The tagged variant: `tag` is the caller's own label, echoed back.
        #[qsignal]
        #[cxx_name = "taggedScheduleReady"]
        fn tagged_schedule_ready(self: Pin<&mut Self>, tag: QString, json: QString);
        #[qsignal]
        #[cxx_name = "taggedScheduleFailed"]
        fn tagged_schedule_failed(self: Pin<&mut Self>, tag: QString, message: QString);

        /// `id` and `provider` ECHO the request, because this is a broadcast:
        /// `StackView` keeps a hidden DetailPage alive, so without them the
        /// page underneath would adopt the reply meant for the page on screen
        /// and redraw its episode grid with the wrong work's catalogue.
        #[qsignal]
        #[cxx_name = "episodesReady"]
        fn episodes_ready(self: Pin<&mut Self>, id: i64, provider: QString, json: QString);
        #[qsignal]
        #[cxx_name = "episodesFailed"]
        fn episodes_failed(self: Pin<&mut Self>, id: i64, provider: QString, message: QString);

        // --- Speed test ------------------------------------------------------

        /// One progress update while a speed test runs, as a JSON
        /// `SpeedTestProgress` (`{phase, percent, latencyMs, downloadMbps}`).
        #[qsignal]
        #[cxx_name = "speedTestProgress"]
        fn speed_test_progress(self: Pin<&mut Self>, json: QString);
        /// The finished measurement, as a JSON `SpeedTestResult`.
        #[qsignal]
        #[cxx_name = "speedTestReady"]
        fn speed_test_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "speedTestFailed"]
        fn speed_test_failed(self: Pin<&mut Self>, message: QString);

        // --- Release / torrent results ---------------------------------------

        #[qsignal]
        #[cxx_name = "releasesReady"]
        fn releases_ready(self: Pin<&mut Self>, tag: QString, json: QString);
        #[qsignal]
        #[cxx_name = "releasesFailed"]
        fn releases_failed(self: Pin<&mut Self>, tag: QString, message: QString);

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

        /// A provisional work the provider already had from a list response,
        /// emitted BEFORE `animeReady` so the detail page can paint at once.
        ///
        /// Carries no `relations` or `recommendations`; those arrive with
        /// `animeReady`, which always supersedes this.
        #[qsignal]
        #[cxx_name = "animePartial"]
        fn anime_partial(self: Pin<&mut Self>, json: QString);

        /// One release's probe verdict. `badge` is "green"|"yellow"|"red",
        /// `combinedScore` the probe-adjusted rank. Fires in completion order.
        #[qsignal]
        #[cxx_name = "probeResult"]
        fn probe_result(
            self: Pin<&mut Self>,
            tag: QString,
            index: i32,
            badge: QString,
            combined_score: i64,
        );

        /// The whole probe batch finished (or could not start). Always fires,
        /// even when every probe failed, so the UI can stop its spinner.
        #[qsignal]
        #[cxx_name = "probeDone"]
        fn probe_done(self: Pin<&mut Self>, tag: QString);

        /// Full result for one on-demand release probe.
        #[qsignal]
        #[cxx_name = "releaseProbeReady"]
        fn release_probe_ready(self: Pin<&mut Self>, tag: QString, json: QString);

        #[qsignal]
        #[cxx_name = "releaseProbeFailed"]
        fn release_probe_failed(self: Pin<&mut Self>, tag: QString, message: QString);

        /// A `.torrent` was written to disk.
        #[qsignal]
        #[cxx_name = "downloadDone"]
        fn download_done(self: Pin<&mut Self>);
        #[qsignal]
        #[cxx_name = "downloadFailed"]
        fn download_failed(self: Pin<&mut Self>, message: QString);

        // --- Permanent downloads ---------------------------------------------

        /// The full list after a change, as a JSON `DownloadRecord[]`.
        #[qsignal]
        #[cxx_name = "downloadsReady"]
        fn downloads_ready(self: Pin<&mut Self>, json: QString);

        /// One or more records changed progress or status.
        #[qsignal]
        #[cxx_name = "downloadProgress"]
        fn download_progress(self: Pin<&mut Self>, json: QString);

        /// A download was accepted and stored.
        #[qsignal]
        #[cxx_name = "downloadStarted"]
        fn download_started(self: Pin<&mut Self>, json: QString);

        /// A download finished and its file is in the library.
        #[qsignal]
        #[cxx_name = "downloadCompleted"]
        fn download_completed(self: Pin<&mut Self>, json: QString);

        /// A download could not start or finish. `id` is empty when the record
        /// never existed.
        #[qsignal]
        #[cxx_name = "episodeDownloadFailed"]
        fn episode_download_failed(self: Pin<&mut Self>, id: QString, message: QString);

        /// The file list for one torrent, after `listDownloadFiles` or a
        /// successful `setDownloadFiles`.
        #[qsignal]
        #[cxx_name = "downloadFilesReady"]
        fn download_files_ready(self: Pin<&mut Self>, token: QString, json: QString);

        /// A download's file list could not be read or changed.
        #[qsignal]
        #[cxx_name = "downloadFilesFailed"]
        fn download_files_failed(self: Pin<&mut Self>, token: QString, message: QString);

        /// A release's file list, for the include/exclude picker.
        ///
        /// `token` is echoed from `listReleaseFiles` so a slow reply for a
        /// previously opened release can be ignored. `json` is a
        /// `{ name, files: [{ idx, name, lengthBytes }] }` object.
        #[qsignal]
        #[cxx_name = "releaseFilesReady"]
        fn release_files_ready(self: Pin<&mut Self>, token: QString, json: QString);

        /// The file list could not be read (bad magnet, no metadata, timeout).
        #[qsignal]
        #[cxx_name = "releaseFilesFailed"]
        fn release_files_failed(self: Pin<&mut Self>, token: QString, message: QString);
    }

    // Enables `self.qt_thread()` and the `queue(...)` used to hop back onto the
    // Qt event loop from a tokio worker.
    impl cxx_qt::Threading for Backend {}
}

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use kitsune_core::auth::LastPlayedStore;
use kitsune_core::auth::{flow, TokenStore};
use kitsune_core::diagnostics::speedtest::{SpeedTestProgress, SpeedTester};
use kitsune_core::downloads::{
    cached_torrent_path, plan_queue, remove_cached_torrent, remove_download_cache,
    save_cached_torrent, DownloadPriority, DownloadRecord, DownloadStatus, DownloadsStore,
    QueueAction,
};
use kitsune_core::indexer::{
    download_to, probe_rank, probe_releases_standalone, rank, HealthBadge, IndexerRegistry,
    ProbeOutcome, ReleaseCache, ReleaseRequest, SearchMode,
};
use kitsune_core::list::{ListSync, MediaListStore};
use kitsune_core::player::state::PlayerState;
use kitsune_core::player::{LastReleaseStore, WatchPositionStore};
use kitsune_core::providers::cache::ttl;
use kitsune_core::providers::{
    AniListProvider, AnimeProvider, CachedProvider, FailoverProvider, KitsuProvider,
};
use kitsune_core::settings::{MetadataSource, Settings, SettingsStore};
use kitsune_core::types::{
    BrowseQuery, EpisodeInfo, ListFilter, ListStatus, ProviderId, RecommendationRating, Release,
    ReleasePreference, Title,
};

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

    /// The metadata fallback, held concretely because its episode catalogue is
    /// not on the `AnimeProvider` trait.
    ///
    /// Jikan used to fill this role; it is gone, and Kitsu's
    /// `/anime/{id}/episodes` is now the only source that carries per-episode
    /// stills. Pointing at the SAME instance the failover decorator wraps keeps
    /// one connection pool.
    kitsu: Arc<KitsuProvider>,
    /// The AniList token, read from disk at startup so a signed-in reader
    /// stays signed in across restarts.
    tokens: Arc<TokenStore>,
    /// The reader's preferences, read and written by the Qt app.
    settings: Arc<SettingsStore>,
    /// The indexers a release search fans out to.
    indexers: Arc<IndexerRegistry>,
    /// A TTL cache in front of `search_all`, so reopening an episode serves
    /// the list and its probe badges without another round trip. Empty results
    /// are not kept, so Retry always does real work.
    releases: Arc<ReleaseCache>,
    /// The torrent engine + stream bridge + player holds, started lazily on
    /// the first playback. Shares the settings store so the reader's download
    /// directory and player preferences are honoured.
    player: Arc<PlayerState>,
    /// The work the reader most recently opened, for the Continue Watching
    /// sort and the resume disc.
    last_played: Arc<LastPlayedStore>,
    /// Where the reader stopped in each work, for MM:SS and precise resume.
    watch_positions: Arc<WatchPositionStore>,
    /// The release last played per work, so a resume can reselect it.
    last_releases: Arc<LastReleaseStore>,
    /// The permanent episode downloads and their status.
    ///
    /// `None` when the store could not be opened (no data directory, or an
    /// unwritable one). Every download command then reports that rather than
    /// panicking, so a broken environment cannot take the whole app down.
    downloads: Option<Arc<DownloadsStore>>,
    /// The reader's media list, kept locally.
    ///
    /// Every list read is answered from here, so the pages work while AniList is
    /// down and for a reader who has never signed in.
    media_list: Arc<MediaListStore>,
    /// Pushes local list changes to AniList and pulls the server's view back.
    ///
    /// Held as one unit so the store and the engine cannot disagree about which
    /// directory they are reconciling.
    list_sync: Arc<ListSync>,
    /// Whether a sync pass is already running, so overlapping triggers collapse
    /// into one. Three pages call `authStatus()` on startup; without this each
    /// started its own pull of the same list.
    sync_in_flight: Arc<AtomicBool>,
    /// The internet speed tester. Owns its pooled client and a "one run at a
    /// time" guard, so a second click cannot split the bandwidth and
    /// under-report both runs.
    ///
    /// `None` when the HTTP client could not be built (a broken TLS setup).
    /// The command then reports that instead of crashing at startup, and every
    /// other feature keeps working -- the speed test is a diagnostic, not a
    /// prerequisite.
    speed_tester: Option<Arc<SpeedTester>>,
    /// Whether the startup reattach pass has been started.
    ///
    /// The Downloads page is created lazily and can call `restoreDownloads`
    /// again; the pass must not add the same torrent twice.
    downloads_restore_started: Arc<AtomicBool>,
    /// Whether every startup reattach has finished.
    ///
    /// Polling is suspended until then: records still carry stale engine IDs,
    /// and a poll that raced the reattach would mark a resumable group Failed.
    downloads_restore_done: Arc<AtomicBool>,
    /// Group keys whose reattach is still running.
    ///
    /// `pollDownloads` skips these: before the new engine ID is stored, the
    /// record still points at a stale ID, and the "torrent vanished" branch
    /// would otherwise mark a perfectly resumable download as Failed.
    restoring_downloads: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
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

        // The second metadata source. Reads fall back to it when AniList is
        // rate limited or down; writes never go near it, because it has no
        // AniList token and its ids live in a namespace of their own.
        let kitsu = Arc::new(KitsuProvider::new());

        // Which source is asked FIRST. Both are always available -- the
        // difference is only whether a reader who has given up on AniList
        // waits for its timeout before Kitsu answers.
        let source = settings.read().metadata_source();
        let primary: Arc<dyn AnimeProvider> = match source {
            MetadataSource::AniList => anilist.clone(),
            MetadataSource::Kitsu => kitsu.clone(),
        };
        let secondary: Arc<dyn AnimeProvider> = match source {
            MetadataSource::AniList => kitsu.clone(),
            MetadataSource::Kitsu => anilist.clone(),
        };

        // The metadata path goes through the cache, so repeated reads (back to
        // a title already opened, the home rail re-rendering) do not hit
        // AniList again. The concrete `anilist` handle stays UNWRAPPED for
        // account writes -- a saved list entry or vote must never be served
        // from cache.
        let provider: Arc<dyn AnimeProvider> = Arc::new(CachedProvider::new(Arc::new(
            FailoverProvider::new(primary, secondary),
        )));

        // The local list is the source of truth for every READ. `ListSync`
        // pulls AniList's view directly, so no list cache sits in between -- a
        // cache here would only risk serving a stale copy of the reader's own
        // edits back to them.
        let media_list = Arc::new(MediaListStore::new());
        let list_sync = Arc::new(ListSync::new(
            anilist.clone(),
            Some(kitsu.clone()),
            media_list.clone(),
        ));

        Self {
            runtime: Arc::new(runtime),
            provider,
            anilist,
            last_played: Arc::new(LastPlayedStore::new()),
            watch_positions: Arc::new(WatchPositionStore::new()),
            last_releases: Arc::new(LastReleaseStore::new()),
            downloads: match DownloadsStore::new() {
                Ok(store) => Some(Arc::new(store)),
                Err(err) => {
                    tracing::warn!("downloads store unavailable: {err:#}");
                    None
                }
            },
            media_list,
            list_sync,
            sync_in_flight: Arc::new(AtomicBool::new(false)),
            kitsu,
            tokens,
            player: Arc::new(PlayerState::with_settings(settings.clone())),
            settings,
            indexers: Arc::new(IndexerRegistry::new()),
            releases: Arc::new(ReleaseCache::new()),
            // A bad client build is not worth failing startup over: it needs
            // a TLS backend, which is present in every shipped build, so a
            // failure here means the environment is broken. `None` lets the
            // command report that instead of panicking at startup.
            speed_tester: match SpeedTester::new() {
                Ok(tester) => Some(Arc::new(tester)),
                Err(err) => {
                    tracing::warn!("speed test unavailable: {err:#}");
                    None
                }
            },
            downloads_restore_started: Arc::new(AtomicBool::new(false)),
            downloads_restore_done: Arc::new(AtomicBool::new(false)),
            restoring_downloads: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }
}

/// Every record belonging to one torrent group.
fn group_records(records: &[DownloadRecord], group_key: &str) -> Vec<DownloadRecord> {
    records
        .iter()
        .filter(|record| record.group_key() == group_key)
        .cloned()
        .collect()
}

/// The group a row belongs to. `None` when the id no longer exists.
fn group_key_for_id(records: &[DownloadRecord], id: u64) -> Option<String> {
    records
        .iter()
        .find(|record| record.id == id)
        .map(DownloadRecord::group_key)
}

/// The info hash for a group, preferring a persisted one and falling back to
/// the magnet for records written before the field existed.
fn group_info_hash(records: &[DownloadRecord]) -> Option<String> {
    records
        .iter()
        .find_map(|record| {
            record
                .info_hash
                .as_deref()
                .filter(|hash| !hash.trim().is_empty())
                .map(str::to_string)
        })
        .or_else(|| {
            records
                .iter()
                .filter_map(|record| record.magnet.as_deref())
                .find_map(|magnet| kitsune_core::torrent::parse_info_hash(magnet).ok())
        })
}

/// The first non-empty magnet in a group.
fn group_magnet(records: &[DownloadRecord]) -> Option<String> {
    records.iter().find_map(|record| {
        record
            .magnet
            .clone()
            .filter(|magnet| !magnet.trim().is_empty())
    })
}

/// Distinct engine IDs referenced by a group, in encounter order.
fn unique_torrent_ids(records: &[DownloadRecord]) -> Vec<usize> {
    let mut ids = Vec::new();
    for id in records.iter().filter_map(|record| record.torrent_id) {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// The union of every file index selected in a group.
fn group_indices(records: &[DownloadRecord]) -> Vec<usize> {
    let mut indices: Vec<usize> = records
        .iter()
        .map(|record| record.file_idx as usize)
        .collect();
    indices.sort_unstable();
    indices.dedup();
    indices
}

/// The torrent's file count for a group, or the number of known rows.
fn group_file_count(records: &[DownloadRecord], group_key: &str) -> u32 {
    records
        .iter()
        .filter(|record| record.group_key() == group_key)
        .filter_map(|record| record.torrent_file_count)
        .max()
        .unwrap_or_else(|| {
            records
                .iter()
                .filter(|record| record.group_key() == group_key)
                .count() as u32
        })
}

/// The cache directory for a group.
///
/// Newer records persist the exact per-torrent folder. A record written before
/// that field existed kept its partial data directly in the configured download
/// directory (the session root), so restore falls back to that root instead of
/// inventing a new per-hash folder and redownloading everything.
fn group_cache_dir(records: &[DownloadRecord], player: &PlayerState) -> std::path::PathBuf {
    records
        .iter()
        .find_map(|record| {
            record
                .cache_dir
                .as_deref()
                .filter(|dir| !dir.trim().is_empty())
                .map(std::path::PathBuf::from)
        })
        .unwrap_or_else(|| player.resolved_download_dir())
}

/// Whether a record should be reattached at startup.
fn needs_restore(record: &DownloadRecord) -> bool {
    matches!(
        record.status,
        DownloadStatus::Downloading | DownloadStatus::Queued | DownloadStatus::Paused
    )
}

/// Record that a group's reattach is running, without holding the lock across
/// an await.
fn mark_restoring(
    restoring: &std::sync::Mutex<std::collections::HashSet<String>>,
    group_key: &str,
) {
    restoring
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(group_key.to_string());
}

fn unmark_restoring(
    restoring: &std::sync::Mutex<std::collections::HashSet<String>>,
    group_key: &str,
) {
    restoring
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(group_key);
}

fn is_restoring(
    restoring: &std::sync::Mutex<std::collections::HashSet<String>>,
    group_key: &str,
) -> bool {
    restoring
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains(group_key)
}

/// A torrent reattached from cached metadata or its magnet.
struct AttachedGroup {
    torrent_id: usize,
    files: Vec<kitsune_core::player::TorrentFile>,
    info_hash: String,
    cache_dir: std::path::PathBuf,
}

/// Reattach one group's torrent, preferring cached `.torrent` metadata.
///
/// `paused` is true for the startup restore and for temporary inspection; the
/// add path leaves it false so a resumed group keeps transferring.
async fn attach_group(
    player: &PlayerState,
    records: &[DownloadRecord],
    paused: bool,
    indices: &[usize],
) -> Result<AttachedGroup, String> {
    if indices.is_empty() {
        return Err("the group has no selected files".to_string());
    }

    let info_hash = group_info_hash(records)
        .ok_or_else(|| "no info hash or magnet for this torrent".to_string())?;
    let cache_dir = group_cache_dir(records, player);
    let cached = cached_torrent_path(&info_hash);
    let magnet = group_magnet(records);

    if cached.is_none() && magnet.is_none() {
        return Err("no cached metadata and no magnet to restore from".to_string());
    }

    let started = player
        .start_download_in(
            if cached.is_some() {
                None
            } else {
                magnet.as_deref()
            },
            cached.as_deref(),
            indices,
            Some(&cache_dir),
            paused,
        )
        .await
        .map_err(|err| err.to_string())?;

    let files = player.torrent_files(started);
    if files.is_empty() {
        return Err("the torrent did not expose any files".to_string());
    }

    Ok(AttachedGroup {
        torrent_id: started,
        files,
        info_hash,
        cache_dir,
    })
}

/// A group's live torrent and file list, when the engine still knows it.
fn live_files(
    player: &PlayerState,
    records: &[DownloadRecord],
) -> Option<(usize, Vec<kitsune_core::player::TorrentFile>)> {
    for torrent_id in unique_torrent_ids(records) {
        let files = player.torrent_files(torrent_id);
        if !files.is_empty() {
            return Some((torrent_id, files));
        }
    }
    None
}

/// Cache metadata from a freshly attached torrent, when it is available.
fn save_attached_metadata(player: &PlayerState, attached: &AttachedGroup) {
    if let Some(bytes) = player.torrent_bytes(attached.torrent_id) {
        if let Err(err) = save_cached_torrent(&attached.info_hash, &bytes) {
            tracing::warn!(
                "could not cache torrent metadata for {}: {err:#}",
                attached.info_hash
            );
        }
    }
}

/// Records refreshed from the torrent engine while a download runs.
///
/// Pure, so the store -> JSON mapping and the status transitions are unit
/// tested without a runtime, a torrent, or the Qt event loop.
fn downloads_json(records: &[DownloadRecord]) -> String {
    // Same shape as `download_progress`, so a store refresh after pause,
    // priority or a file edit keeps the fields the grouped page needs. A plain
    // record list has no `groupKey`/`fileCount`; QML would have to guess and
    // would briefly relabel a multi-file torrent as one file.
    let views: Vec<DownloadView> = records
        .iter()
        .map(|record| {
            let file_count = group_file_count(records, &record.group_key());
            DownloadView::idle(record.clone(), file_count, false)
        })
        .collect();
    views_json(&views)
}

/// Seconds until ONE file completes, from the whole-torrent rate.
///
/// librqbit reports one download speed and one ETA per torrent. On a season
/// pack that ETA covers every selected file, so printing it on episode 3 is off
/// by hours. Scaling by this file's remaining bytes against the torrent's
/// remaining bytes gives a usable estimate at the same rate.
///
/// `None` when the rate is zero (nothing is arriving, so there is no estimate)
/// or the file is already complete.
fn file_eta(
    record: &DownloadRecord,
    progress: &kitsune_core::torrent::TorrentProgress,
) -> Option<u64> {
    let bytes_per_second = progress.download_mbps * 1024.0 * 1024.0;
    if bytes_per_second <= 0.0 {
        return None;
    }

    let file_done = progress
        .file_progress
        .get(record.file_idx as usize)
        .copied()
        .unwrap_or(0);
    let remaining = record.total_bytes.saturating_sub(file_done);
    if remaining == 0 {
        return None;
    }

    Some((remaining as f64 / bytes_per_second) as u64)
}

/// A download record plus the live numbers the page shows.
///
/// Flattened rather than nested so the QML shape stays exactly the record it
/// already reads, with four extra keys -- no `view.record.title` churn in every
/// binding.
///
/// The live fields are deliberately NOT on `DownloadRecord`: that one is
/// persisted, and a speed from five minutes ago in `downloads.json` would be a
/// lie the page could not tell from a live one.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadView {
    #[serde(flatten)]
    record: DownloadRecord,
    /// Identity shared by every file of this torrent, for grouping in QML.
    group_key: String,
    /// Files the torrent contained, when known.
    file_count: u32,
    /// True while this group is being reattached after a restart.
    restoring: bool,
    /// Current download rate in MiB/s. Zero when not live.
    speed_mbps: f64,
    /// Peers currently transferring pieces.
    peers_live: u32,
    /// Seconds until this file completes, when derivable.
    eta_seconds: Option<u64>,
}

impl DownloadView {
    /// A record with no live data (queued, complete, failed).
    fn idle(record: DownloadRecord, file_count: u32, restoring: bool) -> Self {
        Self {
            group_key: record.group_key(),
            record,
            file_count,
            restoring,
            speed_mbps: 0.0,
            peers_live: 0,
            eta_seconds: None,
        }
    }
}

/// The view list, for the page.
fn views_json(views: &[DownloadView]) -> String {
    serde_json::to_string(views).unwrap_or_else(|_| "[]".into())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadedView {
    record_id: u64,
    episode: Option<u32>,
    release_title: String,
    magnet: Option<String>,
    torrent_url: Option<String>,
    file_name: String,
    size_bytes: u64,
}

fn downloaded_views_for(
    records: &[DownloadRecord],
    anime_id: i64,
    provider: &str,
) -> Vec<DownloadedView> {
    let provider = provider.trim().to_ascii_lowercase();
    records
        .iter()
        .filter(|record| {
            record.anime_id == anime_id
                && record.provider.eq_ignore_ascii_case(&provider)
                && record.status == DownloadStatus::Complete
                && !record.output_path.is_empty()
                && std::fs::metadata(&record.output_path).is_ok()
        })
        .map(|record| DownloadedView {
            record_id: record.id,
            episode: record.episode,
            release_title: record.release_title.clone(),
            magnet: record.magnet.clone(),
            torrent_url: record.torrent_url.clone(),
            file_name: record.file_name.clone(),
            size_bytes: record.total_bytes,
        })
        .collect()
}

/// A torrent's file list, with this group's selected indices marked.
fn download_files_json(
    group_key: &str,
    name: &str,
    files: &[kitsune_core::player::TorrentFile],
    selected: &std::collections::HashSet<usize>,
) -> String {
    let files: Vec<serde_json::Value> = files
        .iter()
        .map(|file| {
            serde_json::json!({
                "idx": file.idx,
                "name": file.name,
                "lengthBytes": file.length_bytes,
                "selected": selected.contains(&file.idx),
            })
        })
        .collect();
    serde_json::to_string(&serde_json::json!({
        "groupKey": group_key,
        "name": name,
        "files": files,
    }))
    .unwrap_or_else(|_| "null".into())
}

/// The episode number a file name announces, if any.
///
/// A season pack lists `Show - 01.mkv`, `Show - 02.mkv`, ... and each record
/// must carry its OWN number, or the Downloads page shows seven rows all
/// claiming to be episode 1. Delegates to the indexer's parser so a file name
/// is read exactly like a release name, with the same "delimited token"
/// discipline that stops `1080p` matching as episode 1080.
fn episode_from_filename(file_name: &str) -> Option<u32> {
    let parsed = kitsune_core::indexer::parse::parse_release(file_name);
    parsed.episode
}

/// Parse a QML-supplied download id (`""` or garbage becomes `0`).
fn download_id(raw: &str) -> u64 {
    raw.trim().parse::<u64>().unwrap_or(0)
}

/// Unix seconds, for the store's timestamps.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Open `path` with the desktop's file manager.
///
/// Used for "show in folder", so it opens the CONTAINING directory rather than
/// the file: the reader wants to see it next to the rest of the library. A
/// spawn failure is logged, not reported -- there is no useful action the UI
/// could offer, and the button is a convenience.
fn open_in_file_manager(path: &std::path::Path) {
    let dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";

    if let Err(err) = std::process::Command::new(program).arg(dir).spawn() {
        tracing::warn!("could not open {}: {err}", dir.display());
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
                    // Logged as well as handed to QML. A page that fails to load
                    // shows an error block the reader may not report, and before
                    // this a metadata failure left NO trace in the log at all --
                    // which is exactly why "nothing renders" was undiagnosable.
                    tracing::error!("metadata request failed: {message}");
                    let _ = qt_thread.queue(move |qobject| err(qobject, QString::from(&message)));
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
                let filter: ListFilter = serde_json::from_value(serde_json::Value::String(filter))
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
                            qobject.tagged_list_failed(QString::from(&tag), QString::from(&message))
                        });
                    }
                },
                Err(message) => {
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.tagged_list_failed(QString::from(&tag), QString::from(&message))
                    });
                }
            }
        });
    }

    /// One page of a curated list, as an `AnimePage` JSON object. Backs the
    /// scrolling "Top 100": `pageInfo` says whether another page follows.
    fn get_list_paged(self: Pin<&mut Self>, filter: QString, page: i32, per_page: i32) {
        let filter = filter.to_string();
        let page = page.max(1) as u32;
        let per_page = limit_or_default(per_page);
        self.spawn_metadata(
            move |provider| async move {
                let filter: ListFilter = serde_json::from_value(serde_json::Value::String(filter))
                    .map_err(|err| format!("unknown list filter: {err}"))?;
                provider
                    .list_page(filter, page, per_page)
                    .await
                    .map(|page| json_or_empty(&page))
                    .map_err(|err| err.to_string())
            },
            Self::list_ready,
            Self::list_failed,
        );
    }

    /// Every home shelf in one provider call, so the home screen costs a
    /// single AniList request instead of six.
    ///
    /// Not `spawn_metadata`: the HERO slides need a banner cross-fill first,
    /// which `spawn_metadata` has no way to express (it only sees the trait
    /// object, not the two concrete clients).
    fn get_home(self: Pin<&mut Self>) {
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);
        let anilist = Arc::clone(&self.rust().anilist);
        let kitsu = Arc::clone(&self.rust().kitsu);

        runtime.spawn(async move {
            let result = match provider.home_shelves().await {
                Ok(mut shelves) => {
                    // Only the HERO slides need this. They are the first few
                    // trending titles, painted full-bleed behind the headline,
                    // where a missing banner means a portrait poster stretched
                    // across a wide strip.
                    //
                    // Capped and SEQUENTIAL on purpose: a cross-fill is two
                    // requests per work (map the id, then ask), and running
                    // them concurrently for a whole shelf is exactly the burst
                    // that trips AniList's rate limit. `HERO_BANNER_FILLS`
                    // bounds the worst case to a handful of requests.
                    for anime in shelves.trending.iter_mut().take(HERO_BANNER_FILLS) {
                        if anime.banner_image.is_none() {
                            anime.banner_image = borrow_banner(&anilist, &kitsu, anime).await;
                        }
                    }
                    Ok(json_or_empty(&shelves))
                }
                Err(err) => Err(err.to_string()),
            };

            match result {
                Ok(json) => {
                    let _ =
                        qt_thread.queue(move |qobject| qobject.home_ready(QString::from(&json)));
                }
                Err(message) => {
                    tracing::error!("metadata request failed: {message}");
                    let _ = qt_thread
                        .queue(move |qobject| qobject.home_failed(QString::from(&message)));
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

    fn get_anime(mut self: Pin<&mut Self>, id: i64, provider: QString) {
        // The namespace travels WITH the id, from the card the reader clicked.
        //
        // It is deliberately NOT inferred from the partial cache: a cold cache
        // (a fresh launch, the resume disc, `KITSUNE_OPEN_ID`) would fall back
        // to AniList and fetch whatever work happens to own that number. The
        // caller always knows which source issued the id, so it says so.
        let namespace = provider_id_from(&provider.to_string());

        // Paint from what a list response already gave us, before any request.
        // Synchronous on purpose: the answer is in memory, and emitting it here
        // guarantees QML sees it before the async reply can arrive.
        //
        // The provider is CHECKED: the partial cache is keyed on the bare id,
        // so a cached partial from a different namespace must not be painted --
        // it would flash another work's poster before the real one arrives.
        if let Some(partial) = self
            .rust()
            .provider
            .cached_partial(id)
            .filter(|partial| partial.provider == namespace)
        {
            let json = serde_json::to_string(&partial).unwrap_or_else(|_| "null".into());
            // `as_mut` reborrows: the signal takes `Pin<&mut Self>`, and `self`
            // is still needed for `spawn_metadata` below.
            self.as_mut().anime_partial(QString::from(&json));
        }

        // The entry rides in the SAME request as the work, so this costs no
        // extra round trip. It is emitted on `listEntryReady`, which already
        // carries exactly `{id,status,progress}`, so `DetailPage` needs no
        // change and cannot race a separate `getListEntry`.
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);
        // Both concrete clients are needed for the banner cross-fill below.
        // `provider` is the decorator stack and cannot answer "what does the
        // OTHER source have for this work".
        let anilist = Arc::clone(&self.rust().anilist);
        let kitsu = Arc::clone(&self.rust().kitsu);

        runtime.spawn(async move {
            // The reader's list entry is an AniList concept, so only the
            // AniList-id path can carry one. A work opened by a Kitsu id was
            // never on the reader's AniList list under that id, so there is
            // nothing to fetch and "not on list" is the honest answer.
            let fetched = if namespace == ProviderId::AniList {
                // Both sources can address an AniList id: the primary natively,
                // the fallback through its mapping table. That is what keeps
                // the detail page working during an outage.
                provider.by_id_with_entry(id).await
            } else {
                provider
                    .by_id_in(namespace, id)
                    .await
                    .map(|anime| (anime, None))
            };

            match fetched {
                Ok((mut anime, entry)) => {
                    // A banner is the one field worth borrowing from the other
                    // source: it is a wide image the detail page paints behind
                    // the title, and a work often has one on AniList and not on
                    // Kitsu (or the reverse). Falling back to the COVER -- what
                    // QML already does -- crops a portrait poster across a wide
                    // strip, so it is worth one extra request to avoid.
                    if let Some(found) = anime.as_mut() {
                        if found.banner_image.is_none() {
                            found.banner_image = borrow_banner(&anilist, &kitsu, found).await;
                        }
                    }

                    let json = serde_json::to_string(&anime).unwrap_or_else(|_| "null".into());
                    // `None` -> JSON `null`, which QML reads as "not on list".
                    let entry_json =
                        serde_json::to_string(&entry).unwrap_or_else(|_| "null".into());

                    let _ = qt_thread
                        .clone()
                        .queue(move |qobject| qobject.anime_ready(QString::from(&json)));
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.list_entry_ready(id, QString::from(&entry_json))
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.anime_failed(
                            id,
                            QString::from(provider_namespace(namespace)),
                            QString::from(&message),
                        )
                    });
                }
            }
        });
    }

    fn get_recommendations(self: Pin<&mut Self>, id: i64, page: i32, per_page: i32) {
        let page = page.max(1) as u32;
        let per_page = limit_or_default(per_page);

        // Custom spawn (not `spawn_metadata`) because the reply must carry the
        // requested work id: the recommendations page can be re-pushed for a
        // different work while a reply is in flight, and a stale page would
        // then render against the new heading.
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);

        runtime.spawn(async move {
            match provider.recommendations(id, page, per_page).await {
                Ok(result) => {
                    let json = json_or_empty(&result);
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.recommendations_ready(id, QString::from(&json))
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.recommendations_failed(id, QString::from(&message))
                    });
                }
            }
        });
    }

    /// Cast the reader's vote on a recommendation and emit the new tally.
    fn rate_recommendation(
        self: Pin<&mut Self>,
        media_id: i64,
        recommended_id: i64,
        rating: QString,
    ) {
        let rating = rating.to_string();
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let anilist = Arc::clone(&self.rust().anilist);

        runtime.spawn(async move {
            let parsed: RecommendationRating =
                match serde_json::from_value(serde_json::Value::String(rating)) {
                    Ok(r) => r,
                    Err(err) => {
                        let message = format!("unknown recommendation rating: {err}");
                        let _ = qt_thread.queue(move |qobject| {
                            qobject.recommendation_rate_failed(QString::from(&message))
                        });
                        return;
                    }
                };

            match anilist
                .rate_recommendation(media_id, recommended_id, parsed)
                .await
            {
                Ok(tally) => {
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.recommendation_rated(media_id, recommended_id, tally)
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.recommendation_rate_failed(QString::from(&message))
                    });
                }
            }
        });
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

    /// The tagged variant of `get_schedule`, for the home page's embedded
    /// schedule widget: the reply carries `tag` back so a standalone schedule
    /// page's own request cannot be mistaken for this one's.
    fn get_schedule_tagged(self: Pin<&mut Self>, tag: QString, from: i64, to: i64, limit: i32) {
        let tag = tag.to_string();
        let limit = limit_or_default(limit);

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);

        runtime.spawn(async move {
            match provider.schedule(from, to, limit).await {
                Ok(items) => {
                    let json = json_or_empty(&items);
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.tagged_schedule_ready(QString::from(&tag), QString::from(&json))
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.tagged_schedule_failed(QString::from(&tag), QString::from(&message))
                    });
                }
            }
        });
    }

    fn get_episodes(self: Pin<&mut Self>, id: i64, provider: QString) {
        // Episodes come from Kitsu, not the AnimeProvider, so this does not go
        // through `spawn_metadata`; the plumbing is otherwise identical.
        //
        // The id alone is NOT enough: "12" is One Piece on Kitsu and an
        // unrelated work on AniList. The caller passes the namespace it got the
        // id from, which is the difference between the right episode list and
        // an empty one.
        let provider = provider.to_string();
        // Parsed once, then used for BOTH the fetch decision and the identity
        // echoed back to QML, so the page compares against exactly the
        // namespace the request was made in.
        let namespace = provider_id_from(&provider);
        let namespace_str = provider_namespace(namespace);
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let kitsu = Arc::clone(&self.rust().kitsu);

        runtime.spawn(async move {
            // A Kitsu id addresses episodes directly; an AniList id has to be
            // bridged through Kitsu's mapping table first.
            let resolved = if namespace == ProviderId::Kitsu {
                Ok(Some(id))
            } else {
                kitsu.kitsu_id_for_anilist(id).await
            };

            let result = match resolved {
                Ok(Some(kitsu_id)) => kitsu.episodes(kitsu_id).await.map(|catalogue| {
                    // `aired` is passed through AS COUNTED, including zero.
                    //
                    // It used to be replaced with the record count when it
                    // was zero, on the theory that an empty count meant the
                    // provider withheld it. Kitsu does not withhold it --
                    // it returns a record for every PLANNED episode, so a
                    // cour that has not started airing reports zero dated
                    // records and a full list of undated ones. Substituting
                    // the list length turned "nothing has aired" into
                    // "all twelve have aired", which is the bug this whole
                    // path exists to avoid.
                    //
                    // A work with no catalogue at all reports 0 here too,
                    // which the episode rules read as "unknown" and fall
                    // back to synthesis for.
                    tracing::info!(
                        kitsu_id,
                        records = catalogue.info.len(),
                        aired = catalogue.aired,
                        // True when paging stopped at the page cap: the
                        // grid then synthesises the tail from `aired`.
                        truncated = catalogue.truncated,
                        "episode catalogue"
                    );
                    serde_json::json!({ "info": catalogue.info, "aired": catalogue.aired })
                }),
                // No mapping means no episode data. Reported as an empty
                // catalogue rather than an error, so the page falls back to
                // synthesising from what the provider already told us.
                Ok(None) => Ok(serde_json::json!({ "info": [], "aired": 0 })),
                Err(err) => Err(err),
            };

            match result {
                Ok(payload) => {
                    // `info` and `aired` stay at the top level so the existing
                    // reader of this payload keeps working; the identity is
                    // added alongside, not wrapped around it.
                    let mut payload = payload;
                    if let Some(object) = payload.as_object_mut() {
                        object.insert("id".into(), serde_json::json!(id));
                        object.insert(
                            "provider".into(),
                            serde_json::json!(provider_namespace(namespace)),
                        );
                    }
                    let json = payload.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.episodes_ready(id, QString::from(namespace_str), QString::from(&json))
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.episodes_failed(
                            id,
                            QString::from(namespace_str),
                            QString::from(&message),
                        )
                    });
                }
            }
        });
    }

    /// Build the display episode list from a work's scalars and its (optional)
    /// episode catalogue. Pure: the shared `kitsune_core::episodes` rules do the
    /// work, so the Qt app and the web app cannot disagree.
    fn build_episodes(
        self: Pin<&mut Self>,
        episode_count: i32,
        status: QString,
        cover_image: QString,
        info_json: QString,
    ) -> QString {
        let info_json = info_json.to_string();
        // The catalogue arrives as `{ info, aired }`: the episodes themselves
        // plus how many have aired, which is what stops an airing show from
        // offering episodes the reader cannot watch yet.
        let catalogue: EpisodeCatalogueWire = serde_json::from_str(&info_json).unwrap_or_default();
        let info = catalogue.info;

        let cover = cover_image.to_string();
        let status = status.to_string();
        let episodes = kitsune_core::episodes::episodes_for_facts(
            (episode_count > 0).then_some(episode_count as u32),
            (!status.is_empty()).then_some(status.as_str()),
            (!cover.is_empty()).then_some(cover.as_str()),
            &info,
            (catalogue.aired > 0).then_some(catalogue.aired),
            catalogue.next_airing,
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

    // --- Speed test -------------------------------------------------------

    fn run_speed_test(self: Pin<&mut Self>) {
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);

        // Missing means the HTTP client could not be built; say so rather than
        // silently doing nothing.
        let Some(tester) = self.rust().speed_tester.clone() else {
            self.speed_test_failed(QString::from(
                "speed test unavailable: could not build the HTTP client",
            ));
            return;
        };

        runtime.spawn(async move {
            // Progress is emitted from inside the measurement, on this same
            // runtime, so each update is queued straight onto the Qt thread.
            // The tester refuses a concurrent run itself.
            let progress_thread = qt_thread.clone();
            let result = tester
                .run(move |progress: SpeedTestProgress| {
                    let json = serde_json::to_string(&progress).unwrap_or_default();
                    let _ = progress_thread
                        .queue(move |qobject| qobject.speed_test_progress(QString::from(&json)));
                })
                .await;

            match result {
                Ok(result) => {
                    let json = serde_json::to_string(&result).unwrap_or_else(|_| "null".into());
                    let _ = qt_thread
                        .queue(move |qobject| qobject.speed_test_ready(QString::from(&json)));
                }
                Err(err) => {
                    let message = format!("speed test failed: {err:#}");
                    let _ = qt_thread
                        .queue(move |qobject| qobject.speed_test_failed(QString::from(&message)));
                }
            }
        });
    }

    // --- Releases & torrents ----------------------------------------------

    fn search_releases(
        self: Pin<&mut Self>,
        tag: QString,
        titles: QString,
        episode: i32,
        absolute_episode: i32,
        mode: QString,
        force: bool,
    ) {
        let tag = tag.to_string();
        let titles = titles.to_string();
        let mode = mode.to_string();

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let indexers = Arc::clone(&self.rust().indexers);
        let settings = Arc::clone(&self.rust().settings);
        let releases = Arc::clone(&self.rust().releases);

        runtime.spawn(async move {
            // Every early return echoes the tag, so QML can always tell which
            // open an answer belongs to.
            let fail_tagged = |message: String| {
                let tag = QString::from(&tag);
                let message = QString::from(&message);
                let _ = qt_thread.queue(move |qobject| qobject.releases_failed(tag, message));
            };

            let titles: Vec<String> = match serde_json::from_str(&titles) {
                Ok(titles) => titles,
                Err(err) => return fail_tagged(format!("bad titles payload: {err}")),
            };

            // `<= 0` means "no episode" (a film or a title-level search).
            let episode = (episode > 0).then_some(episode as u32);
            let absolute_episode = (absolute_episode > 0).then_some(absolute_episode as u32);

            let mode: SearchMode = match serde_json::from_value(serde_json::Value::String(mode)) {
                Ok(mode) => mode,
                Err(err) => return fail_tagged(format!("unknown search mode: {err}")),
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

            // A cache hit answers immediately; `force` (Retry) always searches.
            let found = releases
                .search(
                    indexers.indexers(),
                    &request,
                    &preference,
                    ttl::RELEASES,
                    force,
                )
                .await
                .unwrap_or_default();

            let json = serde_json::to_string(&found).unwrap_or_else(|_| "[]".into());
            let tag = QString::from(&tag);
            let _ =
                qt_thread.queue(move |qobject| qobject.releases_ready(tag, QString::from(&json)));
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
                    let _ =
                        qt_thread.queue(move |qobject| qobject.torrent_added(QString::from(&json)));
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

    fn prepare_stream(self: Pin<&mut Self>, torrent_id: i64, file_idx: i64) {
        let id = torrent_id as usize;
        let file = file_idx as usize;
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            // Timed here as well as inside `prepare_stream`: the difference
            // between the two numbers is scheduling/queueing overhead, and a
            // large gap there would point at the runtime rather than the
            // engine.
            tracing::info!("backend: prepareStream requested torrent={id} file={file}");
            let started = std::time::Instant::now();
            // One call, not two: the selection must be in force before the
            // URL is handed to the player, or the engine downloads a file
            // nobody is watching. `prepare_stream` waits for the torrent to
            // leave initializing, verifies the selection, then warms the head
            // and tail so the player is not left blocking on a range.
            match player.prepare_stream(id, file).await {
                Ok(url) => {
                    tracing::info!(
                        "backend: prepareStream completed torrent={id} file={file} in {}ms",
                        started.elapsed().as_millis()
                    );
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.stream_url_ready(id as i64, file as i64, QString::from(&url))
                    });
                }
                Err(err) => {
                    tracing::error!(
                        "backend: prepareStream FAILED torrent={id} file={file} after {}ms: {err:#}",
                        started.elapsed().as_millis()
                    );
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
        let id = torrent_id as usize;
        let progress = self.rust().player.torrent_progress(id);
        // The Rust half of the 1 Hz diagnosis. `file_progress` is per-index
        // bytes-on-disk, which is the number to compare against mpv's
        // `cache-end` in the log: a large `file_progress[i]` next to a tiny
        // `cache-end` means mpv is NOT reading index i. Logged at debug
        // because QML polls this twice a second.
        if let Some(p) = &progress {
            tracing::debug!(
                "torrent {id}: state={} peers={} dl={:.1}MiB/s total={}MB/{}MB \
                 finished={} file_progress={:?}",
                p.state,
                p.peers_live,
                p.download_mbps,
                p.progress_bytes / 1_000_000,
                p.total_bytes / 1_000_000,
                p.finished,
                p.file_progress
            );
        }
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

    /// Probe every release for swarm health, reporting each verdict as it
    /// lands. Nothing is returned: the answers are `probeResult` signals,
    /// terminated by `probeDone`.
    fn probe_releases(self: Pin<&mut Self>, tag: QString, releases_json: QString) {
        let tag = tag.to_string();
        let releases_json = releases_json.to_string();

        let releases: Vec<Release> = match serde_json::from_str(&releases_json) {
            Ok(releases) => releases,
            Err(err) => {
                tracing::warn!("bad releases payload for probe: {err}");
                self.probe_done(QString::from(&tag));
                return;
            }
        };

        // A cached batch is replayed rather than re-probed: the verdicts are
        // index-aligned with this exact list, so they still apply. Emitted one
        // per release, then `probeDone`, exactly like a live batch.
        if let Some(cached) = self.rust().releases.cached_probes(&releases) {
            let qt_thread = self.qt_thread();
            for outcome in cached {
                let tag = QString::from(&tag);
                let badge = QString::from(badge_literal(outcome.badge));
                let index = outcome.index as i32;
                let score = outcome.combined_score;
                let _ = qt_thread
                    .clone()
                    .queue(move |qobject| qobject.probe_result(tag, index, badge, score));
            }
            let tag = QString::from(&tag);
            let _ = qt_thread.queue(move |qobject| qobject.probe_done(tag));
            return;
        }

        // The static half of every score, computed once; a probe only adds to
        // it, so re-deriving per event would be wasted work.
        let preference: ReleasePreference = {
            let stored = self.rust().settings.read();
            ReleasePreference {
                preferred_resolutions: stored.preferred_resolutions,
                min_seeders: stored.min_seeders,
            }
        };
        let base: Arc<Vec<i64>> = Arc::new(
            releases
                .iter()
                .map(|release| rank::score(release, &preference))
                .collect(),
        );

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let cache = Arc::clone(&self.rust().releases);

        runtime.spawn(async move {
            // A dedicated session, matching the Tauri path: registering a
            // release on the player's session with an empty file selection
            // would make a later playback request for it download nothing.
            let dir = std::env::temp_dir().join("kitsune-probes");

            // Outcomes are collected as they land so the finished batch can be
            // cached in one write; a partial batch is never stored.
            let collected: Arc<std::sync::Mutex<Vec<ProbeOutcome>>> =
                Arc::new(std::sync::Mutex::new(Vec::with_capacity(releases.len())));

            let for_events = Arc::clone(&base);
            let for_store = Arc::clone(&collected);
            let tag_for_events = tag.clone();
            // The callback fires once per release, so the thread handle is
            // cloned per call rather than moved into the closure once.
            let thread_for_events = qt_thread.clone();
            probe_releases_standalone(dir, &releases, move |index, probe| {
                let base_score = for_events.get(index).copied().unwrap_or(0);
                let outcome = probe_rank::outcome(index, base_score, probe);

                if let Ok(mut collected) = for_store.lock() {
                    collected.push(outcome.clone());
                }

                // `queue` needs a `'static` closure, so the values are moved
                // in, not borrowed.
                let tag = QString::from(&tag_for_events);
                let badge = QString::from(badge_literal(outcome.badge));
                let index = outcome.index as i32;
                let score = outcome.combined_score;
                let _ = thread_for_events
                    .clone()
                    .queue(move |qobject| qobject.probe_result(tag, index, badge, score));
            })
            .await;

            // Store before `probeDone`, so a modal that reacts to the final
            // signal by reopening finds a warm cache.
            let outcomes = std::sync::Arc::try_unwrap(collected)
                .map(|mutex| mutex.into_inner().unwrap_or_default())
                .unwrap_or_default();
            if !outcomes.is_empty() {
                cache.store_probes(&releases, outcomes);
            }

            let tag = QString::from(&tag);
            let _ = qt_thread.queue(move |qobject| qobject.probe_done(tag));
        });
    }

    fn probe_release(self: Pin<&mut Self>, tag: QString, release_json: QString) {
        let tag = tag.to_string();
        let release: Release = match serde_json::from_str(&release_json.to_string()) {
            Ok(release) => release,
            Err(err) => {
                self.release_probe_failed(
                    QString::from(&tag),
                    QString::from(&format!("bad release payload: {err}")),
                );
                return;
            }
        };

        let preference = {
            let stored = self.rust().settings.read();
            ReleasePreference {
                preferred_resolutions: stored.preferred_resolutions,
                min_seeders: stored.min_seeders,
            }
        };
        let base_score = rank::score(&release, &preference);
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);

        runtime.spawn(async move {
            let dir = std::env::temp_dir().join("kitsune-probes");
            let releases = vec![release];
            let mut result = None;
            probe_releases_standalone(dir, &releases, |_, probe| {
                result = Some(probe_rank::outcome(0, base_score, probe));
            })
            .await;

            match result {
                Some(outcome) => {
                    let json = serde_json::to_string(&outcome).unwrap_or_else(|_| "null".into());
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.release_probe_ready(QString::from(&tag), QString::from(&json))
                    });
                }
                None => {
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.release_probe_failed(
                            QString::from(&tag),
                            QString::from("the availability check could not start"),
                        )
                    });
                }
            }
        });
    }

    /// Fetch a `.torrent` from an indexer and write it to `path`.
    // --- Permanent episode downloads ---------------------------------------

    fn start_episode_download(
        self: Pin<&mut Self>,
        anime_id: i64,
        provider: QString,
        episode: i32,
        season: i32,
        title: QString,
        episode_title: QString,
        cover_url: QString,
        release_json: QString,
        file_indices_json: QString,
    ) {
        let Some(store) = self.rust().downloads.clone() else {
            self.episode_download_failed(
                QString::from(""),
                QString::from("the downloads store is unavailable"),
            );
            return;
        };

        let provider = provider.to_string();
        let title = title.to_string();
        let episode_title = episode_title.to_string();
        let cover_url = cover_url.to_string();

        // A season pack sends several indices; a single-episode release sends
        // one. Both arrive as a JSON array so there is only one code path.
        let file_indices: Vec<usize> =
            match serde_json::from_str::<Vec<i64>>(&file_indices_json.to_string()) {
                Ok(list) => list
                    .into_iter()
                    .filter(|i| *i >= 0)
                    .map(|i| i as usize)
                    .collect(),
                Err(err) => {
                    self.episode_download_failed(
                        QString::from(""),
                        QString::from(&format!("unreadable file list: {err}")),
                    );
                    return;
                }
            };

        if file_indices.is_empty() {
            self.episode_download_failed(
                QString::from(""),
                QString::from("no files were selected"),
            );
            return;
        }

        let release: Release = match serde_json::from_str(&release_json.to_string()) {
            Ok(release) => release,
            Err(err) => {
                self.episode_download_failed(
                    QString::from(""),
                    QString::from(&format!("unreadable release: {err}")),
                );
                return;
            }
        };

        // The engine id is process-local, so the info hash is the only stable
        // identity. Refuse a release without one rather than starting a
        // download that could never be reattached after a restart.
        let info_hash = release
            .info_hash
            .as_deref()
            .map(str::trim)
            .filter(|hash| hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(str::to_ascii_lowercase)
            .or_else(|| {
                kitsune_core::torrent::parse_info_hash(&release.magnet_uri)
                    .ok()
                    .map(|hash| hash.to_ascii_lowercase())
            });
        let Some(info_hash) = info_hash else {
            self.episode_download_failed(
                QString::from(""),
                QString::from("the release has no usable info hash"),
            );
            return;
        };

        let cache_dir = match self.rust().player.cache_dir_for(&info_hash) {
            Ok(dir) => dir,
            Err(err) => {
                self.episode_download_failed(
                    QString::from(""),
                    QString::from(&format!("invalid download cache: {err}")),
                );
                return;
            }
        };

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);
        let library_dir = self.rust().player.resolved_library_dir();
        let library_dir_string = library_dir.display().to_string();
        let season_number = release
            .parsed
            .season
            .or((season > 0).then_some(season as u32));

        runtime.spawn(async move {
            let magnet = release.magnet_uri.clone();

            let started = player
                .start_download_in(Some(&magnet), None, &file_indices, Some(&cache_dir), false)
                .await;
            let torrent_id = match started {
                Ok(id) => id,
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.episode_download_failed(QString::from(""), QString::from(&message))
                    });
                    return;
                }
            };

            // Cache the metadata while it is guaranteed to be in memory. A
            // restart can then reattach without waiting for DHT peers, and it
            // also preserves the file list after the torrent is removed.
            if let Some(bytes) = player.torrent_bytes(torrent_id) {
                if let Err(err) = save_cached_torrent(&info_hash, &bytes) {
                    tracing::warn!("could not cache torrent metadata for {info_hash}: {err:#}");
                }
            }

            // Lengths come from the torrent's own metadata; a release's
            // `sizeBytes` is the WHOLE torrent, which on a season pack is not
            // any one episode's size and would make every bar wrong.
            let torrent_files = player.torrent_files(torrent_id);
            let torrent_file_count = torrent_files.len() as u32;
            let mut created: Vec<DownloadRecord> = Vec::with_capacity(file_indices.len());

            for file_idx in &file_indices {
                let (total_bytes, file_name) = torrent_files
                    .iter()
                    .find(|file| file.idx == *file_idx)
                    .map(|file| (file.length_bytes, file.name.clone()))
                    .unwrap_or((0, String::new()));

                // A pack's files carry their own episode numbers ("... - 07"),
                // so each record should say which episode it is rather than
                // repeating the one the modal was opened for. Falls back to the
                // modal's episode when the name has no number.
                let file_episode = if file_indices.len() > 1 {
                    episode_from_filename(&file_name).or((episode > 0).then_some(episode as u32))
                } else {
                    (episode > 0).then_some(episode as u32)
                };

                created.push(DownloadRecord {
                    id: 0,
                    anime_id,
                    provider: provider.clone(),
                    episode: file_episode,
                    title: title.clone(),
                    cover_image: (!cover_url.is_empty() && cover_url != "undefined")
                        .then_some(cover_url.clone()),
                    episode_title: (episode_title != "undefined" && !episode_title.is_empty())
                        .then_some(episode_title.clone()),
                    release_title: release.title.clone(),
                    torrent_id: Some(torrent_id),
                    magnet: Some(magnet.clone()),
                    torrent_url: release.torrent_url.clone(),
                    info_hash: Some(info_hash.clone()),
                    cache_dir: Some(cache_dir.display().to_string()),
                    file_idx: *file_idx as u32,
                    file_name,
                    status: DownloadStatus::Downloading,
                    progress_bytes: 0,
                    total_bytes,
                    release_seeders: release.seeders,
                    output_path: String::new(),
                    added_at: now_secs(),
                    completed_at: None,
                    priority: DownloadPriority::Normal,
                    user_paused: false,
                    season: season_number,
                    library_dir: Some(library_dir_string.clone()),
                    torrent_file_count: Some(torrent_file_count),
                    error: None,
                });
            }

            let ids = match store.insert_many(created.clone()) {
                Ok(ids) => ids,
                Err(err) => {
                    tracing::warn!("could not record downloads: {err:#}");
                    let _ = player.remove_torrent(torrent_id).await;
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.episode_download_failed(QString::from(""), QString::from(&message))
                    });
                    return;
                }
            };

            for (record, id) in created.iter_mut().zip(ids) {
                record.id = id;
            }

            // The modal only needs to know the FIRST record: it reports "started
            // N episodes" from the list it already has, and the Downloads page
            // reloads the whole store anyway.
            let first = created.first().cloned();
            let json = serde_json::to_string(&first).unwrap_or_else(|_| "null".into());
            let _ = qt_thread.queue(move |qobject| qobject.download_started(QString::from(&json)));
        });
    }

    fn list_release_files(self: Pin<&mut Self>, token: QString, release_json: QString) {
        let token_string = token.to_string();
        let release: Release = match serde_json::from_str(&release_json.to_string()) {
            Ok(release) => release,
            Err(err) => {
                self.release_files_failed(
                    token,
                    QString::from(&format!("unreadable release: {err}")),
                );
                return;
            }
        };

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            let handle = match player.add_magnet(&release.magnet_uri).await {
                Ok(handle) => handle,
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.release_files_failed(
                            QString::from(&token_string),
                            QString::from(&message),
                        )
                    });
                    return;
                }
            };

            // The list is all this command needs, so the torrent is dropped
            // straight away: leaving it registered would keep the transfer
            // running for a release the reader may only be inspecting.
            if let Err(err) = player.remove_torrent(handle.id).await {
                tracing::warn!("could not drop inspected torrent {}: {err:#}", handle.id);
            }

            let payload = serde_json::json!({
                "name": release.title,
                "files": handle.files,
            });
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "null".into());
            let _ = qt_thread.queue(move |qobject| {
                qobject.release_files_ready(QString::from(&token_string), QString::from(&json))
            });
        });
    }

    fn get_downloads(self: Pin<&mut Self>) {
        // Synchronous: a small JSON file, read on the Qt thread. Doing this in
        // the runtime would make every page open race its own first paint.
        let json = match self.rust().downloads.as_ref() {
            Some(store) => downloads_json(&store.read_all()),
            None => "[]".to_string(),
        };
        self.downloads_ready(QString::from(&json));
    }

    fn restore_downloads(self: Pin<&mut Self>) {
        let Some(store) = self.rust().downloads.clone() else {
            return;
        };

        // The Downloads page can be instantiated more than once; reattaching a
        // group twice would add duplicate torrents to the session.
        if self
            .rust()
            .downloads_restore_started
            .swap(true, Ordering::SeqCst)
        {
            return;
        }

        let mut groups: std::collections::HashMap<String, Vec<DownloadRecord>> =
            std::collections::HashMap::new();
        for record in store.read_all().into_iter().filter(needs_restore) {
            groups.entry(record.group_key()).or_default().push(record);
        }

        if groups.is_empty() {
            self.rust()
                .downloads_restore_done
                .store(true, Ordering::SeqCst);
            self.downloads_ready(QString::from(&downloads_json(&store.read_all())));
            return;
        }

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);
        let restoring = Arc::clone(&self.rust().restoring_downloads);
        let restore_done = Arc::clone(&self.rust().downloads_restore_done);
        let remaining = Arc::new(std::sync::atomic::AtomicUsize::new(groups.len()));

        for (group_key, records) in groups {
            mark_restoring(&restoring, &group_key);
            let store = store.clone();
            let player = Arc::clone(&player);
            let restoring = Arc::clone(&restoring);
            let remaining = Arc::clone(&remaining);
            let restore_done = Arc::clone(&restore_done);
            let qt_thread = qt_thread.clone();

            runtime.spawn(async move {
                let result = attach_group(&player, &records, true, &group_indices(&records)).await;

                let final_list = match result {
                    Ok(attached) => {
                        if let Some(bytes) = player.torrent_bytes(attached.torrent_id) {
                            if let Err(err) = save_cached_torrent(&attached.info_hash, &bytes) {
                                tracing::warn!(
                                    "could not cache restored torrent {}: {err:#}",
                                    attached.info_hash
                                );
                            }
                        }

                        let hash = attached.info_hash.clone();
                        let cache = attached.cache_dir.display().to_string();
                        let file_count = attached.files.len() as u32;
                        let torrent_id = attached.torrent_id;
                        let update = store.mutate(|all| {
                            for record in all.iter_mut() {
                                if record.group_key() != group_key || !needs_restore(record) {
                                    continue;
                                }
                                record.torrent_id = Some(torrent_id);
                                record.info_hash = Some(hash.clone());
                                record.cache_dir = Some(cache.clone());
                                record.torrent_file_count = Some(file_count);
                                record.status = DownloadStatus::Paused;
                                record.error = None;
                            }
                        });
                        if let Err(err) = update {
                            tracing::warn!("could not record restored group {group_key}: {err:#}");
                        }

                        tracing::info!(
                            "restored download group {group_key} as torrent {torrent_id} (paused, {} files)",
                            attached.files.len()
                        );
                        downloads_json(&store.read_all())
                    }
                    Err(message) => {
                        tracing::warn!("could not restore download group {group_key}: {message}");
                        let reason = message.clone();
                        let update = store.mutate(|all| {
                            for record in all.iter_mut() {
                                if record.group_key() != group_key || !needs_restore(record) {
                                    continue;
                                }
                                record.torrent_id = None;
                                record.status = DownloadStatus::Failed;
                                record.error = Some(reason.clone());
                            }
                        });
                        if let Err(err) = update {
                            tracing::warn!("could not mark group {group_key} failed: {err:#}");
                        }
                        downloads_json(&store.read_all())
                    }
                };

                unmark_restoring(&restoring, &group_key);

                // The last group to finish publishes the complete list once,
                // rather than every group racing the page with a partial one.
                if remaining.fetch_sub(1, Ordering::SeqCst) == 1 {
                    restore_done.store(true, Ordering::SeqCst);
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.downloads_ready(QString::from(&final_list))
                    });
                }
            });
        }
    }

    fn poll_downloads(self: Pin<&mut Self>) {
        let Some(store) = self.rust().downloads.clone() else {
            self.downloads_ready(QString::from("[]"));
            return;
        };

        // The startup reattach owns the store while it runs. A poll that read
        // a record before its stale engine ID was replaced would see a missing
        // torrent and permanently mark a resumable download Failed.
        if self.rust().downloads_restore_started.load(Ordering::SeqCst)
            && !self.rust().downloads_restore_done.load(Ordering::SeqCst)
        {
            let views: Vec<DownloadView> = store
                .read_all()
                .into_iter()
                .map(|record| {
                    let count =
                        group_file_count(std::slice::from_ref(&record), &record.group_key());
                    DownloadView::idle(record, count, true)
                })
                .collect();
            self.download_progress(QString::from(&views_json(&views)));
            return;
        }

        let player = Arc::clone(&self.rust().player);
        let restoring = Arc::clone(&self.rust().restoring_downloads);
        let mut records = store.read_all();
        let mut finished: Vec<DownloadRecord> = Vec::new();
        let mut completed_paths: Vec<(u64, usize)> = Vec::new();
        // Live numbers per record id, for the view sent to QML.
        let mut live: std::collections::HashMap<u64, (f64, u32, Option<u64>)> =
            std::collections::HashMap::new();

        // Reconcile before polling bytes. A newly added batch used to leave
        // every torrent downloading at once, which made one episode consume
        // the pipe while the rest sat at 0%. The queue makes the default
        // policy one active download and pauses the remainder.
        let queue_plan = plan_queue(&records, player.max_concurrent_downloads());
        for (id, action) in queue_plan {
            let Some(record) = records.iter_mut().find(|record| record.id == id) else {
                continue;
            };
            if is_restoring(&restoring, &record.group_key()) {
                continue;
            }
            let Some(torrent_id) = record.torrent_id else {
                continue;
            };

            match action {
                QueueAction::Start if record.status == DownloadStatus::Queued => {
                    record.status = DownloadStatus::Downloading;
                    let runtime = Arc::clone(&self.rust().runtime);
                    let player = Arc::clone(&self.rust().player);
                    runtime.spawn(async move {
                        if let Err(err) = player.resume_torrent(torrent_id).await {
                            tracing::warn!("could not resume queued torrent {torrent_id}: {err:#}");
                        }
                    });
                }
                QueueAction::Queue if record.status == DownloadStatus::Downloading => {
                    record.status = DownloadStatus::Queued;
                    let runtime = Arc::clone(&self.rust().runtime);
                    let player = Arc::clone(&self.rust().player);
                    runtime.spawn(async move {
                        if let Err(err) = player.pause_torrent(torrent_id).await {
                            tracing::warn!("could not queue torrent {torrent_id}: {err:#}");
                        }
                    });
                }
                _ => {}
            }
        }

        for record in records.iter_mut() {
            if is_restoring(&restoring, &record.group_key()) {
                continue;
            }
            let (Some(torrent_id), DownloadStatus::Downloading) =
                (record.torrent_id, record.status)
            else {
                continue;
            };

            let Some((progress, done)) =
                player.download_snapshot(torrent_id, record.file_idx as usize)
            else {
                // The torrent vanished under us (engine restart, removal from
                // another surface). Saying "failed" is more honest than a bar
                // that never moves again.
                if record.status == DownloadStatus::Downloading {
                    record.status = DownloadStatus::Failed;
                    record.error = Some("the torrent is no longer in the download session".into());
                    finished.push(record.clone());
                }
                continue;
            };

            record.progress_bytes = progress
                .file_progress
                .get(record.file_idx as usize)
                .copied()
                .unwrap_or(progress.progress_bytes);
            if record.total_bytes == 0 {
                record.total_bytes = progress.total_bytes;
            }

            // The live numbers the page shows. `download_mbps` is the whole
            // torrent's rate, not this file's: librqbit reports one speed per
            // torrent, and per-file rates do not exist. For a single-file
            // download -- the normal case -- they are the same number.
            live.insert(
                record.id,
                (
                    progress.download_mbps,
                    progress.peers_live,
                    // The torrent's ETA covers every selected file, so it is
                    // only this file's ETA when it is the only one selected.
                    // Showing a season pack's whole-torrent ETA on episode 3
                    // would be off by hours.
                    if progress.file_progress.len() <= 1 {
                        progress.eta_seconds
                    } else {
                        file_eta(record, &progress)
                    },
                ),
            );

            if done {
                completed_paths.push((record.id, record.file_idx as usize));
            }
        }

        let in_flight: Vec<u64> = completed_paths.iter().map(|(id, _)| *id).collect();

        if !completed_paths.is_empty() {
            // Several files of one torrent can finish between two polls. Only
            // the LAST of them may drop the torrent: an earlier one would
            // delete the data the next `finish_download` is still moving.
            let completed_set: std::collections::HashSet<u64> =
                completed_paths.iter().map(|(id, _)| *id).collect();
            let mut last_in_group: std::collections::HashMap<String, u64> =
                std::collections::HashMap::new();
            let mut group_has_remaining: std::collections::HashMap<String, bool> =
                std::collections::HashMap::new();
            for (id, _) in &completed_paths {
                let Some(record) = records.iter().find(|record| record.id == *id) else {
                    continue;
                };
                let key = record.group_key();
                last_in_group.insert(key.clone(), *id);
                group_has_remaining.entry(key.clone()).or_insert_with(|| {
                    records.iter().any(|other| {
                        other.group_key() == key
                            && other.status != DownloadStatus::Complete
                            && !completed_set.contains(&other.id)
                    })
                });
            }

            let runtime = Arc::clone(&self.rust().runtime);
            let qt_thread = self.qt_thread();
            let store = store.clone();
            let player = Arc::clone(&self.rust().player);
            let snapshot = records.clone();
            let default_library = self.rust().player.resolved_library_dir();

            runtime.spawn(async move {
                for (id, file_idx) in completed_paths {
                    let Some(record) = snapshot.iter().find(|r| r.id == id).cloned() else {
                        continue;
                    };
                    let Some(torrent_id) = record.torrent_id else {
                        continue;
                    };

                    let season = record.season;
                    let library_dir = record
                        .library_dir
                        .as_deref()
                        .filter(|dir| !dir.trim().is_empty())
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| default_library.clone());

                    // A season pack backs several records with ONE torrent. If
                    // any OTHER record still needs it, the transfer must stay
                    // alive: dropping it here would delete the data the
                    // remaining episodes are still reading from.
                    let group_key = record.group_key();
                    let drop_torrent = last_in_group.get(&group_key).copied() == Some(id)
                        && !group_has_remaining
                            .get(&group_key)
                            .copied()
                            .unwrap_or(false);

                    let moved = player
                        .finish_download(
                            torrent_id,
                            file_idx,
                            &library_dir,
                            &record.title,
                            season,
                            record.episode,
                            drop_torrent,
                        )
                        .await;

                    let mut done_record = record.clone();
                    match moved {
                        Ok(path) => {
                            done_record.status = DownloadStatus::Complete;
                            done_record.output_path = path.display().to_string();
                            done_record.progress_bytes = done_record.total_bytes;
                            done_record.completed_at = Some(now_secs());
                            done_record.torrent_id = None;
                            done_record.error = None;
                        }
                        Err(err) => {
                            tracing::warn!("download {} could not finish: {err:#}", record.id);
                            done_record.status = DownloadStatus::Failed;
                            done_record.error = Some(err.to_string());
                        }
                    }

                    if let Err(err) = store.update(&done_record) {
                        tracing::warn!("could not update download {}: {err:#}", done_record.id);
                    }

                    let json =
                        serde_json::to_string(&done_record).unwrap_or_else(|_| "null".into());
                    let ok = done_record.status == DownloadStatus::Complete;
                    // `downloadsReady` always carries an ARRAY; the page parses
                    // it into its list and calls `.filter` on the result. Sending
                    // one record here (as a failure used to) made that call
                    // throw `Property 'filter' of object is not a function` on
                    // every repaint.
                    let list_json =
                        serde_json::to_string(&store.read_all()).unwrap_or_else(|_| "[]".into());

                    // One emit per `queue`: the generated signal method takes
                    // `self: Pin<&mut Self>`, so a single `qobject` cannot be
                    // used twice in one closure. A second handle is cheap.
                    if ok {
                        let completed_thread = qt_thread.clone();
                        let _ = completed_thread
                            .queue(move |qobject| qobject.download_completed(QString::from(&json)));
                    }
                    let _ = qt_thread
                        .queue(move |qobject| qobject.downloads_ready(QString::from(&list_json)));
                }
            });
        }

        if !finished.is_empty() {
            for record in finished {
                let _ = store.update(&record);
            }
        }

        // Persist byte counters and status changes by ID. Writing the whole
        // snapshot would clobber a group changed by the restorer or the file
        // editor between our read and this write.
        //
        // Records whose files are being moved are skipped: the finisher owns
        // them until it writes `Complete`, and a stale `Downloading` copy would
        // revert a finished download to a running one with no torrent id.
        let updates: std::collections::HashMap<u64, DownloadRecord> = records
            .iter()
            .filter(|record| !in_flight.contains(&record.id))
            .filter(|record| !is_restoring(&restoring, &record.group_key()))
            .map(|record| (record.id, record.clone()))
            .collect();
        if let Err(err) = store.mutate(|all| {
            for record in all.iter_mut() {
                if let Some(update) = updates.get(&record.id) {
                    *record = update.clone();
                }
            }
        }) {
            tracing::warn!("could not persist download progress: {err:#}");
        }

        // Send the VIEW list, so the page can show speed, peers and ETA next
        // to the persisted numbers.
        let views: Vec<DownloadView> = records
            .iter()
            .map(|record| {
                let group_key = record.group_key();
                let file_count = group_file_count(&records, &group_key);
                let restoring = is_restoring(&restoring, &group_key);
                match live.get(&record.id) {
                    Some((speed, peers, eta)) => DownloadView {
                        group_key,
                        record: record.clone(),
                        file_count,
                        restoring,
                        speed_mbps: *speed,
                        peers_live: *peers,
                        eta_seconds: *eta,
                    },
                    None => DownloadView::idle(record.clone(), file_count, restoring),
                }
            })
            .collect();
        self.download_progress(QString::from(&views_json(&views)));
    }

    fn list_download_files(self: Pin<&mut Self>, token: QString, group_key: QString) {
        let Some(store) = self.rust().downloads.clone() else {
            self.download_files_failed(token, QString::from("the downloads store is unavailable"));
            return;
        };

        let token_string = token.to_string();
        let key = group_key.to_string();
        let records = group_records(&store.read_all(), &key);
        if records.is_empty() {
            self.download_files_failed(token, QString::from("the download no longer exists"));
            return;
        }

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            let mut temporary = None;
            let (torrent_id, files) = match live_files(&player, &records) {
                Some(found) => found,
                None => match attach_group(&player, &records, true, &group_indices(&records)).await
                {
                    Ok(attached) => {
                        save_attached_metadata(&player, &attached);
                        temporary = Some(attached.torrent_id);
                        (attached.torrent_id, attached.files)
                    }
                    Err(message) => {
                        let _ = qt_thread.queue(move |qobject| {
                            qobject.download_files_failed(
                                QString::from(&token_string),
                                QString::from(&message),
                            )
                        });
                        return;
                    }
                },
            };

            let selected: std::collections::HashSet<usize> = records
                .iter()
                .map(|record| record.file_idx as usize)
                .collect();
            let name = records
                .first()
                .map(|record| record.release_title.clone())
                .unwrap_or_default();
            let json = download_files_json(&key, &name, &files, &selected);

            // Inspection must not leave a transfer behind. The attach was
            // paused, so dropping it here cannot lose downloaded pieces.
            if temporary == Some(torrent_id) {
                if let Err(err) = player.remove_torrent(torrent_id).await {
                    tracing::warn!("could not drop inspected torrent {torrent_id}: {err:#}");
                }
            }

            let _ = qt_thread.queue(move |qobject| {
                qobject.download_files_ready(QString::from(&token_string), QString::from(&json))
            });
        });
    }

    fn set_download_files(
        self: Pin<&mut Self>,
        token: QString,
        group_key: QString,
        indices_json: QString,
    ) {
        let Some(store) = self.rust().downloads.clone() else {
            self.download_files_failed(token, QString::from("the downloads store is unavailable"));
            return;
        };

        let mut selected: Vec<usize> =
            match serde_json::from_str::<Vec<i64>>(&indices_json.to_string()) {
                Ok(indices) => indices
                    .into_iter()
                    .filter(|idx| *idx >= 0)
                    .map(|idx| idx as usize)
                    .collect(),
                Err(err) => {
                    self.download_files_failed(
                        token,
                        QString::from(&format!("unreadable file list: {err}")),
                    );
                    return;
                }
            };
        selected.sort_unstable();
        selected.dedup();

        if selected.is_empty() {
            self.download_files_failed(token, QString::from("a download needs at least one file"));
            return;
        }

        let token_string = token.to_string();
        let key = group_key.to_string();
        let records = group_records(&store.read_all(), &key);
        if records.is_empty() {
            self.download_files_failed(token, QString::from("the download no longer exists"));
            return;
        }

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);

        runtime.spawn(async move {
            let selected_set: std::collections::HashSet<usize> = selected.iter().copied().collect();
            let was_active = records
                .iter()
                .any(|record| record.status == DownloadStatus::Downloading);
            let was_queued = records
                .iter()
                .any(|record| record.status == DownloadStatus::Queued);
            let had_incomplete = records.iter().any(|record| {
                matches!(
                    record.status,
                    DownloadStatus::Downloading | DownloadStatus::Queued | DownloadStatus::Paused
                )
            });
            let prototype = records[0].clone();

            let (torrent_id, files) = match live_files(&player, &records) {
                Some((torrent_id, files)) => {
                    if let Err(err) = player.set_only_files(torrent_id, selected.clone()).await {
                        let message = format!("could not change the file selection: {err}");
                        let _ = qt_thread.queue(move |qobject| {
                            qobject.download_files_failed(
                                QString::from(&token_string),
                                QString::from(&message),
                            )
                        });
                        return;
                    }
                    (torrent_id, files)
                }
                None => match attach_group(&player, &records, true, &selected).await {
                    Ok(attached) => {
                        save_attached_metadata(&player, &attached);
                        (attached.torrent_id, attached.files)
                    }
                    Err(message) => {
                        let _ = qt_thread.queue(move |qobject| {
                            qobject.download_files_failed(
                                QString::from(&token_string),
                                QString::from(&message),
                            )
                        });
                        return;
                    }
                },
            };

            if selected_set
                .iter()
                .any(|idx| !files.iter().any(|file| file.idx == *idx))
            {
                let message = "the torrent no longer contains one of the selected files";
                let _ = qt_thread.queue(move |qobject| {
                    qobject
                        .download_files_failed(QString::from(&token_string), QString::from(message))
                });
                return;
            }

            let info_hash = group_info_hash(&records);
            let cache_dir = records.iter().find_map(|record| record.cache_dir.clone());
            let file_count = files.len() as u32;
            let new_status = if was_active {
                DownloadStatus::Downloading
            } else if was_queued {
                DownloadStatus::Queued
            } else if had_incomplete {
                DownloadStatus::Paused
            } else {
                // Adding a file to a finished group starts a new transfer.
                DownloadStatus::Downloading
            };
            let should_resume =
                was_active || (!had_incomplete && new_status == DownloadStatus::Downloading);

            let make_row = |file: &kitsune_core::player::TorrentFile| {
                let status = new_status.clone();
                let user_paused = status == DownloadStatus::Paused;
                DownloadRecord {
                    id: 0,
                    anime_id: prototype.anime_id,
                    provider: prototype.provider.clone(),
                    episode: episode_from_filename(&file.name).or(prototype.episode),
                    title: prototype.title.clone(),
                    cover_image: prototype.cover_image.clone(),
                    episode_title: prototype.episode_title.clone(),
                    release_title: prototype.release_title.clone(),
                    torrent_id: Some(torrent_id),
                    magnet: prototype.magnet.clone(),
                    torrent_url: prototype.torrent_url.clone(),
                    info_hash: info_hash.clone(),
                    cache_dir: cache_dir.clone(),
                    file_idx: file.idx as u32,
                    file_name: file.name.clone(),
                    status,
                    progress_bytes: 0,
                    total_bytes: file.length_bytes,
                    release_seeders: prototype.release_seeders,
                    output_path: String::new(),
                    added_at: now_secs(),
                    completed_at: None,
                    priority: prototype.priority,
                    user_paused,
                    season: prototype.season,
                    library_dir: prototype.library_dir.clone(),
                    torrent_file_count: Some(file_count),
                    error: None,
                }
            };

            let new_rows: Vec<DownloadRecord> = files
                .iter()
                .filter(|file| selected_set.contains(&file.idx))
                .filter(|file| {
                    !records
                        .iter()
                        .any(|record| record.file_idx as usize == file.idx)
                })
                .map(&make_row)
                .collect();

            let update = store.mutate(|all| {
                all.retain(|record| {
                    record.group_key() != key || selected_set.contains(&(record.file_idx as usize))
                });
                for record in all.iter_mut() {
                    if record.group_key() != key {
                        continue;
                    }
                    record.torrent_id = Some(torrent_id);
                    record.info_hash = info_hash.clone();
                    record.cache_dir = cache_dir.clone();
                    record.torrent_file_count = Some(file_count);
                    if record.status != DownloadStatus::Complete {
                        record.error = None;
                    }
                }
                let mut next = all.iter().map(|record| record.id).max().unwrap_or(0) + 1;
                for mut row in new_rows {
                    row.id = next;
                    next += 1;
                    all.push(row);
                }
            });

            if let Err(err) = update {
                let message = format!("could not save the file selection: {err}");
                let _ = qt_thread.queue(move |qobject| {
                    qobject.download_files_failed(
                        QString::from(&token_string),
                        QString::from(&message),
                    )
                });
                return;
            }

            if should_resume {
                if let Err(err) = player.resume_torrent(torrent_id).await {
                    tracing::warn!("could not resume edited torrent {torrent_id}: {err:#}");
                }
            }

            let selected_after: std::collections::HashSet<usize> = selected_set.clone();
            let name = prototype.release_title.clone();
            let files_json = download_files_json(&key, &name, &files, &selected_after);
            let list_json = downloads_json(&store.read_all());

            let files_thread = qt_thread.clone();
            let _ = files_thread.queue(move |qobject| {
                qobject
                    .download_files_ready(QString::from(&token_string), QString::from(&files_json))
            });
            let _ =
                qt_thread.queue(move |qobject| qobject.downloads_ready(QString::from(&list_json)));
        });
    }

    fn remove_download(self: Pin<&mut Self>, id: QString, delete_file: bool) {
        let Some(store) = self.rust().downloads.clone() else {
            return;
        };
        let id = download_id(&id.to_string());
        let all = store.read_all();
        let Some(group_key) = group_key_for_id(&all, id) else {
            return;
        };
        let targets = group_records(&all, &group_key);
        let target_ids: std::collections::HashSet<u64> =
            targets.iter().map(|record| record.id).collect();
        let torrent_ids = unique_torrent_ids(&targets);

        if let Err(err) = store.mutate(|records| {
            records.retain(|record| !target_ids.contains(&record.id));
        }) {
            tracing::warn!("could not remove download group {group_key}: {err:#}");
            return;
        }

        for record in &targets {
            if delete_file && !record.output_path.is_empty() {
                if let Err(err) = std::fs::remove_file(&record.output_path) {
                    tracing::warn!("could not delete {}: {err}", record.output_path);
                }
            }
        }

        let remaining = store.read_all();
        let still_exists = remaining
            .iter()
            .any(|record| record.group_key() == group_key);

        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);
        let info_hash = group_info_hash(&targets);
        let cache_dirs: Vec<(String, std::path::PathBuf)> = targets
            .iter()
            .filter_map(|record| {
                let hash = record.info_hash.clone().or_else(|| info_hash.clone())?;
                let dir = record.cache_dir.clone().map(std::path::PathBuf::from)?;
                Some((hash, dir))
            })
            .collect();

        runtime.spawn(async move {
            for torrent_id in torrent_ids {
                if let Err(err) = player.remove_torrent(torrent_id).await {
                    tracing::warn!("could not stop torrent {torrent_id}: {err:#}");
                }
            }

            // A group is one torrent, so once its last row is gone the cached
            // metadata and partial data have no remaining owner.
            if !still_exists {
                if let Some(hash) = info_hash {
                    if let Err(err) = remove_cached_torrent(&hash) {
                        tracing::warn!("could not remove cached torrent {hash}: {err:#}");
                    }
                }
                for (hash, dir) in cache_dirs {
                    if let Err(err) = remove_download_cache(&dir, &hash) {
                        tracing::warn!(
                            "could not remove download cache {}: {err:#}",
                            dir.display()
                        );
                    }
                }
            }
        });

        self.downloads_ready(QString::from(&downloads_json(&store.read_all())));
    }

    fn pause_download(self: Pin<&mut Self>, id: QString) {
        self.set_download_paused(id, true);
    }

    fn resume_download(self: Pin<&mut Self>, id: QString) {
        self.set_download_paused(id, false);
    }

    fn set_download_priority(self: Pin<&mut Self>, id: QString, priority: QString) {
        let Some(store) = self.rust().downloads.clone() else {
            return;
        };
        let id = download_id(&id.to_string());
        let priority = match priority.to_string().as_str() {
            "high" => DownloadPriority::High,
            "low" => DownloadPriority::Low,
            _ => DownloadPriority::Normal,
        };

        let all = store.read_all();
        let Some(group_key) = group_key_for_id(&all, id) else {
            return;
        };
        if let Err(err) = store.mutate(|records| {
            for record in records.iter_mut() {
                if record.group_key() == group_key {
                    record.priority = priority;
                }
            }
        }) {
            tracing::warn!("could not update priority for {group_key}: {err:#}");
        }
        self.downloads_ready(QString::from(&downloads_json(&store.read_all())));
    }

    fn library_dir(self: Pin<&mut Self>) -> QString {
        QString::from(
            &self
                .rust()
                .player
                .resolved_library_dir()
                .display()
                .to_string(),
        )
    }

    fn download_path(self: Pin<&mut Self>, id: QString) -> QString {
        let Some(store) = self.rust().downloads.as_ref() else {
            return QString::from("");
        };
        let id = download_id(&id.to_string());
        let path = store
            .read_all()
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| r.output_path)
            .unwrap_or_default();

        if path.is_empty() || !std::path::Path::new(&path).exists() {
            return QString::from("");
        }
        QString::from(&path)
    }

    fn downloaded_for_anime(self: Pin<&mut Self>, anime_id: i64, provider: QString) -> QString {
        let records = match self.rust().downloads.as_ref() {
            Some(store) => store.read_all(),
            None => Vec::new(),
        };
        let views = downloaded_views_for(&records, anime_id, &provider.to_string());
        let json = serde_json::to_string(&views).unwrap_or_else(|_| "[]".into());
        QString::from(&json)
    }

    fn open_download_folder(self: Pin<&mut Self>, id: QString) {
        let Some(store) = self.rust().downloads.as_ref() else {
            return;
        };
        let id = download_id(&id.to_string());
        if let Some(record) = store.read_all().into_iter().find(|r| r.id == id) {
            if !record.output_path.is_empty() {
                open_in_file_manager(std::path::Path::new(&record.output_path));
            }
        }
    }

    fn open_library_folder(self: Pin<&mut Self>) {
        open_in_file_manager(&self.rust().player.resolved_library_dir());
    }

    /// Pause or resume one download group and record the new status.
    ///
    /// A group is one torrent, so pausing a card pauses every file row. Resume
    /// doubles as Retry: a Failed/Paused group with no live torrent is
    /// reattached from cached metadata (or its magnet).
    fn set_download_paused(self: Pin<&mut Self>, id: QString, paused: bool) {
        let Some(store) = self.rust().downloads.clone() else {
            return;
        };
        let id = download_id(&id.to_string());
        let all = store.read_all();
        let Some(group_key) = group_key_for_id(&all, id) else {
            return;
        };
        let records = group_records(&all, &group_key);
        let torrent_ids = unique_torrent_ids(&records);
        let live_id = torrent_ids.first().copied();

        if paused {
            let _ = store.mutate(|all| {
                for record in all.iter_mut() {
                    if record.group_key() == group_key
                        && !matches!(
                            record.status,
                            DownloadStatus::Complete | DownloadStatus::Failed
                        )
                    {
                        record.status = DownloadStatus::Paused;
                        record.user_paused = true;
                    }
                }
            });

            if let Some(torrent_id) = live_id {
                let runtime = Arc::clone(&self.rust().runtime);
                let player = Arc::clone(&self.rust().player);
                runtime.spawn(async move {
                    if let Err(err) = player.pause_torrent(torrent_id).await {
                        tracing::warn!("could not pause torrent {torrent_id}: {err:#}");
                    }
                });
            }

            self.downloads_ready(QString::from(&downloads_json(&store.read_all())));
            return;
        }

        // No live torrent: this is a Retry of a failed or stale group. Attach
        // it running rather than merely flipping the flag, or the row would
        // say Downloading with nothing behind it.
        if live_id.is_none() {
            let qt_thread = self.qt_thread();
            let runtime = Arc::clone(&self.rust().runtime);
            let player = Arc::clone(&self.rust().player);
            let restoring = Arc::clone(&self.rust().restoring_downloads);
            mark_restoring(&restoring, &group_key);

            runtime.spawn(async move {
                let result = attach_group(&player, &records, false, &group_indices(&records)).await;
                let list_json = match result {
                    Ok(attached) => {
                        if let Some(bytes) = player.torrent_bytes(attached.torrent_id) {
                            if let Err(err) = save_cached_torrent(&attached.info_hash, &bytes) {
                                tracing::warn!("could not cache retried torrent: {err:#}");
                            }
                        }
                        let torrent_id = attached.torrent_id;
                        let hash = attached.info_hash.clone();
                        let cache = attached.cache_dir.display().to_string();
                        let file_count = attached.files.len() as u32;
                        if let Err(err) = store.mutate(|all| {
                            for record in all.iter_mut() {
                                if record.group_key() != group_key
                                    || record.status == DownloadStatus::Complete
                                {
                                    continue;
                                }
                                record.torrent_id = Some(torrent_id);
                                record.info_hash = Some(hash.clone());
                                record.cache_dir = Some(cache.clone());
                                record.torrent_file_count = Some(file_count);
                                record.status = DownloadStatus::Downloading;
                                record.user_paused = false;
                                record.error = None;
                            }
                        }) {
                            tracing::warn!("could not record retry for {group_key}: {err:#}");
                        }
                        downloads_json(&store.read_all())
                    }
                    Err(message) => {
                        if let Err(err) = store.mutate(|all| {
                            for record in all.iter_mut() {
                                if record.group_key() == group_key
                                    && record.status != DownloadStatus::Complete
                                {
                                    record.status = DownloadStatus::Failed;
                                    record.error = Some(message.clone());
                                }
                            }
                        }) {
                            tracing::warn!(
                                "could not record retry failure for {group_key}: {err:#}"
                            );
                        }
                        downloads_json(&store.read_all())
                    }
                };

                unmark_restoring(&restoring, &group_key);
                let _ = qt_thread
                    .queue(move |qobject| qobject.downloads_ready(QString::from(&list_json)));
            });
            return;
        }

        // A restored group is paused on disk. Flip only the rows that can run,
        // then let the queue planner enforce the concurrency limit.
        let Some(torrent_id) = live_id else {
            return;
        };
        let _ = store.mutate(|all| {
            for record in all.iter_mut() {
                if record.group_key() == group_key
                    && !matches!(
                        record.status,
                        DownloadStatus::Complete | DownloadStatus::Failed
                    )
                {
                    record.status = DownloadStatus::Downloading;
                    record.user_paused = false;
                    record.error = None;
                }
            }
        });

        let runtime = Arc::clone(&self.rust().runtime);
        let player = Arc::clone(&self.rust().player);
        runtime.spawn(async move {
            if let Err(err) = player.resume_torrent(torrent_id).await {
                tracing::warn!("could not resume torrent {torrent_id}: {err:#}");
            }
        });

        self.downloads_ready(QString::from(&downloads_json(&store.read_all())));
    }

    fn download_torrent(self: Pin<&mut Self>, url: QString, path: QString) {
        let url = url.to_string();
        let path = path.to_string();

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);

        runtime.spawn(async move {
            match download_to(&url, std::path::Path::new(&path)).await {
                Ok(()) => {
                    let _ = qt_thread.queue(|qobject| qobject.download_done());
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread
                        .queue(move |qobject| qobject.download_failed(QString::from(&message)));
                }
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

        // No sync here. This is a READ -- it answers "is a token stored" -- and
        // a read that starts a network pull cannot be called from a handler
        // that a pull itself fires. That is exactly the loop it used to form:
        // authStatus -> sync -> listChanged -> authStatus -> sync.
        //
        // Syncing is an explicit command now (`syncMediaList`), issued at
        // startup and after a sign-in.
    }

    fn sync_media_list(self: Pin<&mut Self>) {
        // A signed-out reader is skipped entirely: the pull needs a token, and
        // the push has nothing to send it with.
        if !self.rust().tokens.has_token() {
            return;
        }
        self.spawn_sync();
    }

    fn apply_auth_url(mut self: Pin<&mut Self>, url: QString) -> bool {
        let url = url.to_string();

        let Some(token) = flow::parse_token(&url) else {
            // Either not our scheme, or no usable token in the fragment. A
            // foreign URL is not an error -- deep links can come from
            // anywhere -- so this is logged at debug and otherwise ignored.
            tracing::debug!("ignored a deep link that carried no token");
            return false;
        };

        // Store BEFORE telling the provider. Holding a token that failed to
        // persist would work until the next restart and then appear to sign
        // the reader out for no reason -- worse than not signing in at all.
        // Same ordering as the Tauri implementation.
        if let Err(err) = self.rust().tokens.write(&token) {
            let message = format!("could not store the AniList token: {err:#}");
            tracing::error!("{message}");
            self.as_mut().auth_error(QString::from(&message));
            return false;
        }

        self.rust().anilist.set_token(Some(token));
        tracing::info!("signed in from a deep link");
        self.as_mut().auth_status_ready(true);

        // Reconcile straight away: the reader just signed in, and the list on
        // screen is either empty or belongs to whoever was signed in before.
        self.as_mut().spawn_sync();
        true
    }

    fn take_pending_auth_url(mut self: Pin<&mut Self>) -> bool {
        let slot = crate::single_instance::pending();
        let url = {
            let mut guard = slot.lock().expect("pending-auth mutex poisoned");
            guard.take()
        };
        let Some(url) = url else {
            return false;
        };
        self.as_mut().apply_auth_url(QString::from(&url))
    }

    fn init_deep_link_notify(self: Pin<&mut Self>) {
        let qt_thread = self.qt_thread();
        let callback: crate::single_instance::NotifyCallback = std::sync::Arc::new(move || {
            let thread = qt_thread.clone();
            let _ = thread.queue(|qobject| {
                qobject.pending_auth_arrived();
            });
        });
        crate::single_instance::set_notify(callback);
    }

    fn logout(self: Pin<&mut Self>) {
        // Logged at entry as well as on success: "Disconnect did nothing" has
        // two very different causes -- the click never reaching this function,
        // or the file removal failing -- and without these two lines there is
        // no way to tell them apart from the outside.
        tracing::info!(
            path = %self.rust().tokens.path().display(),
            "logout requested"
        );
        // Clear the store, then the live provider, so signing out takes effect
        // on the next request rather than at the next restart.
        if let Err(err) = self.rust().tokens.clear() {
            let message = err.to_string();
            tracing::error!(%message, "logout failed to clear the token store");
            self.auth_error(QString::from(&message));
            return;
        }
        self.rust().anilist.clear_token();
        tracing::info!("logout cleared the token");
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
                    let _ = qt_thread.queue(move |qobject| err(qobject, QString::from(&message)));
                }
            }
        });
    }

    fn continue_watching(self: Pin<&mut Self>, limit: i32) {
        let limit = limit_or_default(limit);
        // Served from the LOCAL list, not from AniList.
        //
        // Two reasons: it works while AniList is down (which it is, as often as
        // not), and it costs no request on a screen that is opened constantly.
        // The shelf is derived, not stored: `progress > 0` and newest first.
        let store = Arc::clone(&self.rust().media_list);
        // Local watch times, so the ORDER is right before the row is truncated.
        //
        // This matters because the truncation happens HERE, before the UI can
        // re-sort: AniList does not bump `updatedAt` for a same-progress
        // re-watch, so a work the reader just played could sort outside the
        // limit and never reach the row at all. Merging the local stamp into
        // the sort key keeps the two signals consistent with what the reader
        // expects ("the thing I just watched is first").
        let watched = self.rust().watch_positions.read_all();
        let items: Vec<kitsune_core::providers::anilist::ContinueWatchingItem> = {
            let mut rows: Vec<_> = store
                .visible()
                .into_iter()
                .filter(|entry| entry.progress > 0)
                .map(|entry| {
                    // `max` of the two: whichever source saw it more recently.
                    let local_at = watched
                        .get(&entry.media_id.to_string())
                        .map(|position| position.at)
                        .unwrap_or(0);
                    (entry.updated_at.max(local_at), entry)
                })
                .collect();
            // Most recently watched first, by EITHER signal.
            rows.sort_by(|a, b| b.0.cmp(&a.0));
            rows.into_iter()
                .take(limit as usize)
                .map(
                    |(_, entry)| kitsune_core::providers::anilist::ContinueWatchingItem {
                        anime: entry.anime,
                        progress: entry.progress,
                    },
                )
                .collect()
        };

        self.continue_watching_ready(QString::from(&json_or_empty(&items)));
    }

    fn user_list(self: Pin<&mut Self>) {
        // Read straight from the local store: no network, no cache, no wait.
        //
        // This is the change that makes My List work at all while AniList is
        // unreachable. The sync engine fills the store in the background; this
        // path only ever reads it.
        let store = Arc::clone(&self.rust().media_list);
        let entries: Vec<kitsune_core::providers::anilist::UserListEntry> = store
            .visible()
            .into_iter()
            .map(|entry| kitsune_core::providers::anilist::UserListEntry {
                anime: entry.anime,
                status: entry.status,
                progress: entry.progress,
                // AniList's entry id, or 0 for one that has never been pushed.
                // The UI only uses it to call back into delete, which now takes
                // the media id instead.
                entry_id: entry.anilist_entry_id.unwrap_or(0),
                added_at: Some(entry.added_at),
                // Carried through so a surface can order by "last changed"
                // without a second lookup.
                updated_at: Some(entry.updated_at),
            })
            .collect();

        self.user_list_ready(QString::from(&json_or_empty(&entries)));
    }

    fn set_list_entry(mut self: Pin<&mut Self>, media_id: i64, status: QString, progress: i32) {
        let status = status.to_string();
        // `progress <= 0` means "leave progress alone"; sending 0 would reset a
        // tracked position. This is the status-change case (Watching ->
        // Completed), which must not wipe how far the reader got.
        let progress = if progress > 0 {
            Some(progress as u32)
        } else {
            None
        };

        let parsed: ListStatus = match serde_json::from_value(serde_json::Value::String(status)) {
            Ok(s) => s,
            Err(err) => {
                let message = format!("unknown list status: {err}");
                self.auth_error(QString::from(&message));
                return;
            }
        };

        // The card the reader is looking at, so the new record carries a usable
        // snapshot. Absent for a write from a surface that has only the id, in
        // which case an existing record keeps its own snapshot.
        let anime = self.rust().provider.cached_partial(media_id);
        let store = Arc::clone(&self.rust().media_list);

        let existing = anime.as_ref().and_then(|a| store.get(a.provider, media_id));

        let Some(anime) = anime.or_else(|| existing.as_ref().map(|e| e.anime.clone())) else {
            // No snapshot anywhere. This used to give up with an error, which
            // made the write silently disappear: a card whose work was never
            // cached (a My List row, a Continue Watching row, a card evicted
            // from the partial cache) could not be added, and because the
            // function returned before `spawn_sync` no sync request was ever
            // issued either. The detail page worked only because opening it
            // had already cached the work.
            //
            // Fetch the work instead and finish the write when it lands.
            self.defer_list_entry(media_id, parsed, progress);
            return;
        };

        self.apply_list_entry(media_id, parsed, progress, anime);
    }

    /// Finish a list write once the work's snapshot is known.
    ///
    /// Shared by the fast path (the work was cached) and the deferred one (it
    /// had to be fetched), so the two cannot drift: both upsert the record and
    /// then emit `listChanged` and start a sync. Skipping either of those was
    /// what made a card's add button look dead -- the entry never reached
    /// AniList, because no sync was ever requested.
    fn apply_list_entry(
        mut self: Pin<&mut Self>,
        media_id: i64,
        status: ListStatus,
        progress: Option<u32>,
        anime: kitsune_core::types::Anime,
    ) {
        let store = Arc::clone(&self.rust().media_list);
        // Re-read the record: on the deferred path the store may have changed
        // while the fetch was in flight.
        let existing = store.get(anime.provider, media_id);

        let record = kitsune_core::list::local_change(anime, status, progress, existing);
        if let Err(err) = store.upsert(record) {
            let message = format!("could not save to your list: {err}");
            self.as_mut().auth_error(QString::from(&message));
            return;
        }

        // Tell the UI immediately: the write is already durable on disk, so
        // waiting for the network would only make the menu feel broken.
        self.as_mut().list_changed();
        self.as_mut().spawn_sync();
    }

    /// Fetch the work a list write named, then finish the write.
    ///
    /// Needed because a record carries a snapshot: the list pages render a
    /// cover and title with no network at all, so an entry without one would
    /// be a blank row. Every surface that shows a card warms the partial cache
    /// (browse, home shelves, recommendations, the detail page), but a My List
    /// row or a Continue Watching row was built from the local store, so a
    /// write from those had no snapshot and used to be dropped.
    fn defer_list_entry(
        mut self: Pin<&mut Self>,
        media_id: i64,
        status: ListStatus,
        progress: Option<u32>,
    ) {
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let provider = Arc::clone(&self.rust().provider);

        runtime.spawn(async move {
            let fetched = provider.by_id(media_id).await;
            let _ = qt_thread.queue(move |mut qobject| match fetched {
                Ok(Some(anime)) => qobject
                    .as_mut()
                    .apply_list_entry(media_id, status, progress, anime),
                // The id exists but no source could answer for it. Say so on
                // the write signal rather than on the read one.
                Ok(None) => qobject.as_mut().auth_error(QString::from(
                    "could not add to the list: that work was not found",
                )),
                Err(err) => {
                    let message = format!("could not add to the list: {err}");
                    qobject.as_mut().auth_error(QString::from(&message))
                }
            });
        });
    }

    fn delete_list_entry(mut self: Pin<&mut Self>, media_id: i64) {
        // `media_id`, not AniList's list-entry id: the store keys on
        // `(provider, media_id)`, and an entry that has never been pushed has
        // no list-entry id at all. It is marked deleted (a tombstone) rather
        // than removed, so the delete can be retried and a pull cannot
        // resurrect the work.
        let store = Arc::clone(&self.rust().media_list);

        let provider = self
            .rust()
            .provider
            .cached_partial(media_id)
            .map(|a| a.provider)
            .or_else(|| store.get(ProviderId::AniList, media_id).map(|e| e.provider))
            .or_else(|| store.get(ProviderId::Kitsu, media_id).map(|e| e.provider))
            .unwrap_or(ProviderId::AniList);

        if let Err(err) = store.mark_deleted(provider, media_id) {
            let message = format!("could not remove from your list: {err}");
            self.auth_error(QString::from(&message));
            return;
        }

        self.as_mut().list_changed();
        self.as_mut().spawn_sync();
    }

    /// Run one sync pass in the background.
    ///
    /// Never blocks the caller and never surfaces an error: a sync failure
    /// leaves the dirty entries queued for the next attempt, which is the
    /// intended behaviour offline.
    fn spawn_sync(self: Pin<&mut Self>) {
        // Signed out: there is no token to push with and the pull needs one, so
        // a pass would be pure overhead. `sync_media_list` has always guarded
        // this; the write paths reach `spawn_sync` directly and did not, which
        // meant every offline add fired a doomed request.
        if !self.rust().tokens.has_token() {
            return;
        }

        // Collapse overlapping requests into one pass.
        //
        // `authStatus()` is called by the home page, My List and the detail
        // page, and each of those used to start a FULL sync -- three pulls of
        // the same 269-entry list at startup, which is exactly the rate-limit
        // pressure the local store exists to relieve.
        //
        // `compare_exchange` rather than a flag check: two Qt-thread callers
        // cannot interleave here, but a later refactor that spawns this from
        // an async context would otherwise race.
        if self
            .rust()
            .sync_in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let sync = Arc::clone(&self.rust().list_sync);
        let in_flight = Arc::clone(&self.rust().sync_in_flight);
        runtime.spawn(async move {
            let report = sync.run().await;
            // One line per pass rather than per entry: a long outage would
            // otherwise fill the log with the same failure.
            tracing::info!("list sync: {}", report.summary());

            // THE FIRST-RUN RACE: the page asks for the list as soon as it
            // opens, which on a fresh install is BEFORE this pull has landed.
            // It read an empty store and had no reason to ask again, so the
            // list stayed blank until the app was restarted.
            //
            // Re-notifying after a pull that actually succeeded closes that
            // gap. A failed pull changes nothing, so there is nothing to
            // re-read and no signal is sent.
            // Cleared BEFORE notifying: a page that reacts to
            // `listChanged` by starting another sync must not be refused
            // merely because this one has not returned yet.
            in_flight.store(false, Ordering::Release);

            if report.pulled_ok {
                let _ = qt_thread.queue(|qobject| qobject.list_changed());
            }
        });
    }

    fn get_list_entry(self: Pin<&mut Self>, media_id: i64) {
        // The STORE answers first: it is the reader's own list, and it is
        // current even when AniList has not heard about the latest change.
        let store = Arc::clone(&self.rust().media_list);
        let provider = self
            .rust()
            .provider
            .cached_partial(media_id)
            .map(|a| a.provider)
            .unwrap_or(ProviderId::AniList);

        if let Some(entry) = store.get(provider, media_id) {
            let payload = if entry.deleted {
                None
            } else {
                Some(kitsune_core::providers::anilist::ListEntry {
                    id: entry.anilist_entry_id.unwrap_or(0),
                    status: entry.status,
                    progress: entry.progress,
                })
            };
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "null".into());
            self.list_entry_ready(media_id, QString::from(&json));
            return;
        }

        // Not in the store. The detail lookup may already have fetched it, so
        // answer from that copy rather than making a request.
        if let Some(known) = self.rust().provider.cached_entry(media_id) {
            let json = serde_json::to_string(&known).unwrap_or_else(|_| "null".into());
            self.list_entry_ready(media_id, QString::from(&json));
            return;
        }

        // A plain runtime spawn rather than `spawn_anilist`: that helper reports
        // failures on `authError`, which is broadcast to every listener in the
        // app. Reading an entry is not an account operation, so a 429 here used
        // to surface in the settings footer (and anywhere else listening) for a
        // page the reader was not even on.
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let anilist = Arc::clone(&self.rust().anilist);

        runtime.spawn(async move {
            match anilist.list_entry(media_id).await {
                // `None` -> JSON `null`, which QML reads as "not on list".
                Ok(entry) => {
                    let json = serde_json::to_string(&entry).unwrap_or_else(|_| "null".into());
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.list_entry_ready(media_id, QString::from(&json))
                    });
                }
                // Deliberately silent. The entry is a decoration on the detail
                // page's status menu; a failed read leaves it showing nothing
                // rather than raising an app-wide error the reader cannot act
                // on. `by_id` already delivers the entry in the common case, so
                // this path is a cache-miss fallback.
                Err(err) => {
                    tracing::warn!("could not read the list entry for {media_id}: {err:#}");
                }
            }
        });
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

    // --- Local resume state ------------------------------------------------

    /// Record that `anime_id` was opened. Synchronous file write, so it runs on
    /// the Qt thread directly -- no runtime hop for a few bytes to disk.
    fn record_last_played(self: Pin<&mut Self>, anime_id: i64, episode: i32, provider: QString) {
        // `episode <= 0` means "no episode" (a film, or a play with none picked).
        let episode = if episode > 0 {
            Some(episode as u32)
        } else {
            None
        };
        let provider = provider.to_string();
        // A failure here is not worth surfacing: the record is a hint, and a
        // missed write only means the row keeps its previous order.
        //
        // The namespace is stored WITH the id so the disc can resume from the
        // source that issued it rather than assuming AniList.
        let _ = self
            .rust()
            .last_played
            .write(anime_id, Some(&provider), episode);
        // Nudge the disc and the Continue Watching row to re-read, so a just
        // started (or just finished) play updates the resume time live.
        self.library_changed();
    }

    fn get_last_played(self: Pin<&mut Self>) {
        let json = self
            .rust()
            .last_played
            .read()
            .and_then(|record| serde_json::to_string(&record).ok())
            .unwrap_or_else(|| "null".to_string());
        self.last_played_ready(QString::from(&json));
    }

    fn record_watch_position(
        self: Pin<&mut Self>,
        anime_id: i64,
        episode: i32,
        position_seconds: f64,
        duration_seconds: f64,
    ) {
        let episode = if episode > 0 {
            Some(episode as u32)
        } else {
            None
        };
        let _ = self.rust().watch_positions.write(
            anime_id,
            episode,
            position_seconds,
            duration_seconds,
        );
        // Deliberately no `library_changed()` here: this fires every few
        // seconds during playback, and the home row's handler would then hit
        // AniList on each tick. The modal saves the final position on close
        // and the disc re-reads when it is shown again, which is when the
        // updated time actually needs to appear.
    }

    fn get_watch_positions(self: Pin<&mut Self>) {
        let entries = self.rust().watch_positions.read_all();
        let json = serde_json::to_string(&entries).unwrap_or_else(|_| "{}".to_string());
        self.watch_positions_ready(QString::from(&json));
    }

    fn record_last_release(
        self: Pin<&mut Self>,
        anime_id: i64,
        magnet: QString,
        file_idx: i32,
        name: QString,
    ) {
        let magnet = magnet.to_string();
        if magnet.is_empty() {
            return;
        }
        let idx = if file_idx < 0 { 0 } else { file_idx as u32 };
        let _ = self
            .rust()
            .last_releases
            .write(anime_id, &magnet, idx, &name.to_string());
        // Any write to local resume state nudges the disc and home row to
        // re-read, so a just-watched work appears without a restart.
        self.library_changed();
    }

    fn get_last_release(self: Pin<&mut Self>, anime_id: i64) {
        let json = self
            .rust()
            .last_releases
            .read(anime_id)
            .and_then(|release| serde_json::to_string(&release).ok())
            .unwrap_or_else(|| "null".to_string());
        self.last_release_ready(QString::from(&json));
    }
}

/// Serialise to a JSON array, falling back to `[]` so a serialisation failure
/// never drops the whole response (the UI already handles an empty list).
/// The episode catalogue as it crosses from the backend to QML.
///
/// `{ info, aired }` rather than a bare array: the aired count is what stops an
/// airing show from offering episodes the reader cannot watch yet, and it has to
/// survive the trip because only the provider knows it.
///
/// At module scope because cxx-qt parses the `#[qobject]` block and rejects a
/// `struct` declared inside a method body.
#[derive(serde::Deserialize, Default)]
struct EpisodeCatalogueWire {
    #[serde(default)]
    info: Vec<EpisodeInfo>,
    #[serde(default)]
    aired: u32,
    /// The episode about to air, when the work is still releasing.
    ///
    /// Travels with the catalogue rather than on its own signal: the grid is
    /// built in one call, and a separate `nextAiring` property would be a
    /// second source of truth for the same list.
    #[serde(rename = "nextAiring", default)]
    next_airing: Option<kitsune_core::types::NextAiringEpisode>,
}

/// Parse an `Anime.provider` spelling into a [`ProviderId`].
///
/// Unknown values fall back to AniList rather than failing: every caller that
/// predates this parameter meant AniList, and an unparseable value should not
/// blank the detail page.
fn provider_id_from(raw: &str) -> ProviderId {
    match raw.trim() {
        "kitsu" => ProviderId::Kitsu,
        "tmdb" => ProviderId::Tmdb,
        _ => ProviderId::AniList,
    }
}

/// The inverse of [`provider_id_from`], for echoing a request's namespace back
/// to QML. Spelled exactly like `Anime.provider` so a page can compare it
/// against its own `animeProvider` property without translating.
fn provider_namespace(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Kitsu => "kitsu",
        ProviderId::Tmdb => "tmdb",
        _ => "anilist",
    }
}

fn json_or_empty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}

/// How many trending titles may have a banner cross-filled on the home screen.
///
/// The hero shows five. Each fill is up to two requests, so this caps the home
/// load at ten extra calls in the worst case -- and only for works that are
/// actually missing a banner.
const HERO_BANNER_FILLS: usize = 5;

/// Try to fill a missing banner image from the OTHER metadata source.
///
/// A work is fetched from whichever source issued its id, and that source may
/// simply not have a banner: Kitsu's equivalent field is absent on plenty of
/// titles, and AniList does not cover everything Kitsu does. The detail page
/// falls back to the cover when no banner exists, which crops a portrait poster
/// across a wide strip -- worth one request to avoid.
///
/// Returns `None` when the other source has nothing, or cannot be reached. A
/// failure here is deliberately SILENT and swallowed: the banner is decoration,
/// and the page renders correctly without it, so there is nothing to report.
async fn borrow_banner(
    anilist: &kitsune_core::providers::AniListProvider,
    kitsu: &kitsune_core::providers::KitsuProvider,
    anime: &kitsune_core::types::Anime,
) -> Option<String> {
    use kitsune_core::types::ProviderId;

    match anime.provider {
        // Came from AniList, so ask Kitsu -- but only if the mapping exists.
        // A Kitsu lookup by a bare AniList id would fetch whatever work holds
        // that number, and paint a stranger's banner.
        ProviderId::AniList => {
            let kitsu_id = kitsu.kitsu_id_for_anilist(anime.id).await.ok().flatten()?;
            kitsu.banner_for(kitsu_id).await.ok().flatten()
        }
        // Came from Kitsu, so ask AniList. `id_mal` is useless here (that is
        // MyAnimeList's namespace); the bridge is Kitsu's own AniList mapping.
        ProviderId::Kitsu => {
            let anilist_id = kitsu.anilist_id_for_kitsu(anime.id).await.ok().flatten()?;
            anilist.banner_for(anilist_id).await.ok().flatten()
        }
        // A source with no counterpart to borrow from.
        _ => None,
    }
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

/// The wire string for a [`HealthBadge`], matching the serde shape the Svelte
/// frontend already consumed ("green" | "yellow" | "red").
fn badge_literal(badge: HealthBadge) -> &'static str {
    match badge {
        HealthBadge::Green => "green",
        HealthBadge::Yellow => "yellow",
        HealthBadge::Red => "red",
    }
}

#[cfg(test)]
mod downloaded_for_anime_tests {
    use super::*;
    use kitsune_core::downloads::DownloadPriority;

    fn sample_record(id: u64) -> DownloadRecord {
        DownloadRecord {
            id,
            anime_id: 1,
            provider: "anilist".to_string(),
            episode: Some(1),
            title: "Kitsune".to_string(),
            cover_image: None,
            episode_title: None,
            release_title: "[GSC] Show - 01".to_string(),
            torrent_id: None,
            magnet: None,
            torrent_url: None,
            info_hash: None,
            cache_dir: None,
            file_idx: 0,
            file_name: "Show - 01.mkv".to_string(),
            status: DownloadStatus::Complete,
            progress_bytes: 1,
            total_bytes: 1,
            release_seeders: None,
            output_path: String::new(),
            added_at: 0,
            completed_at: None,
            priority: DownloadPriority::Normal,
            user_paused: false,
            season: None,
            library_dir: None,
            torrent_file_count: None,
            error: None,
        }
    }

    fn with_temp_file() -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join("kitsune-downloaded-for-anime-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("show-01-{}.mkv", std::process::id()));
        std::fs::write(&path, b"x").unwrap();
        (dir, path)
    }

    #[test]
    fn keeps_only_existing_complete_file_for_same_anime_provider() {
        let (_dir, path) = with_temp_file();
        let mut on_disk = sample_record(1);
        on_disk.output_path = path.display().to_string();
        let mut other_anime = sample_record(2);
        other_anime.anime_id = 2;
        other_anime.output_path = path.display().to_string();
        let mut other_provider = sample_record(3);
        other_provider.provider = "kitsu".to_string();
        other_provider.output_path = path.display().to_string();
        let mut missing = sample_record(4);
        missing.file_name = "Show - 01.mkv".to_string();
        missing.output_path =
            "/nonexistent/kitsune-download-should-not-exist/show-01.mkv".to_string();

        let views = downloaded_views_for(
            &[on_disk, other_anime, other_provider, missing],
            1,
            " anilist ",
        );

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].record_id, 1);
        assert_eq!(views[0].episode, Some(1));
        assert_eq!(views[0].release_title, "[GSC] Show - 01");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn drops_file_after_reader_deletes_it() {
        let (dir, path) = with_temp_file();
        let mut record = sample_record(1);
        record.output_path = path.display().to_string();
        assert_eq!(downloaded_views_for(&[record], 1, "anilist").len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
        let mut record = sample_record(1);
        record.output_path = path.display().to_string();
        assert_eq!(downloaded_views_for(&[record], 1, "anilist").len(), 0);
    }
}
