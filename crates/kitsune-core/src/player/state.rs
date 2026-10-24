//! The player session: a torrent engine, its stream bridge, and the chosen
//! external player.
//!
//! The session is started lazily on the first command that needs it, so
//! launching the app does not bind sockets or join the DHT until the reader
//! actually plays something.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

use crate::settings::SettingsStore;
use crate::torrent::{api_for, EngineConfig, HttpBridge, TorrentEngine, TorrentProgress};

use super::resolve_player;

/// How long to wait for a torrent's metadata before giving up.
///
/// A local `.torrent` carries its own metadata, so this normally resolves on
/// the first poll. The wait exists so a magnet that never finds peers fails
/// with a message rather than hanging the command forever.
const METADATA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// How often to re-check for metadata while waiting.
const METADATA_POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// How long to wait for a torrent to finish initializing before streaming.
///
/// librqbit's stream endpoint returns HTTP 500 while a torrent is still
/// `Initializing` (it hashes any existing data on add/resume). mpv treats a
/// 500 as fatal and does not retry, so a stream URL handed over during that
/// window plays nothing. Handing over only once the torrent is live removes
/// the race entirely. The window is short (a re-hash of existing pieces), so a
/// few seconds is ample; the cap keeps a torrent that never goes live from
/// hanging the command.
const LIVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// How often to re-check whether a torrent has gone live.
const LIVE_POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// Bytes to request when warming the HEAD of a stream.
///
/// The head only exists to make the first pieces high-priority so the demuxer
/// can identify the container; a couple of MiB is plenty. It is BOUNDED on
/// purpose: the earlier `bytes=0-` meant "everything from here to EOF", so a
/// cold start had to download the full drain cap before a URL was returned --
/// measured at 13.9s for 8.7MiB, which dominated the start latency.
const WARM_HEAD_BYTES: u64 = 2 * 1024 * 1024;

/// Bytes to request when warming the TAIL of a stream.
///
/// The tail must cover the seek index: Matroska `Cues` for a two-hour episode
/// can run to hundreds of KiB, and mpv seeks there before it will play past
/// the first few seconds. 4MiB is comfortably above one piece plus the index.
const WARM_TAIL_BYTES: u64 = 4 * 1024 * 1024;

/// Cap on the HEAD request.
///
/// This one IS awaited, so it directly delays the URL. Kept tight for that
/// reason: a cold swarm should cost a few seconds at most, not the full
/// drain cap.
const WARM_HEAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// Cap on one warm-up request.
///
/// A warm-up must never delay playback indefinitely, so a dead swarm costs a
/// bounded wait rather than a hang. It is longer than it looks like it should
/// be because the warm-up now really downloads: the tail alone is up to
/// [`WARM_BYTES`] (4 MiB), which at a modest 1 MiB/s is four seconds on its
/// own. Five seconds used to be enough when only headers were awaited; it
/// would have cut a legitimate tail fetch off mid-piece.
const WARM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// How much of a warm-up response body to actually read.
///
/// The warm-up must FETCH bytes, not merely request them: awaiting headers
/// alone pulled nothing and left the player blocked on the file's tail index
/// (see [`fetch_first_chunk`]). But reading the whole remainder of the file
/// would be worse, so the drain stops here.
///
/// Matches [`WARM_TAIL_BYTES`] so a tail request of that size can complete
/// fully; it is only a backstop for the unbounded head range on a tiny file.
const WARM_DRAIN_BYTES: u64 = WARM_TAIL_BYTES;

/// One file inside a torrent, as the frontend picks from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFile {
    /// Index the stream URL is built from.
    pub idx: usize,
    pub name: String,
    pub length_bytes: u64,
}

/// A torrent that has been added, with the files it resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentHandle {
    /// Id the stream URL is built from.
    pub id: usize,
    pub files: Vec<TorrentFile>,
}

/// A live torrent session and its stream bridge.
struct Session {
    engine: TorrentEngine,
    bridge: HttpBridge,
}

