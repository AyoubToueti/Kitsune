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
        fn get_list_paged(
            self: Pin<&mut Self>,
            filter: QString,
            page: i32,
            per_page: i32,
        );

        /// Every home shelf (trending + the five curated blocks) in ONE
        /// request. Emits `homeReady(json)` with a `HomeShelves` object, or
        /// `homeFailed(message)`.
        #[qinvokable]
        #[cxx_name = "getHome"]
        fn get_home(self: Pin<&mut Self>);

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
        fn get_schedule_tagged(
            self: Pin<&mut Self>,
            tag: QString,
            from: i64,
            to: i64,
            limit: i32,
        );

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
        fn pick_file(
            self: Pin<&mut Self>,
            files_json: QString,
            episode: i32,
            offset: i32,
        ) -> i32;

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

        /// Fetch a `.torrent` from an indexer and write it to `path`.
        ///
        /// The fetch happens here rather than in QML: Nyaa cannot be reached
        /// from a web context (CORS), and a native request writes the bytes
        /// straight to disk.
        #[qinvokable]
        #[cxx_name = "downloadTorrent"]
        fn download_torrent(self: Pin<&mut Self>, url: QString, path: QString);

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

        // --- Local resume state ----------------------------------------------

        /// Record that `anime_id` was opened, with the namespace it came from.
        ///
        /// `provider` is stored alongside the id: a resume must ask the source
        /// that issued it, and a bare id cannot say which that is.
        #[qinvokable]
        #[cxx_name = "recordLastPlayed"]
        fn record_last_played(
            self: Pin<&mut Self>,
            anime_id: i64,
            episode: i32,
            provider: QString,
        );

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
        #[qsignal]
        #[cxx_name = "animeFailed"]
        fn anime_failed(self: Pin<&mut Self>, message: QString);

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

        #[qsignal]
        #[cxx_name = "episodesReady"]
        fn episodes_ready(self: Pin<&mut Self>, json: QString);
        #[qsignal]
        #[cxx_name = "episodesFailed"]
        fn episodes_failed(self: Pin<&mut Self>, message: QString);

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

        /// A `.torrent` was written to disk.
        #[qsignal]
        #[cxx_name = "downloadDone"]
        fn download_done(self: Pin<&mut Self>);
        #[qsignal]
        #[cxx_name = "downloadFailed"]
        fn download_failed(self: Pin<&mut Self>, message: QString);
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