/// Where downloaded pieces are written.
///
/// Overridable for tests and for a user who wants downloads on another disk;
/// the default is a directory under the OS temp, which is writable
/// everywhere and needs no permissions prompt.
fn download_dir() -> PathBuf {
    std::env::var_os("KITSUNE_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("kitsune-downloads"))
}

/// Which torrents an external player is reading, and which removals wait on one.
///
/// The watch page removes its torrent when it is left. That is wrong while an
/// external player streams from the same torrent: mpv reads through the bridge
/// independently, so removing the torrent makes its next range request fail. A
/// launch therefore takes a hold, and removal is deferred until the last holder
/// releases.
///
/// Pure and synchronous, so the whole policy is unit tested without a session,
/// a process, or a runtime.
#[derive(Debug, Default)]
struct Holds {
    /// Torrent id -> how many players are reading it.
    counts: HashMap<usize, usize>,
    /// Removals asked for while a player still held the torrent.
    pending: HashSet<usize>,
}

impl Holds {
    /// Record that a player is now reading `id`.
    fn hold(&mut self, id: usize) {
        *self.counts.entry(id).or_insert(0) += 1;
    }

    /// Record that one player stopped reading `id`.
    ///
    /// True when this was the last holder AND a removal was already asked for,
    /// which is the caller's signal to remove the torrent now.
    fn release(&mut self, id: usize) -> bool {
        let Some(count) = self.counts.get_mut(&id) else {
            // A release with no matching hold. Removing here would delete a
            // torrent nobody had finished with, so it is ignored.
            return false;
        };

        *count -= 1;
        if *count > 0 {
            return false;
        }

        self.counts.remove(&id);
        self.pending.remove(&id)
    }

    /// Ask for `id` to be removed.
    ///
    /// True when nothing holds it, meaning the caller removes it now. Otherwise
    /// the request is remembered and [`Holds::release`] reports it later.
    fn request_removal(&mut self, id: usize) -> bool {
        if self.counts.contains_key(&id) {
            self.pending.insert(id);
            false
        } else {
            true
        }
    }

    /// Forget a pending removal for `id`.
    ///
    /// Called when a torrent is added, because librqbit returns the SAME id for
    /// a torrent it already manages. Without this, replaying an episode whose
    /// player had already closed would leave the stale flag set, and the next
    /// player exit would delete the torrent the reader is watching.
    fn cancel_pending(&mut self, id: usize) {
        self.pending.remove(&id);
    }
}

/// Player state shared across the Tauri commands.
///
/// A newtype over `Arc<Inner>` so a command can clone it into a background
/// task. Tauri hands commands a borrow of the managed state, which cannot be
/// moved into `tokio::spawn`; the `Arc` is what lets the task waiting for an
/// external player to exit outlive the command that launched it.
#[derive(Clone)]
pub struct PlayerState {
    inner: Arc<Inner>,
}

/// The state behind [`PlayerState`], shared by every clone of it.
struct Inner {
    /// The torrent session, started on first use.
    ///
    /// A `OnceCell` rather than a mutex-wrapped option: starting awaits, and
    /// two commands racing to start would otherwise bind two bridges. The
    /// cell also makes a failed start retryable, since it stays empty.
    session: OnceCell<Session>,
    /// The chosen external player.
    ///
    /// A plain mutex: it is never held across an await, so the async one
    /// would only add cost.
    player: Mutex<String>,
    /// Deferred removals, keyed by torrent id.
    ///
    /// A plain mutex for the same reason: every operation on it is synchronous
    /// and finishes before the caller awaits.
    holds: Mutex<Holds>,
    /// The reader's preferences, when the app supplied a store.
    ///
    /// `None` in tests and for a bare [`PlayerState::new`]: everything then
    /// falls back to the built-in defaults, which keeps the existing suite
    /// hermetic and free of disk access.
    settings: Option<Arc<SettingsStore>>,
    /// `(torrent, file)` pairs with a warm-up already in flight.
    ///
    /// The watch page calls `prepare_stream` several times for the same file
    /// (measured: four calls inside one second). Each one used to spawn its
    /// own head AND tail reader, and librqbit shares the piece queue EQUALLY
    /// between open readers (`iter_next_pieces` round-robins them), so nine
    /// readers turned the tail -- the one range mpv blocks on for the Matroska
    /// index -- into 1/9th priority. That is why a torrent with 91MB down and
    /// 23 healthy peers still sat at `00:00 / 00:00`.
    ///
    /// Synchronous and never held across an await, so a plain mutex.
    warmups: Mutex<HashSet<(usize, usize)>>,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                session: OnceCell::new(),
                player: Mutex::new(resolve_player(None)),
                holds: Mutex::new(Holds::default()),
                settings: None,
                warmups: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// A state that reads the reader's preferences from `store`.
    ///
    /// The chosen player is seeded from the store so the very first launch --
    /// before the settings page has been opened -- already uses the reader's
    /// choice. The download directory and extra args are read live on each
    /// use, so changing them takes effect without a restart.
    pub fn with_settings(store: Arc<SettingsStore>) -> Self {
        let player = resolve_player(Some(&store.read().player));
        Self {
            inner: Arc::new(Inner {
                session: OnceCell::new(),
                player: Mutex::new(player),
                holds: Mutex::new(Holds::default()),
                settings: Some(store),
                warmups: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// The chosen external player.
    pub fn player(&self) -> String {
        self.inner
            .player
            .lock()
            .expect("player mutex poisoned")
            .clone()
    }

    /// Choose the external player. A blank name restores the default.
    pub fn set_player(&self, name: &str) {
        *self.inner.player.lock().expect("player mutex poisoned") = resolve_player(Some(name));
    }

    /// The external player to launch, read live from settings when a store is
    /// present.
    ///
    /// Deliberately reads the store rather than the in-memory `player` mutex:
    /// that mutex is seeded ONCE at startup, so a choice made in the settings
    /// page never reached the launch path and only took effect after a restart.
    /// The store is the single source of truth; the mutex remains only as the
    /// fallback for a `PlayerState` built without a store (tests).
    pub fn resolved_player(&self) -> String {
        match self.inner.settings.as_ref() {
            Some(store) => resolve_player(Some(&store.read().player)),
            None => self.player(),
        }
    }

    /// Extra arguments for the player, from settings when a store is present.
    ///
    /// Public because the external-player launcher lives in the Tauri crate
    /// (see `src-tauri/src/player/external.rs`), not here.
    pub fn player_args(&self) -> Vec<String> {
        self.inner
            .settings
            .as_ref()
            .map(|store| store.read().player_args)
            .unwrap_or_default()
    }

    /// Where downloaded pieces are written.
    ///
    /// The reader's explicit choice wins; otherwise `KITSUNE_DOWNLOAD_DIR`,
    /// otherwise the OS temp directory. Read on each session start, so a change
    /// in settings applies to the next torrent.
    fn resolved_download_dir(&self) -> PathBuf {
        if let Some(dir) = self
            .inner
            .settings
            .as_ref()
            .and_then(|store| store.read().download_dir)
            .filter(|dir| !dir.trim().is_empty())
        {
            return PathBuf::from(dir);
        }
        download_dir()
    }

    /// Start the torrent engine and stream bridge, once.
    async fn start_session(download_dir: PathBuf) -> Result<Session> {
        // Logged once, and only once per process, because the whole design
        // assumes a single session: two sessions would bind two bridges, run
        // two DHTs, and split the piece queue between them. Seeing this line
        // twice means that assumption broke.
        tracing::info!(
            "start_session: creating the torrent session (dir={})",
            download_dir.display()
        );

        let engine = TorrentEngine::start(EngineConfig::new(download_dir))
            .await
            .context("failed to start the torrent session")?;

        // Port 0 asks the OS for a free port; the bridge reports the real one
        // back. A fixed port would collide with a second instance of the app.
        let bridge = HttpBridge::start(api_for(engine.session()), 0)
            .await
            .context("failed to start the stream bridge")?;

        Ok(Session { engine, bridge })
    }

    /// The session, starting it if this is the first call.
    async fn session(&self) -> Result<&Session> {
        let download_dir = self.resolved_download_dir();
        self.inner
            .session
            .get_or_try_init(|| Self::start_session(download_dir))
            .await
    }

    /// Add a `.torrent` file and wait for the files it contains.
    pub async fn add_torrent(&self, path: &Path) -> Result<TorrentHandle> {
        let session = self.session().await?;

        let id = session
            .engine
            .add_torrent_file(path)
            .await
            .with_context(|| format!("failed to add {}", path.display()))?;

        self.cancel_pending_removal(id);
        let files = wait_for_files(&session.engine, id).await?;
        Ok(TorrentHandle { id, files })
    }

    /// Clear any deferred removal left on a torrent id.
    ///
    /// librqbit returns the SAME id for a torrent it already manages, so
    /// re-adding can resurrect an id that still carries a removal asked for on
    /// a previous visit. Left set, the next player exit would delete the
    /// torrent the reader is now watching.
    fn cancel_pending_removal(&self, id: usize) {
        self.inner
            .holds
            .lock()
            .expect("holds mutex poisoned")
            .cancel_pending(id);
    }

    /// Add a magnet URI and wait for the files it resolves to.
    ///
    /// The magnet counterpart of [`Self::add_torrent`], used when the app
    /// finds the release itself: a search result carries a magnet, not a
    /// `.torrent` path. Metadata still has to arrive from peers, so this can
    /// legitimately take longer than a local file.
    pub async fn add_magnet(&self, magnet_uri: &str) -> Result<TorrentHandle> {
        let session = self.session().await?;

        let id = session
            .engine
            .add_magnet(magnet_uri)
            .await
            .context("failed to add magnet")?;

        self.cancel_pending_removal(id);
        let files = wait_for_files(&session.engine, id).await?;
        Ok(TorrentHandle { id, files })
    }

    /// Drop a torrent from the session, deleting the pieces it cached.
    ///
    /// Called when the reader leaves the watch page: without this the torrent
    /// keeps downloading and seeding for the life of the process, because the
    /// session is created once and never torn down.
    ///
    /// A removal before anything was added is a no-op rather than an error. The
    /// page can unmount without the reader ever having played, and the session
    /// must NOT be started just to remove nothing from it -- so this reads the
    /// cell rather than going through [`Self::session`], which would start one.
    pub async fn remove_torrent(&self, id: usize) -> Result<()> {
        // Defer while an external player reads it, or mpv's next range request
        // fails. `release_torrent` finishes the job once the last holder exits.
        let unheld = self
            .inner
            .holds
            .lock()
            .expect("holds mutex poisoned")
            .request_removal(id);

        if unheld {
            self.remove_torrent_now(id).await?;
        }

        Ok(())
    }

    /// Remove a torrent immediately, ignoring any hold.
    ///
    /// The unconditional half of [`Self::remove_torrent`], used once a deferred
    /// removal is finally due. Reads the session cell rather than going through
    /// [`Self::session`], which would START a session in order to remove
    /// nothing from it.
    async fn remove_torrent_now(&self, id: usize) -> Result<()> {
        let Some(session) = self.inner.session.get() else {
            return Ok(());
        };

        session.engine.remove_torrent(id).await
    }

    /// Note that a player stopped reading a torrent.
    ///
    /// When this was the last holder and the watch page had already asked for
    /// removal, the removal happens here -- which is what keeps mpv playing
    /// after the page is left, then cleans up when it closes.
    pub async fn release_torrent(&self, id: usize) -> Result<()> {
        let due = self
            .inner
            .holds
            .lock()
            .expect("holds mutex poisoned")
            .release(id);

        if due {
            self.remove_torrent_now(id).await?;
        }

        Ok(())
    }

    /// Take a hold on a torrent without launching anything.
    ///
    /// The external-player path takes its hold inside [`Self::launch`]. An
    /// embedded player (the Qt app renders libmpv in its own window) never
    /// goes through that path, so it must hold the torrent itself: without a
    /// hold, leaving the page would remove the torrent out from under a player
    /// that is still reading it. Pair every call with
    /// [`Self::release_torrent`].
    pub fn hold_torrent(&self, id: usize) {
        self.inner
            .holds
            .lock()
            .expect("holds mutex poisoned")
            .hold(id);
    }

    /// The loopback URL that streams one file of one torrent.
    ///
    /// Waits until the torrent is no longer `Initializing` before returning.
    /// librqbit 500s its stream endpoint during the initial (re)hash, and mpv
    /// does not retry a 500, so returning early yields a player stuck at 00:00.
    /// A torrent that errors, or that never leaves `initializing` within the
    /// cap, returns its URL anyway: the caller's own error handling and the
    /// buffering UI are a better place to surface that than a hang here.
    pub async fn stream_url(&self, torrent_id: usize, file_idx: usize) -> Result<String> {
        let session = self.session().await?;

        let deadline = tokio::time::Instant::now() + LIVE_TIMEOUT;
        loop {
            match session.engine.progress(torrent_id) {
                // Live (or finished/paused after being live): safe to serve.
                Some(p) if p.state != "initializing" => break,
                // Errored: hand over the URL so the player reports it.
                Some(p) if p.state == "error" => break,
                // Unknown id: nothing to wait for.
                None => break,
                _ => {}
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(LIVE_POLL).await;
        }

        Ok(session.bridge.url_for(torrent_id, file_idx))
    }

    /// Restrict the download to `file_idx`, warm both ends of it, and return
    /// the URL a player should open.
    ///
    /// This is the play path the Qt app uses, and it exists because three
    /// separate things have to happen in a specific ORDER for playback to
    /// start promptly:
    ///
    /// 1. **Select the file** -- and WAIT for the selection to be accepted.
    ///    librqbit refuses `update_only_files` while it is initializing, and a
    ///    magnet is always initializing right after it is added. The old code
    ///    fired the selection and the URL request as independent tasks, so the
    ///    selection silently failed and librqbit kept downloading the torrent
    ///    in ITS order: a batch release filled its first file while the
    ///    episode the reader was waiting on sat at zero, and the progress bar
    ///    climbed past 20% with nothing to play.
    /// 2. **Warm the head** -- one small ranged read, so the first pieces are
    ///    the highest-priority ones.
    /// 3. **Warm the tail** -- one small ranged read of the END of the file.
    ///    A player resolves duration and the seek index (Matroska `Cues`, or a
    ///    trailing MP4 `moov`) by reading the end of the stream. librqbit
    ///    prioritises pieces around OPEN readers, so asking for the tail up
    ///    front is what stops the player from blocking on a range the torrent
    ///    has not downloaded yet -- the actual cause of "20% downloaded and
    ///    still not playing".
    ///
    /// Every step is best-effort: a failure is logged and the URL is still
    /// returned, because the buffering UI can show a slow swarm but cannot
    /// recover from a hung command.
    pub async fn prepare_stream(&self, torrent_id: usize, file_idx: usize) -> Result<String> {
        // Stage timing, logged at `info`: this function is the whole reason a
        // play can take a long time before mpv is even asked to open a URL, so
        // its stages must be attributable. The sample below is what a report
        // should quote:
        //   prepare_stream: torrent=1 file=2 starting
        //   set_only_files accepted in 1200ms (state became live)
        //   warm head ok in 35ms
        //   warm-up finished in 2100ms total
        //   prepare_stream: torrent=1 file=2 ready in 3400ms -> http://...
        let started = std::time::Instant::now();
        tracing::info!("prepare_stream: torrent={torrent_id} file={file_idx} starting");

        let session = match self.session().await {
            Ok(session) => session,
            Err(err) => {
                tracing::error!(
                    "prepare_stream: could not start the session after {}ms: {err:#}",
                    started.elapsed().as_millis()
                );
                return Err(err);
            }
        };

        // 1. File selection. A torrent that never accepts it is a warning, not
        //    an error: playback still works, just with the engine's own order.
        //    The engine logs its own retry transitions; this line records how
        //    long the caller waited and whether the wait succeeded.
        let select_at = std::time::Instant::now();
        match session.engine.set_only_files(torrent_id, &[file_idx]).await {
            Ok(()) => tracing::info!(
                "prepare_stream: set_only_files({file_idx}) ACCEPTED after {}ms",
                select_at.elapsed().as_millis()
            ),
            Err(err) => tracing::warn!(
                "prepare_stream: set_only_files({file_idx}) NOT accepted after {}ms, \
                 engine keeps its own file order: {err:#}",
                select_at.elapsed().as_millis()
            ),
        }

        // 2. Warm-up, DETACHED. Nothing below this point may delay the URL.
        //
        // Measured: awaiting a cold head cost the full 20s timeout and delayed
        // playback by exactly that much --
        //   warm head FAILED after 20000ms (... stopped after 1048576 bytes)
        //   prepare_stream: ready in 20019ms
        // A cold torrent has no pieces to serve, so the warm-up could only
        // wait. mpv is perfectly capable of issuing its own first request; the
        // warm-up is a PRIORITY HINT, and a hint must never be a gate.
        let url = session.bridge.url_for(torrent_id, file_idx);

        // The file length comes from the engine's metadata. It used to be
        // probed over HTTP (`Range: bytes=0-0`), but that request needs piece
        // 0, so on a cold torrent it blocks exactly like the head did. The
        // metadata is already resolved by the time we get here.
        let length = session
            .engine
            .torrent_files(torrent_id)
            .into_iter()
            .find(|(idx, _, _)| *idx == file_idx)
            .map(|(_, _, len)| len);

        // Only ONE warm-up per (torrent, file). Repeated `prepare_stream`
        // calls for the same episode otherwise open a new pair of readers
        // each time, and librqbit splits the piece queue EQUALLY between
        // open readers -- so the tail, the range mpv actually blocks on,
        // ends up with a fraction of the bandwidth it needs.
        let already_warming = {
            let mut warmups = self.inner.warmups.lock().expect("warmups mutex poisoned");
            !warmups.insert((torrent_id, file_idx))
        };

        if already_warming {
            tracing::info!(
                "prepare_stream: torrent={torrent_id} file={file_idx} ready in {}ms -> {url} \
                 (warm-up already running, not spawning another)",
                started.elapsed().as_millis()
            );
            return Ok(url);
        }

        let warm_url = url.clone();
        let state = self.clone();
        tokio::spawn(async move {
            warm_stream(&warm_url, length).await;
            // Release the guard once the warm-up has finished, so a later
            // replay of the same episode can warm again (the pieces may have
            // been evicted, or the torrent re-added with a fresh id).
            if let Ok(mut warmups) = state.inner.warmups.lock() {
                warmups.remove(&(torrent_id, file_idx));
            }
        });

        tracing::info!(
            "prepare_stream: torrent={torrent_id} file={file_idx} ready in {}ms -> {url} \
             (warm-up detached, len={length:?})",
            started.elapsed().as_millis()
        );

        Ok(url)
    }

    /// A download-progress snapshot for `torrent_id`, for the watch page.
    ///
    /// Reads the session cell rather than going through [`Self::session`],
    /// because that uses `get_or_try_init` and would START a torrent session
    /// merely to answer a progress poll. Polling happens before playback, on a
    /// page where nothing may have been added yet, so starting a session here
    /// would bind sockets and join the DHT for a question about nothing.
    ///
    /// `None` means either "no session" or "no such torrent"; both are simply
    /// "nothing to report yet" and are not errors.
    pub fn torrent_progress(&self, torrent_id: usize) -> Option<TorrentProgress> {
        self.inner.session.get()?.engine.progress(torrent_id)
    }

    /// Pause a torrent's transfer, keeping its partial data.
    ///
    /// Reads the session cell rather than going through [`Self::session`], for
    /// the same reason as [`Self::torrent_progress`]: pausing a torrent that
    /// was never added must not START a session merely to pause nothing.
    pub async fn pause_torrent(&self, id: usize) -> Result<()> {
        let Some(session) = self.inner.session.get() else {
            return Ok(());
        };

        session.engine.pause_torrent(id).await
    }

    /// Resume a paused torrent's transfer.
    ///
    /// The counterpart of [`Self::pause_torrent`], with the same cell-read
    /// rule so resuming nothing cannot start a session.
    pub async fn resume_torrent(&self, id: usize) -> Result<()> {
        let Some(session) = self.inner.session.get() else {
            return Ok(());
        };

        session.engine.resume_torrent(id).await
    }

    /// Restrict a torrent's download to a single file index.
    ///
    /// Called when a file is picked, so a season pack fetches only the episode
    /// being watched rather than its first file. Same cell-read rule as the
    /// pause/resume calls.
    pub async fn set_only_files(&self, id: usize, only: Vec<usize>) -> Result<()> {
        let Some(session) = self.inner.session.get() else {
            return Ok(());
        };

        session.engine.set_only_files(id, &only).await
    }

}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Poll until the torrent reports its files, or the deadline passes.
///
/// A local `.torrent` resolves on the first poll. The loop exists for the
/// case where it does not, so the command fails with a clear message instead
/// of returning an empty file list the UI cannot use.
/// Ask the bridge for the first and last chunk of a stream, then drop both.
///
/// librqbit prioritises pieces around each OPEN reader, so a ranged request
/// makes the pieces it names high priority. Doing this for the head AND the
/// tail is what lets a player start immediately: it needs the head to demux
/// and the tail to know the duration and seek index, and without the tail
/// request it can block on a range that only arrives once the rest of the
/// torrent is nearly done -- the "20% downloaded and still black" symptom.
///
/// The body is never read, and every failure is swallowed: this is a
/// performance hint, and a torrent with no peers yet must not fail playback
/// because of it.
async fn warm_stream(url: &str, length: Option<u64>) {
    // The three stages below are timed individually and logged at `info`:
    // they are the first thing that can block AFTER the file selection, and
    // without numbers here a slow start cannot be attributed to the head
    // request, the length probe, or the tail request.
    let started = std::time::Instant::now();

    let client = match reqwest::Client::builder().timeout(WARM_TIMEOUT).build() {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!("warm-up client unavailable, skipping warm-up: {err:#}");
            return;
        }
    };

    // Head. BOUNDED twice over: at WARM_HEAD_BYTES so it cannot become an
    // "everything to EOF" download, and at its OWN short timeout so a cold
    // torrent (nothing to serve yet) gives up quickly instead of waiting out
    // the long tail budget. This is the caller's remaining latency, even
    // though the whole warm-up is detached, because it competes for pieces
    // with mpv's first read.
    let head_at = std::time::Instant::now();
    let head_range = format!("bytes=0-{}", WARM_HEAD_BYTES.saturating_sub(1));
    let head_client = reqwest::Client::builder()
        .timeout(WARM_HEAD_TIMEOUT)
        .build()
        .unwrap_or(client.clone());
    match fetch_first_chunk(&head_client, url, &head_range).await {
        Ok(bytes) => tracing::info!(
            "warm head ok in {}ms, fetched {bytes} bytes (range={head_range})",
            head_at.elapsed().as_millis()
        ),
        Err(err) => tracing::warn!(
            "warm head FAILED after {}ms (range={head_range}, expected on a cold \
             torrent -- mpv will request it itself): {err:#}",
            head_at.elapsed().as_millis()
        ),
    }

    // Length comes from the engine's METADATA, not from an HTTP probe.
    //
    // The probe was `Range: bytes=0-0`, which needs piece 0 -- so on a cold
    // torrent it blocked exactly like the head did, and the whole warm-up sat
    // there until the client timeout. The caller already knows the length from
    // the resolved torrent info, so no request is needed.
    match length {
        Some(len) => tracing::info!("warm length from metadata: len={len} bytes"),
        None => tracing::warn!(
            "warm length unknown for file (index not found in metadata); \
             head only, no tail warm-up"
        ),
    }

    // Tail. This is the one that MATTERS -- mpv seeks to the end of the file
    // for the Matroska `Cues` index before it will play past the first few
    // seconds -- but it must NOT be awaited.
    //
    // Measured: awaiting a cold tail cost the full 20s timeout and delayed the
    // URL by that much, because mpv then did its own tail seek anyway. So the
    // fetch is spawned detached: it pulls the pieces in the background while
    // mpv is starting, and the URL is returned immediately. Whatever arrives
    // before mpv asks is free progress; whatever does not, mpv waits for on its
    // own request, exactly as it would have without us.
    match length {
        Some(len) if len > WARM_TAIL_BYTES => {
            let start = len - WARM_TAIL_BYTES;
            let range = format!("bytes={start}-");
            let url = url.to_string();
            tokio::spawn(async move {
                let tail_at = std::time::Instant::now();
                let result = match reqwest::Client::builder().timeout(WARM_TIMEOUT).build() {
                    Ok(client) => fetch_first_chunk(&client, &url, &range).await,
                    Err(err) => Err(anyhow!("warm-up client unavailable: {err:#}")),
                };
                match result {
                    Ok(0) => tracing::warn!(
                        "warm tail delivered 0 bytes after {}ms (range={range}) -- \
                         the seek index will not be ready",
                        tail_at.elapsed().as_millis()
                    ),
                    Ok(bytes) => tracing::info!(
                        "warm tail ok in {}ms, fetched {bytes} bytes (range={range})",
                        tail_at.elapsed().as_millis()
                    ),
                    Err(err) => tracing::warn!(
                        "warm tail FAILED after {}ms (range={range}): {err:#}",
                        tail_at.elapsed().as_millis()
                    ),
                }
            });
        }
        Some(len) => tracing::info!("warm tail skipped: file is only {len} bytes"),
        None => {}
    }

    tracing::info!(
        "warm-up finished in {}ms total ({url})",
        started.elapsed().as_millis()
    );
}



/// Issue one ranged request and READ the body, discarding it.
///
/// This used to await only `send()` (headers) and drop the response. That was
/// a real bug, proven by the player log:
///
///   [mpv/ffmpeg] stream level seek from 9469952 to 1260895420
///   (Buffering) AV: 00:00:07 / 00:23:20 (1%) Cache: 0.0s/1KB
///
/// mpv decodes a few seconds, then seeks to the very end of the file for the
/// Matroska `Cues` index and blocks. The warm-up was supposed to have fetched
/// that range first, but awaiting headers alone fetched NOTHING: librqbit
/// answers headers immediately, the response was dropped, and the bytes were
/// never requested from peers. mpv then asked for the same range and waited.
///
/// Draining the body is what actually pulls the pieces, and therefore what
/// makes the tail index present before the player needs it. The drain is
/// bounded by [`WARM_DRAIN_BYTES`] and by the client's timeout, so a torrent
/// with no peers still cannot hang playback.
async fn fetch_first_chunk(
    client: &reqwest::Client,
    url: &str,
    range: &str,
) -> Result<u64> {
    let mut response = client
        .get(url)
        .header(reqwest::header::RANGE, range)
        .send()
        .await
        .with_context(|| format!("warm-up request {range} failed"))?;

    // A non-success status means the range was not served; draining would
    // just read an error page.
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("warm-up request {range} returned HTTP {status}"));
    }

    // Read and discard, stopping at the cap. `chunk()` yields as bytes arrive,
    // so this returns as soon as the requested span is in hand rather than
    // waiting for the whole (possibly huge) remainder of the file.
    let mut drained: u64 = 0;
    loop {
        if drained >= WARM_DRAIN_BYTES {
            break;
        }
        match response.chunk().await {
            Ok(Some(chunk)) => {
                drained += chunk.len() as u64;
            }
            // Clean end of body: the requested range was fully delivered.
            Ok(None) => break,
            Err(err) => {
                // Partial data is still useful: the pieces already delivered
                // stay on disk and keep their priority. Report what arrived.
                return Err(anyhow!(
                    "warm-up {range} stopped after {drained} bytes: {err:#}"
                ));
            }
        }
    }

    Ok(drained)
}

async fn wait_for_files(engine: &TorrentEngine, id: usize) -> Result<Vec<TorrentFile>> {
    let deadline = tokio::time::Instant::now() + METADATA_TIMEOUT;

    loop {
        let files = engine.torrent_files(id);
        if !files.is_empty() {
            return Ok(files
                .into_iter()
                .map(|(idx, name, length_bytes)| TorrentFile {
                    idx,
                    name,
                    length_bytes,
                })
                .collect());
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(anyhow!(
                "torrent {id} produced no files within {}s",
                METADATA_TIMEOUT.as_secs()
            ));
        }

        tokio::time::sleep(METADATA_POLL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::DEFAULT_PLAYER;

    #[test]
    fn new_state_uses_the_default_player() {
        assert_eq!(PlayerState::new().player(), DEFAULT_PLAYER);
    }

    #[test]
    fn set_player_records_a_choice() {
        let state = PlayerState::new();
        state.set_player("vlc");
        assert_eq!(state.player(), "vlc");
    }

    #[test]
    fn set_player_with_a_blank_name_restores_the_default() {
        let state = PlayerState::new();
        state.set_player("vlc");
        state.set_player("   ");
        assert_eq!(state.player(), DEFAULT_PLAYER);
    }

    /// Removal must not start a session in order to remove nothing from it.
    ///
    /// The tempting implementation routes through `Self::session`, which uses
    /// `get_or_try_init` and would therefore bind sockets, join the DHT and
    /// start the bridge -- on a page the reader may never have played. This
    /// pins that the guard reads the cell instead.
    #[tokio::test]
    async fn remove_torrent_before_any_session_is_a_noop() {
        let state = PlayerState::new();

        state
            .remove_torrent(7)
            .await
            .expect("removing before any add should succeed");

        assert!(
            state.inner.session.get().is_none(),
            "removal must not start a session"
        );
    }

    /// Polling progress must not start a session to answer about nothing.
    ///
    /// The watch page polls before playback, when nothing may have been added
    /// yet. Routing through `Self::session` would bind sockets and join the
    /// DHT for that question; this pins that the guard reads the cell instead.
    #[test]
    fn torrent_progress_before_any_session_is_none_without_starting_one() {
        let state = PlayerState::new();

        assert!(state.torrent_progress(7).is_none());
        assert!(
            state.inner.session.get().is_none(),
            "polling progress must not start a session"
        );
    }

    /// A hold alone must not report a removal as due: nothing asked for one.
    #[test]
    fn release_without_a_request_does_not_remove() {
        let mut holds = Holds::default();
        holds.hold(5);

        assert!(!holds.release(5), "no removal was requested");
    }

    /// The whole point of the type: removal is deferred while a player reads.
    #[test]
    fn removal_is_deferred_while_a_player_holds() {
        let mut holds = Holds::default();
        holds.hold(5);

        assert!(
            !holds.request_removal(5),
            "a held torrent must not be removed yet"
        );
        assert!(
            holds.release(5),
            "the last holder leaving should release the deferred removal"
        );
    }

    /// With no holder at all, a request removes immediately -- the ordinary
    /// case of leaving the watch page without ever opening a player.
    #[test]
    fn removal_is_immediate_without_a_hold() {
        let mut holds = Holds::default();

        assert!(holds.request_removal(5));
    }

    /// The deferred removal must wait for EVERY player, not the first to exit.
    #[test]
    fn removal_waits_for_the_last_holder() {
        let mut holds = Holds::default();
        holds.hold(5);
        holds.hold(5);

        assert!(!holds.request_removal(5));
        assert!(!holds.release(5), "one player is still reading it");
        assert!(holds.release(5), "the last player left");
    }

    /// Re-adding a torrent clears a stale request.
    ///
    /// librqbit returns the same id for a torrent it already manages, so a
    /// replay can resurrect an id that still carries a request from an earlier
    /// visit. Without the cancel, the next player exit would delete the torrent
    /// the reader is watching.
    #[test]
    fn cancel_pending_drops_a_deferred_removal() {
        let mut holds = Holds::default();
        holds.hold(5);
        holds.request_removal(5);

        holds.cancel_pending(5);

        assert!(
            !holds.release(5),
            "the stale request must not fire on release"
        );
    }

    /// A release with no matching hold is ignored rather than treated as a
    /// reason to remove, which would delete a torrent nobody had finished with.
    #[test]
    fn an_unmatched_release_is_ignored() {
        let mut holds = Holds::default();

        assert!(!holds.release(5));
    }

    #[test]
    fn torrent_file_serialises_to_camel_case() {
        let file = TorrentFile {
            idx: 3,
            name: "episode.mkv".into(),
            length_bytes: 1_400_000_000,
        };
        let json = serde_json::to_value(&file).expect("serialize TorrentFile");
        assert!(json.get("lengthBytes").is_some(), "expected camelCase key");
        assert!(json.get("length_bytes").is_none());
    }

    #[test]
    fn torrent_handle_serialises_its_files() {
        let handle = TorrentHandle {
            id: 7,
            files: vec![TorrentFile {
                idx: 0,
                name: "a.mkv".into(),
                length_bytes: 10,
            }],
        };
        let json = serde_json::to_value(&handle).expect("serialize TorrentHandle");
        assert_eq!(json["id"], 7);
        assert_eq!(json["files"][0]["idx"], 0);
        assert_eq!(json["files"][0]["name"], "a.mkv");
    }
}