use kitsune_core::auth::{TokenStore, flow};
use kitsune_core::diagnostics::speedtest::{SpeedTestProgress, SpeedTester};
use kitsune_core::indexer::{
    download_to, probe_rank, probe_releases_standalone, rank, HealthBadge, IndexerRegistry,
    ProbeOutcome, ReleaseCache, ReleaseRequest, SearchMode,
};
use kitsune_core::providers::cache::ttl;
use kitsune_core::auth::LastPlayedStore;
use kitsune_core::list::{ListSync, MediaListStore};
use kitsune_core::player::state::PlayerState;
use kitsune_core::player::{LastReleaseStore, WatchPositionStore};
use kitsune_core::providers::{
    AniListProvider, AnimeProvider, CachedProvider, FailoverProvider,
    KitsuProvider,
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
                    // Logged as well as handed to QML. A page that fails to load
                    // shows an error block the reader may not report, and before
                    // this a metadata failure left NO trace in the log at all --
                    // which is exactly why "nothing renders" was undiagnosable.
                    tracing::error!("metadata request failed: {message}");
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

    /// One page of a curated list, as an `AnimePage` JSON object. Backs the
    /// scrolling "Top 100": `pageInfo` says whether another page follows.
    fn get_list_paged(self: Pin<&mut Self>, filter: QString, page: i32, per_page: i32) {
        let filter = filter.to_string();
        let page = page.max(1) as u32;
        let per_page = limit_or_default(per_page);
        self.spawn_metadata(
            move |provider| async move {
                let filter: ListFilter = serde_json::from_value(serde_json::Value::String(
                    filter,
                ))
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
    fn get_home(self: Pin<&mut Self>) {
        self.spawn_metadata(
            move |provider| async move {
                provider
                    .home_shelves()
                    .await
                    .map(|shelves| json_or_empty(&shelves))
                    .map_err(|err| err.to_string())
            },
            Self::home_ready,
            Self::home_failed,
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
                Ok((anime, entry)) => {
                    let json =
                        serde_json::to_string(&anime).unwrap_or_else(|_| "null".into());
                    // `None` -> JSON `null`, which QML reads as "not on list".
                    let entry_json =
                        serde_json::to_string(&entry).unwrap_or_else(|_| "null".into());

                    let _ = qt_thread.clone().queue(move |qobject| {
                        qobject.anime_ready(QString::from(&json))
                    });
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.list_entry_ready(QString::from(&entry_json))
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread
                        .queue(move |qobject| qobject.anime_failed(QString::from(&message)));
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
                        let _ = qt_thread
                            .queue(move |qobject| {
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
    fn get_schedule_tagged(
        self: Pin<&mut Self>,
        tag: QString,
        from: i64,
        to: i64,
        limit: i32,
    ) {
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
                        qobject.tagged_schedule_ready(
                            QString::from(&tag),
                            QString::from(&json),
                        )
                    });
                }
                Err(err) => {
                    let message = err.to_string();
                    let _ = qt_thread.queue(move |qobject| {
                        qobject.tagged_schedule_failed(
                            QString::from(&tag),
                            QString::from(&message),
                        )
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
        let qt_thread = self.qt_thread();
        let runtime = Arc::clone(&self.rust().runtime);
        let kitsu = Arc::clone(&self.rust().kitsu);

        runtime.spawn(async move {
            // A Kitsu id addresses episodes directly; an AniList id has to be
            // bridged through Kitsu's mapping table first.
            let resolved = if provider == "kitsu" {
                Ok(Some(id))
            } else {
                kitsu.kitsu_id_for_anilist(id).await
            };

            let result = match resolved {
                Ok(Some(kitsu_id)) => kitsu
                    .episodes(kitsu_id)
                    .await
                    .map(|catalogue| {
                        // An empty `aired` with a non-empty list means the
                        // provider withheld the count; the list length is then
                        // the honest floor rather than zero.
                        let aired = if catalogue.aired == 0 {
                            catalogue.info.len() as u32
                        } else {
                            catalogue.aired
                        };
                        serde_json::json!({ "info": catalogue.info, "aired": aired })
                    }),
                // No mapping means no episode data. Reported as an empty
                // catalogue rather than an error, so the page falls back to
                // synthesising from what the provider already told us.
                Ok(None) => Ok(serde_json::json!({ "info": [], "aired": 0 })),
                Err(err) => Err(err),
            };

            match result {
                Ok(payload) => {
                    let json = payload.to_string();
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
        let catalogue: EpisodeCatalogueWire =
            serde_json::from_str(&info_json).unwrap_or_default();
        let info = catalogue.info;

        let cover = cover_image.to_string();
        let status = status.to_string();
        let episodes = kitsune_core::episodes::episodes_for_facts(
            (episode_count > 0).then_some(episode_count as u32),
            (!status.is_empty()).then_some(status.as_str()),
            (!cover.is_empty()).then_some(cover.as_str()),
            &info,
            (catalogue.aired > 0).then_some(catalogue.aired),
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
            let _ = qt_thread.queue(move |qobject| {
                qobject.releases_ready(tag, QString::from(&json))
            });
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
                let _ = qt_thread.clone().queue(move |qobject| {
                    qobject.probe_result(tag, index, badge, score)
                });
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
                let _ = thread_for_events.clone().queue(move |qobject| {
                    qobject.probe_result(tag, index, badge, score)
                });
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

    /// Fetch a `.torrent` from an indexer and write it to `path`.
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
                .map(|(_, entry)| {
                    kitsune_core::providers::anilist::ContinueWatchingItem {
                        anime: entry.anime,
                        progress: entry.progress,
                    }
                })
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

    fn set_list_entry(
        mut self: Pin<&mut Self>,
        media_id: i64,
        status: QString,
        progress: i32,
    ) {
        let status = status.to_string();
        // `progress <= 0` means "leave progress alone"; sending 0 would reset a
        // tracked position. This is the status-change case (Watching ->
        // Completed), which must not wipe how far the reader got.
        let progress = if progress > 0 { Some(progress as u32) } else { None };

        let parsed: ListStatus = match serde_json::from_value(serde_json::Value::String(
            status,
        )) {
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

        let existing = anime
            .as_ref()
            .and_then(|a| store.get(a.provider, media_id));

        let Some(anime) = anime.or_else(|| existing.as_ref().map(|e| e.anime.clone())) else {
            // No snapshot anywhere: there is nothing to render a row from, and
            // fetching one would be a network round trip the UI is waiting on.
            self.auth_error(QString::from(
                "could not add to the list: the work is not loaded yet",
            ));
            return;
        };

        let record = kitsune_core::list::local_change(anime, parsed, progress, existing);
        if let Err(err) = store.upsert(record) {
            let message = format!("could not save to your list: {err}");
            self.auth_error(QString::from(&message));
            return;
        }

        // Tell the UI immediately: the write is already durable on disk, so
        // waiting for the network would only make the menu feel broken.
        self.as_mut().list_changed();
        self.as_mut().spawn_sync();
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
            .or_else(|| {
                store
                    .get(ProviderId::AniList, media_id)
                    .map(|e| e.provider)
            })
            .or_else(|| {
                store
                    .get(ProviderId::Kitsu, media_id)
                    .map(|e| e.provider)
            })
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
            self.list_entry_ready(QString::from(&json));
            return;
        }

        // Not in the store. The detail lookup may already have fetched it, so
        // answer from that copy rather than making a request.
        if let Some(known) = self.rust().provider.cached_entry(media_id) {
            let json = serde_json::to_string(&known).unwrap_or_else(|_| "null".into());
            self.list_entry_ready(QString::from(&json));
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
                    let json =
                        serde_json::to_string(&entry).unwrap_or_else(|_| "null".into());
                    let _ = qt_thread
                        .queue(move |qobject| qobject.list_entry_ready(QString::from(&json)));
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
    fn record_last_played(
        self: Pin<&mut Self>,
        anime_id: i64,
        episode: i32,
        provider: QString,
    ) {
        // `episode <= 0` means "no episode" (a film, or a play with none picked).
        let episode = if episode > 0 { Some(episode as u32) } else { None };
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
        let episode = if episode > 0 { Some(episode as u32) } else { None };
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

/// The wire string for a [`HealthBadge`], matching the serde shape the Svelte
/// frontend already consumed ("green" | "yellow" | "red").
fn badge_literal(badge: HealthBadge) -> &'static str {
    match badge {
        HealthBadge::Green => "green",
        HealthBadge::Yellow => "yellow",
        HealthBadge::Red => "red",
    }
}