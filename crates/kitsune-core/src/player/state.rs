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
use tokio::sync::RwLock;

use crate::settings::SettingsStore;
use crate::torrent::{api_for, EngineConfig, HttpBridge, RamBufferConfig, TorrentEngine, TorrentProgress};

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

/// Cap on ESTABLISHING a warm-up connection.
///
/// The tail request deliberately carries NO whole-request timeout. The first
/// implementation used reqwest's total `timeout(20s)` there, and on a swarm
/// that delivered the tail at ~133 KB/s the clock fired mid-body at 20.0s --
/// "stopped after 2669241 bytes: error decoding response body" -- cutting
/// off the last ~1.5 MiB, which is exactly where the Matroska `Cues` live.
/// mpv's own tail read then blocked on the missing pieces and the player sat
/// on a black frame. Bytes WERE arriving the whole time; only the clock was
/// wrong. The drain is bounded by [`WARM_DRAIN_BYTES`] and the per-chunk
/// [`WARM_IDLE_TIMEOUT`] instead, so the only clock left on the request
/// itself is this connect cap: a dead bridge must fail fast, and on loopback
/// the connect lands in microseconds.
const WARM_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Longest gap between two body bytes before a warm-up drain gives up.
///
/// Progress resets it: every delivered chunk restarts the clock, so a
/// slow-but-moving swarm keeps draining for as long as it takes (the
/// [`WARM_DRAIN_BYTES`] cap bounds the total), while a genuinely stalled one
/// is abandoned within seconds instead of hanging the detached task forever.
const WARM_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// How many times the tail warm-up tries before reporting failure.
///
/// Pieces an interrupted attempt already delivered stay in the RAM pool and
/// keep their priority, so a retry re-serves those instantly and usually
/// only has to finish the last piece or two.
const WARM_TAIL_ATTEMPTS: u32 = 2;

/// Pause between tail warm-up attempts.
///
/// Long enough for an in-flight piece to land, short enough that the retry
/// still finishes well before mpv's own tail read gets there.
const WARM_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

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

/// A stream that is ready to hand to a player.
///
/// Carries its own torrent id because the play path can RE-ADD a torrent: when
/// the exact disk-space check fails after the file list resolves, the disk
/// copy is dropped and added again in RAM, which gives it a NEW librqbit id.
/// The caller must use `torrent_id` -- not the id it passed in -- for stats,
/// holds and removal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedStream {
    /// The torrent that is actually streaming, after any re-add.
    pub torrent_id: usize,
    /// Loopback URL to open.
    pub url: String,
    /// Set when space forced RAM against the reader's choice.
    pub ram_fallback: Option<RamFallbackReason>,
}

/// A live torrent session and its stream bridge.
struct Session {
    engine: TorrentEngine,
    bridge: HttpBridge,
}

impl Session {
    /// Stop the bridge, then the engine.
    ///
    /// The ORDER matters: the bridge serves a read-only `Api` over the same
    /// librqbit session, so stopping the engine first leaves a task serving a
    /// dead session. Aborting the bridge first is also what releases the
    /// loopback port for the session that replaces this one.
    async fn shutdown(&self) {
        self.bridge.shutdown();
        self.engine.stop().await;
    }
}

/// What happened when the reader changed the streaming mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamingRestart {
    /// The session was torn down; the next play uses the new mode.
    Applied,
    /// Something is playing, so the session was left alone.
    Busy,
    /// No session had started; the next one will simply read the new setting.
    NothingToDo,
}

/// Where downloaded pieces are written.
///
/// Overridable for tests and for a user who wants downloads on another disk.
/// The default lives under the OS data directory, not in temp: a partial
/// download that vanishes on reboot cannot be resumed, which is exactly the
/// bug this path exists to avoid.
fn download_dir() -> PathBuf {
    std::env::var_os("KITSUNE_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("kitsune")
                .join("downloads")
        })
}

/// Where permanently downloaded episodes are stored.
///
/// Uses the OS videos directory (e.g. `~/Videos` on Linux) with a `Kitsune`
/// subfolder, falling back to the home directory if no videos dir exists.
fn library_dir() -> PathBuf {
    dirs::video_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("Kitsune")
}

/// Convert a settings bandwidth cap (KiB/s) into the bytes/s librqbit wants.
///
/// Zero, absent, and unrepresentable values all read as "unlimited". Pure, so
/// the conversion is asserted rather than trusted -- a wrong unit here would
/// cap a reader's transfers at 1/1024th of what they asked for and look like a
/// broken network.
fn kib_per_sec_to_bytes(kib: Option<u32>) -> Option<u32> {
    let kib = kib?;
    if kib == 0 {
        return None;
    }
    // Saturating rather than wrapping: a typo must not turn into a near-zero
    // cap, and 4 GiB/s is far past any real link.
    Some(kib.saturating_mul(1024))
}

/// Which torrents an external player is reading, and which removals wait on one.
///
/// The watch page removes its torrent when it is left. That is wrong while an
/// external player streams from the same torrent: mpv reads through the bridge
/// independently, so removing the torrent makes its next range request fail. A
/// launch therefore takes a hold, and removal is deferred until the last holder
/// releases.
///
/// Why a play ended up in RAM without the reader asking for it.
///
/// Only produced in classic (disk) mode, when the download volume cannot
/// spare the room. The watch modal turns this into a one-line note so the
/// fallback is visible rather than mysterious.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RamFallbackReason {
    /// The download volume cannot hold the episode plus the reserve.
    NoDiskSpace,
}

/// Where a play will be streamed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingDecision {
    /// Classic mode: pieces go to disk and the bridge serves them.
    Disk,
    /// In-memory storage. `fallback: None` means the reader chose RAM;
    /// `Some(_)` means space forced it and the modal should say so.
    Ram { fallback: Option<RamFallbackReason> },
}

impl StreamingDecision {
    /// Whether this play uses the RAM pool.
    pub fn is_ram(&self) -> bool {
        matches!(self, Self::Ram { .. })
    }

    /// Whether space forced RAM against the reader's choice.
    pub fn forced_fallback(&self) -> Option<RamFallbackReason> {
        match self {
            Self::Ram { fallback } => *fallback,
            Self::Disk => None,
        }
    }
}

/// How a playback torrent was named, kept so a forced RAM fallback can re-add
/// it after the exact size is known. Only stored for torrents the heuristic
/// admitted on disk (the only ones that can need a re-add).
#[derive(Debug, Clone)]
enum PlaybackSource {
    Magnet(String),
    TorrentFile(PathBuf),
}

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
    /// `None` until the first play, and `None` again after a settings change
    /// tears it down so the new mode applies. An `RwLock` rather than a
    /// `OnceCell` because the streaming mode is fixed when the session starts:
    /// changing it means stopping the engine and bridge and starting fresh.
    session: RwLock<Option<Arc<Session>>>,
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
    /// Playback torrents that may still need the exact disk-space check, and
    /// why they might fall back: torrent id -> (source, reason-if-known).
    ///
    /// Populated when a play is admitted on disk by the CHEAP heuristic, so
    /// `prepare_stream` can re-add it in RAM if the resolved file turns out
    /// not to fit. Entries are removed once checked, on removal, and on the
    /// RAM-at-add path (which needs no re-add).
    pending_space_check: Mutex<HashMap<usize, PlaybackSource>>,
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
    /// Torrent ids a player is currently streaming.
    ///
    /// Distinct from `holds`, which tracks players the app LAUNCHED: a stream
    /// prepared but not yet opened by the player has no hold, and yet tearing
    /// the session down under it would break the URL the player is about to
    /// request. Inserted in `prepare_stream`, removed on removal.
    streams: Mutex<HashSet<usize>>,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                session: RwLock::new(None),
                player: Mutex::new(resolve_player(None)),
                holds: Mutex::new(Holds::default()),
                pending_space_check: Mutex::new(HashMap::new()),
                settings: None,
                warmups: Mutex::new(HashSet::new()),
                streams: Mutex::new(HashSet::new()),
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
                session: RwLock::new(None),
                player: Mutex::new(player),
                holds: Mutex::new(Holds::default()),
                pending_space_check: Mutex::new(HashMap::new()),
                settings: Some(store),
                warmups: Mutex::new(HashSet::new()),
                streams: Mutex::new(HashSet::new()),
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
    /// Public because launching an external player is a frontend concern: the
    /// Qt build plays in-process through libmpv, but a future one may want to
    /// hand the stream URL to mpv and needs these flags to do it.
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
    /// otherwise the persistent app-data directory. Read on each session start,
    /// so a change in settings applies to the next torrent.
    pub fn resolved_download_dir(&self) -> PathBuf {
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

    /// Where permanently downloaded episodes are stored.
    ///
    /// The reader's explicit choice wins; otherwise `~/Videos/Kitsune`,
    /// otherwise the OS videos directory with a `kitsune` subfolder.
    /// Distinct from `resolved_download_dir()`, which is the torrent cache.
    pub fn resolved_library_dir(&self) -> PathBuf {
        if let Some(dir) = self
            .inner
            .settings
            .as_ref()
            .and_then(|store| store.read().library_dir)
            .filter(|dir| !dir.trim().is_empty())
        {
            return PathBuf::from(dir);
        }
        library_dir()
    }

    /// Per-torrent partial-data directory for `info_hash`.
    pub fn cache_dir_for(&self, info_hash: &str) -> Result<PathBuf> {
        crate::downloads::download_cache_dir(&self.resolved_download_dir(), info_hash)
    }

    /// Start the torrent engine and stream bridge, once.
    async fn start_session(
        download_dir: PathBuf,
        download_bps: Option<u32>,
        upload_bps: Option<u32>,
        ram: Option<RamBufferConfig>,
    ) -> Result<Session> {
        // Logged once, and only once per process, because the whole design
        // assumes a single session: two sessions would bind two bridges, run
        // two DHTs, and split the piece queue between them. Seeing this line
        // twice means that assumption broke.
        tracing::info!(
            "start_session: creating the torrent session (dir={})",
            download_dir.display()
        );

        // The reader's bandwidth caps, read once at session start. KiB/s in
        // the settings, bytes/s in librqbit, so the conversion lives in
        // `kib_per_sec_to_bytes` and nowhere else.
        let mut config = EngineConfig::new(download_dir).with_limits(download_bps, upload_bps);

        // RAM streaming: when on, playback torrents hold their pieces in a
        // bounded in-memory pool and write nothing to the disk. Permanent
        // downloads ignore this and always go to disk.
        if let Some(buffer) = &ram {
            tracing::info!(
                buffer_mib = buffer.buffer_bytes / 1024 / 1024,
                "start_session: RAM streaming enabled"
            );
            config = config.with_ram(buffer.clone());
        }

        tracing::info!(
            download_bps = ?config.download_bps,
            upload_bps = ?config.upload_bps,
            "start_session: bandwidth limits"
        );

        let engine = TorrentEngine::start(config)
            .await
            .context("failed to start the torrent session")?;

        // Port 0 asks the OS for a free port; the bridge reports the real one
        // back. A fixed port would collide with a second instance of the app.
        let bridge = HttpBridge::start(api_for(engine.session()), 0)
            .await
            .context("failed to start the stream bridge")?;

        Ok(Session { engine, bridge })
    }

    /// The session if one has been started, WITHOUT starting it.
    ///
    /// Every poll and removal path uses this: asking about a torrent must not
    /// bind sockets or join the DHT merely to answer "nothing yet".
    fn session_snapshot(&self) -> Option<Arc<Session>> {
        self.inner
            .session
            .try_read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// The session, starting it if this is the first call.
    ///
    /// Read-then-write rather than a plain lock: the common case (a session
    /// already running) takes the read half and returns, while only the first
    /// play -- and the first play after a restart -- pays for the write half
    /// and the engine start it guards.
    async fn session(&self) -> Result<Arc<Session>> {
        if let Some(session) = self.inner.session.read().await.clone() {
            return Ok(session);
        }

        let download_dir = self.resolved_download_dir();
        let (download_bps, upload_bps) = self.resolved_limits();
        let ram = self.resolved_ram_config();

        let mut guard = self.inner.session.write().await;
        // Another caller may have started it while this one awaited the write
        // lock. Re-check, or two engines would bind two bridges.
        if let Some(session) = guard.as_ref() {
            return Ok(session.clone());
        }

        let session = Arc::new(
            Self::start_session(download_dir, download_bps, upload_bps, ram).await?,
        );
        *guard = Some(session.clone());
        Ok(session)
    }

    /// The RAM streaming configuration from settings.
    ///
    /// Without a settings store the built-in defaults decide, exactly like
    /// every other preference: RAM streaming is on with the prototype's
    /// 256 MiB buffer. Read once at session start -- the pool size is fixed
    /// for the life of the session, like the bandwidth caps.
    pub fn resolved_ram_config(&self) -> Option<RamBufferConfig> {
        let settings = match self.inner.settings.as_ref() {
            Some(store) => store.read(),
            None => crate::settings::Settings::default(),
        };
        if !settings.ram_streaming {
            return None;
        }
        Some(RamBufferConfig::from_mib(settings.ram_buffer_mib))
    }

    /// Whether a play should stream from RAM, from disk, or fall back to RAM
    /// because the download volume is nearly full.
    ///
    /// `needed_bytes` is the episode's length when the caller knows it. With
    /// `None` -- before metadata has resolved -- the check is a cheap
    /// heuristic against the RAM buffer plus the reserve, which is the
    /// smallest amount of disk a stream could plausibly need. That is
    /// deliberately conservative in the safe direction: it may pick RAM for a
    /// small episode on a tight volume, never disk for one that cannot fit.
    ///
    /// `available_space` failing means "cannot tell", and that degrades to the
    /// reader's own choice rather than silently changing it.
    pub fn decide_playback_mode(&self, needed_bytes: Option<u64>) -> StreamingDecision {
        let settings = match self.inner.settings.as_ref() {
            Some(store) => store.read(),
            None => crate::settings::Settings::default(),
        };

        // The reader asked for RAM: nothing to second-guess.
        if settings.ram_streaming {
            return StreamingDecision::Ram { fallback: None };
        }

        let dir = self.resolved_download_dir();
        let reserve = settings.reserve_bytes();
        let fits = match needed_bytes {
            Some(needed) => crate::torrent::fits_on_disk(&dir, needed, reserve),
            // Unknown size: assume the episode needs at least one buffer's
            // worth on disk before it is worth attempting a disk stream.
            None => crate::torrent::fits_on_disk(&dir, settings.ram_buffer_bytes(), reserve),
        };

        if fits {
            StreamingDecision::Disk
        } else {
            tracing::warn!(
                "decide_playback_mode: {} cannot spare {} bytes + {} reserve; streaming from RAM",
                dir.display(),
                needed_bytes.unwrap_or_default(),
                reserve
            );
            StreamingDecision::Ram {
                fallback: Some(RamFallbackReason::NoDiskSpace),
            }
        }
    }

    /// A snapshot of the shared RAM streaming buffer, when enabled.
    ///
    /// The watch page can show "buffer 84 MiB / 256 MiB" from this. `None`
    /// in disk mode or before a session exists -- reading it must not START
    /// a session, so it reads the cell like the other progress readers.
    pub fn ram_stats(&self) -> Option<crate::torrent::RamPoolStats> {
        self.session_snapshot()?.engine.ram_stats()
    }

    /// Whether torrent `id` is currently backed by the RAM streaming pool.
    ///
    /// The scrub-preview player must not touch RAM streams: every preview
    /// seek makes the engine download a whole 32 MiB window at the hovered
    /// position, stealing bandwidth from the real playback and churning the
    /// buffer with data nobody watches. Disk downloads can afford it.
    pub fn is_ram_streaming(&self, id: usize) -> bool {
        self.session_snapshot()
            .is_some_and(|s| s.engine.is_ram_backed(id))
    }

    /// The bandwidth caps from settings, converted to bytes per second.
    ///
    /// KiB/s in the settings file, bytes/s in librqbit. A zero, absent, or
    /// unparseable value means unlimited -- the reader clearing the field must
    /// not produce a dead transfer, and `NonZeroU32` could not represent it
    /// anyway.
    pub fn resolved_limits(&self) -> (Option<u32>, Option<u32>) {
        let Some(store) = self.inner.settings.as_ref() else {
            return (None, None);
        };
        let settings = store.read();
        (
            kib_per_sec_to_bytes(settings.download_limit_kbps),
            kib_per_sec_to_bytes(settings.upload_limit_kbps),
        )
    }

    /// How many downloads may transfer at once.
    ///
    /// Falls back to 1, which is the behaviour the downloads page needs to stop
    /// eleven torrents competing for the same pipe.
    pub fn max_concurrent_downloads(&self) -> u32 {
        self.inner
            .settings
            .as_ref()
            .map(|store| store.read().max_concurrent_downloads)
            .filter(|limit| *limit > 0)
            .unwrap_or(1)
    }

    /// Add a `.torrent` file and wait for the files it contains.
    pub async fn add_torrent(&self, path: &Path) -> Result<TorrentHandle> {
        let session = self.session().await?;

        // Cheap disk check before the file list exists. A shortfall here is
        // the ONLY reason to force RAM: the session's own mode already
        // handles the reader's choice.
        let decision = self.decide_playback_mode(None);
        let force_ram = decision.forced_fallback().is_some();

        let id = session
            .engine
            .add_torrent_file_with_force(path, force_ram)
            .await
            .with_context(|| format!("failed to add {}", path.display()))?;

        self.cancel_pending_removal(id);
        // Admitted on disk by the heuristic: keep the source so the exact
        // check in `prepare_stream` can re-add it if the episode is too big.
        if decision == StreamingDecision::Disk {
            self.inner
                .pending_space_check
                .lock()
                .expect("pending space check mutex poisoned")
                .insert(id, PlaybackSource::TorrentFile(path.to_path_buf()));
        }

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

        // See `add_torrent`: the heuristic only ever forces RAM on a
        // disk-mode session that cannot spare the space.
        let decision = self.decide_playback_mode(None);
        let force_ram = decision.forced_fallback().is_some();

        let id = session
            .engine
            .add_magnet_with_force(magnet_uri, force_ram)
            .await
            .context("failed to add magnet")?;

        self.cancel_pending_removal(id);
        if decision == StreamingDecision::Disk {
            self.inner
                .pending_space_check
                .lock()
                .expect("pending space check mutex poisoned")
                .insert(id, PlaybackSource::Magnet(magnet_uri.to_string()));
        }

        let files = wait_for_files(&session.engine, id).await?;
        Ok(TorrentHandle { id, files })
    }

    /// Add a release for PERMANENT download and restrict it to one file.
    ///
    /// The play path ([`Self::prepare_stream`]) warms the head and tail so a
    /// player can start reading immediately. A download wants the opposite:
    /// every piece of the file, in whatever order the engine prefers, and no
    /// warm-up traffic competing with the real transfer. So this selects the
    /// file and returns, leaving the engine to fill it in.
    ///
    /// Returns the torrent id, which the caller polls with
    /// [`Self::torrent_progress`] and finally hands to
    /// [`Self::finish_download`].
    ///
    /// The file selection is best-effort for the same reason as playback: a
    /// torrent that never leaves `Initializing` still downloads, just in the
    /// engine's own order, and refusing to start would be worse than a slower
    /// download.
    pub async fn start_download(
        &self,
        magnet_uri: Option<&str>,
        torrent_path: Option<&Path>,
        file_indices: &[usize],
    ) -> Result<usize> {
        self.start_download_in(magnet_uri, torrent_path, file_indices, None, false)
            .await
    }

    /// Start a permanent download with an explicit cache directory.
    ///
    /// `cache_dir` is the per-torrent output folder; without it the session
    /// root is used, which is fine for one-off callers and tests but not for
    /// downloads that must survive a settings change. `paused` is used by the
    /// startup restore: the torrent is reattached with all its files selected
    /// and left stopped until the reader asks to continue.
    pub async fn start_download_in(
        &self,
        magnet_uri: Option<&str>,
        torrent_path: Option<&Path>,
        file_indices: &[usize],
        cache_dir: Option<&Path>,
        paused: bool,
    ) -> Result<usize> {
        // Every requested index must exist, and a request with none is a
        // caller bug rather than "download everything".
        let mut wanted: Vec<usize> = file_indices.to_vec();
        wanted.sort_unstable();
        wanted.dedup();

        if wanted.is_empty() {
            return Err(anyhow!("a download needs at least one file"));
        }

        let options = match cache_dir {
            Some(dir) => {
                crate::torrent::ensure_download_dir(dir)?;
                crate::torrent::download_options(dir.display().to_string(), wanted.clone(), paused)
            }
            None => {
                let mut options = crate::torrent::playback_options(None);
                options.only_files = Some(wanted.clone());
                options.paused = paused;
                options
            }
        };

        let session = self.session().await?;
        let id = match (magnet_uri, torrent_path) {
            (Some(uri), _) => session
                .engine
                .add_magnet_with_options(uri, options)
                .await
                .context("failed to add magnet")?,
            (None, Some(path)) => session
                .engine
                .add_torrent_file_with_options(path, options)
                .await
                .with_context(|| format!("failed to add {}", path.display()))?,
            (None, None) => return Err(anyhow!("a download needs a magnet or a torrent file")),
        };

        // A torrent that is currently STREAMING FROM RAM must not be turned
        // into a download: librqbit returns the EXISTING handle for a
        // re-added torrent (its options are not reapplied), so the download
        // would run on the RAM storage, fill the streaming buffer, and
        // `finish_download` would then find no file on disk. Refuse clearly;
        // the torrent is left alone so playback continues.
        if session.engine.is_ram_backed(id) {
            return Err(anyhow!(
                "this release is currently streaming in RAM mode; stop playback \
                 (or turn RAM streaming off) before downloading it"
            ));
        }

        self.cancel_pending_removal(id);
        let files = match wait_for_files(&session.engine, id).await {
            Ok(files) => files,
            Err(err) => {
                let _ = session.engine.remove_torrent(id).await;
                return Err(err);
            }
        };

        for file_idx in &wanted {
            if !files.iter().any(|file| file.idx == *file_idx) {
                let _ = session.engine.remove_torrent(id).await;
                return Err(anyhow!(
                    "torrent {id} has no file at index {file_idx} (it has {} files)",
                    files.len()
                ));
            }
        }

        tracing::info!(
            "start_download: torrent {id} files {wanted:?} queued (paused={paused}, cache={:?})",
            cache_dir
        );
        Ok(id)
    }

    /// Reattach an existing download, paused, from cached metadata or a magnet.
    ///
    /// `only_files` is the union of every selected file in the group, so a
    /// season pack comes back as one torrent even though each episode is a
    /// separate record.
    pub async fn restore_download(
        &self,
        magnet_uri: Option<&str>,
        torrent_path: Option<&Path>,
        file_indices: &[usize],
        cache_dir: &Path,
    ) -> Result<usize> {
        self.start_download_in(
            magnet_uri,
            torrent_path,
            file_indices,
            Some(cache_dir),
            true,
        )
        .await
    }

    /// Move a finished download into the library and stop its torrent.
    ///
    /// Called once [`crate::torrent::TorrentEngine::file_complete`] reports the
    /// file is fully on disk. The copy is a rename when both paths are on the
    /// same filesystem and a copy+delete otherwise, so a library on a second
    /// drive works.
    ///
    /// `season`/`episode` are optional: without them the original torrent file
    /// name is kept under `{library}/{anime}/`, which is still browsable and is
    /// better than refusing to finish a download whose numbering is unknown.
    ///
    /// Returns the final path. The torrent is removed afterwards so the cache
    /// does not keep a second copy, but a removal failure is logged rather than
    /// propagated: the file IS in the library at that point, and reporting an
    /// error would make a successful download look failed.
    ///
    /// `drop_torrent` exists because ONE torrent can back SEVERAL downloads: a
    /// season pack is added once with every ticked file selected, then each file
    /// becomes its own record. Dropping the torrent after the first file
    /// finished would delete the transfer the remaining files are still reading
    /// from, and they would sit at their current byte count forever. The caller
    /// passes `false` while any sibling record still references this torrent.
    #[allow(clippy::too_many_arguments)]
    pub async fn finish_download(
        &self,
        torrent_id: usize,
        file_idx: usize,
        library_dir: &Path,
        anime_title: &str,
        season: Option<u32>,
        episode: Option<u32>,
        drop_torrent: bool,
    ) -> Result<PathBuf> {
        let session = self.session().await?;

        let source = session
            .engine
            .file_disk_path(torrent_id, file_idx)
            .ok_or_else(|| anyhow!("torrent {torrent_id} has no file at index {file_idx}"))?;

        if !source.exists() {
            return Err(anyhow!(
                "expected the download at {} but nothing is there",
                source.display()
            ));
        }

        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("episode.mkv");
        let ext = crate::downloads::extension_of(file_name);

        let target = crate::downloads::library_path(
            library_dir,
            anime_title,
            season,
            episode,
            file_name,
            &ext,
        );

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        move_file(&source, &target)
            .with_context(|| format!("failed to move {} into the library", source.display()))?;

        // The file is in the library now, so the cached copy is dead weight --
        // but ONLY once no sibling record is still pulling from this torrent.
        // Best effort: a failure here leaves a duplicate on disk, not a broken
        // download.
        if drop_torrent {
            if let Err(err) = session.engine.remove_torrent(torrent_id).await {
                tracing::warn!("finish_download: could not drop torrent {torrent_id}: {err:#}");
            }
        }

        tracing::info!(
            "finish_download: torrent {torrent_id} file {file_idx} -> {}",
            target.display()
        );
        Ok(target)
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
        self.forget_torrent(id);

        let Some(session) = self.session_snapshot() else {
            return Ok(());
        };

        session.engine.remove_torrent(id).await
    }

    /// Drop every per-torrent bookkeeping entry for `id`.
    ///
    /// Call this whenever a torrent goes away, or a re-added id leaves a stale
    /// space-check entry (and its stored magnet) behind for the process's
    /// lifetime. Also clears the warm-up guard, which is keyed by torrent id.
    fn forget_torrent(&self, id: usize) {
        if let Ok(mut pending) = self.inner.pending_space_check.lock() {
            pending.remove(&id);
        }
        if let Ok(mut warmups) = self.inner.warmups.lock() {
            warmups.retain(|(torrent, _)| *torrent != id);
        }
        if let Ok(mut streams) = self.inner.streams.lock() {
            streams.remove(&id);
        }
    }

    /// Whether anything is using the session right now.
    ///
    /// A restart must not happen under a running player: it would break the
    /// URL mpv is reading. Both sets are consulted because a stream can be
    /// prepared before the player is launched, and a player can be held after
    /// its torrent was removed from the stream set.
    pub fn has_active_playback(&self) -> bool {
        let holding = self
            .inner
            .holds
            .lock()
            .map(|holds| !holds.counts.is_empty())
            .unwrap_or(false);
        let streaming = self
            .inner
            .streams
            .lock()
            .map(|streams| !streams.is_empty())
            .unwrap_or(false);
        holding || streaming
    }

    /// Stop the current torrent session so the next play uses the streaming
    /// mode from settings.
    ///
    /// The mode is baked into a session when it starts (the pool, the add
    /// options, the bandwidth caps), so it cannot be changed live. When
    /// nothing is playing the session is dropped and the NEXT play creates a
    /// fresh one; when something is, this refuses rather than interrupting it.
    pub async fn apply_streaming_mode(&self) -> Result<StreamingRestart> {
        // Checked BEFORE the cell is emptied. Taking the session out first
        // leaves `None` visible to a concurrent `session()` call, which would
        // start a SECOND engine and bridge while this one is being stopped.
        if self.has_active_playback() {
            return Ok(StreamingRestart::Busy);
        }

        let Some(session) = self.inner.session.write().await.take() else {
            return Ok(StreamingRestart::NothingToDo);
        };

        tracing::info!("apply_streaming_mode: stopping the session so the new mode applies");
        session.shutdown().await;
        Ok(StreamingRestart::Applied)
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
    pub async fn prepare_stream(&self, torrent_id: usize, file_idx: usize) -> Result<PreparedStream> {
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

        // 1. The EXACT disk-space check, now that the file list exists.
        //
        //    `add_torrent`/`add_magnet` only ran a heuristic (the episode's
        //    size was unknown then). With the real length in hand this can
        //    decide properly, and a torrent admitted on disk that will not fit
        //    is dropped and re-added in RAM. Re-adding allocates a NEW id, so
        //    everything below must use `torrent_id` from here on.
        let mut torrent_id = torrent_id;
        let mut ram_fallback = None;

        let length = session
            .engine
            .torrent_files(torrent_id)
            .into_iter()
            .find(|(idx, _, _)| *idx == file_idx)
            .map(|(_, _, len)| len);

        let pending_source = self
            .inner
            .pending_space_check
            .lock()
            .expect("pending space check mutex poisoned")
            .remove(&torrent_id);

        if let Some(source) = pending_source {
            let decision = self.decide_playback_mode(length);
            if decision.forced_fallback().is_some() && !session.engine.is_ram_backed(torrent_id) {
                tracing::warn!(
                    "prepare_stream: torrent={torrent_id} file={file_idx} does not fit on \
                     disk ({} bytes); re-adding it in RAM",
                    length.unwrap_or_default()
                );

                // Drop the disk copy first: the RAM re-add must not reuse the
                // torrent librqbit already manages (it would keep the disk
                // options AND the old id).
                if let Err(err) = session.engine.remove_torrent(torrent_id).await {
                    tracing::warn!("prepare_stream: could not drop disk torrent {torrent_id}: {err:#}");
                }
                self.forget_torrent(torrent_id);

                let readded = match &source {
                    PlaybackSource::Magnet(uri) => {
                        session.engine.add_magnet_with_force(uri, true).await
                    }
                    PlaybackSource::TorrentFile(path) => {
                        session.engine.add_torrent_file_with_force(path, true).await
                    }
                }
                .with_context(|| "failed to re-add the torrent for RAM streaming")?;

                torrent_id = readded;
                ram_fallback = decision.forced_fallback();

                // The new id needs the same deferred-removal hygiene as a
                // normal add, and the file list has to resolve again.
                self.cancel_pending_removal(torrent_id);
                wait_for_files(&session.engine, torrent_id).await?;
            } else if decision == StreamingDecision::Disk {
                // Plenty of room after all: nothing to do, and no note.
            } else {
                // Already RAM-backed (the setting is on, or the heuristic
                // forced it): report the note if space was the reason.
                ram_fallback = decision.forced_fallback();
            }
        }

        // 2. File selection. A torrent that never accepts it is a warning, not
        //    an error: playback still works, just with the engine's own order.
        //    The engine logs its own retry transitions; this line records how
        //    long the caller waited and whether the wait succeeded.
        //
        //    SKIPPED for a RAM-backed torrent: those are added with every file
        //    deselected, and librqbit fetches exactly the window around each
        //    open reader. Selecting the file here would queue the WHOLE
        //    episode into the buffer -- the disk-mode failure mode this code
        //    exists to prevent, rebuilt in memory. The check is
        //    `is_ram_backed`, not `ram_mode`: a forced fallback puts a RAM
        //    torrent in a session that is otherwise disk-mode.
        if !session.engine.is_ram_backed(torrent_id) {
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
        } else {
            tracing::info!(
                "prepare_stream: torrent={torrent_id} file={file_idx} RAM mode; \
                 the streaming window does the selecting"
            );
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

        // Mark the torrent as live for the restart guard: a URL about to be
        // opened counts as playback even before the player asks for a byte.
        if let Ok(mut streams) = self.inner.streams.lock() {
            streams.insert(torrent_id);
        }

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
            return Ok(PreparedStream {
                torrent_id,
                url,
                ram_fallback,
            });
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

        Ok(PreparedStream {
            torrent_id,
            url,
            ram_fallback,
        })
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
    /// Raw `.torrent` metadata for a live torrent, without starting a session.
    pub fn torrent_bytes(&self, torrent_id: usize) -> Option<Vec<u8>> {
        self.session_snapshot()?.engine.torrent_bytes(torrent_id)
    }

    /// The files a LIVE torrent resolved to, or an empty list.
    ///
    /// Reads the session cell rather than going through [`Self::session`] for
    /// the same reason as the progress readers: the download poller asks about
    /// a torrent it just added, and a missing session there means "gone", not
    /// "start one".
    pub fn torrent_files(&self, torrent_id: usize) -> Vec<TorrentFile> {
        let Some(session) = self.session_snapshot() else {
            return Vec::new();
        };

        session
            .engine
            .torrent_files(torrent_id)
            .into_iter()
            .map(|(idx, name, length_bytes)| TorrentFile {
                idx,
                name,
                length_bytes,
            })
            .collect()
    }

    /// Progress and completion for one file, WITHOUT starting a session.
    ///
    /// The permanent-download poller calls this once a second per active
    /// download. It reads the session cell directly for the same reason as
    /// [`Self::torrent_progress`]: polling must never bind a bridge or join the
    /// DHT for a torrent that was never added.
    ///
    /// The bool is "this file is fully on disk"; see
    /// [`crate::torrent::TorrentEngine::file_complete`] for what counts.
    pub fn download_snapshot(
        &self,
        torrent_id: usize,
        file_idx: usize,
    ) -> Option<(TorrentProgress, bool)> {
        let session = self.session_snapshot()?;
        let progress = session.engine.progress(torrent_id)?;
        let done = session.engine.file_complete(torrent_id, file_idx);
        Some((progress, done))
    }

    pub fn torrent_progress(&self, torrent_id: usize) -> Option<TorrentProgress> {
        self.session_snapshot()?.engine.progress(torrent_id)
    }

    /// Pause a torrent's transfer, keeping its partial data.
    ///
    /// Reads the session cell rather than going through [`Self::session`], for
    /// the same reason as [`Self::torrent_progress`]: pausing a torrent that
    /// was never added must not START a session merely to pause nothing.
    pub async fn pause_torrent(&self, id: usize) -> Result<()> {
        let Some(session) = self.session_snapshot() else {
            return Ok(());
        };

        session.engine.pause_torrent(id).await
    }

    /// Resume a paused torrent's transfer.
    ///
    /// The counterpart of [`Self::pause_torrent`], with the same cell-read
    /// rule so resuming nothing cannot start a session.
    pub async fn resume_torrent(&self, id: usize) -> Result<()> {
        let Some(session) = self.session_snapshot() else {
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
        let Some(session) = self.session_snapshot() else {
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

    // Head. BOUNDED twice over: at WARM_HEAD_BYTES so it cannot become an
    // "everything to EOF" download, and at its OWN short total timeout so a
    // cold torrent (nothing to serve yet) gives up quickly instead of
    // waiting out the long tail budget. This is the caller's remaining
    // latency, even though the whole warm-up is detached, because it
    // competes for pieces with mpv's first read. A total timeout is fine
    // here: the head is 2 MiB, so even a sluggish swarm finishes inside it,
    // and the request is awaited so it must stay cheap.
    let head_at = std::time::Instant::now();
    let head_range = format!("bytes=0-{}", WARM_HEAD_BYTES.saturating_sub(1));
    let head_client = match reqwest::Client::builder().timeout(WARM_HEAD_TIMEOUT).build() {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!("warm-up client unavailable, skipping warm-up: {err:#}");
            return;
        }
    };
    match fetch_first_chunk(&head_client, url, &head_range, None).await {
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
    // The fetch is spawned detached: it pulls the pieces in the background
    // while mpv is starting, and the URL is returned immediately. Whatever
    // arrives before mpv asks is free progress; whatever does not, mpv waits
    // for on its own request, exactly as it would have without us.
    //
    // There is NO whole-request timeout here on purpose. The tail drains
    // until its 4 MiB range is served or the swarm stops MOVING (see
    // [`WARM_IDLE_TIMEOUT`]): the total timeout used to kill a slow tail
    // mid-piece, and the missing end bytes are precisely the seek index mpv
    // then blocks on. A failed attempt retries once -- pieces it delivered
    // stay resident, so the retry typically completes within a second or
    // two.
    match length {
        Some(len) if len > WARM_TAIL_BYTES => {
            let start = len - WARM_TAIL_BYTES;
            let range = format!("bytes={start}-");
            let url = url.to_string();
            tokio::spawn(async move {
                let tail_at = std::time::Instant::now();
                let mut attempt = 1u32;
                // A `loop` expression rather than a for over attempts: the
                // result is the loop's value, so there is no dead
                // initializer for the compiler to flag and no way to fall
                // out of the retry budget without an outcome.
                let result: Result<u64> = loop {
                    let client = match reqwest::Client::builder()
                        .connect_timeout(WARM_CONNECT_TIMEOUT)
                        .build()
                    {
                        Ok(client) => client,
                        Err(err) => break Err(anyhow!("warm-up client unavailable: {err:#}")),
                    };
                    match fetch_first_chunk(&client, &url, &range, Some(WARM_IDLE_TIMEOUT)).await {
                        Ok(bytes) => break Ok(bytes),
                        Err(err) => {
                            attempt += 1;
                            if attempt > WARM_TAIL_ATTEMPTS {
                                break Err(err);
                            }
                            tokio::time::sleep(WARM_RETRY_DELAY).await;
                        }
                    }
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
                        "warm tail FAILED after {}ms over {WARM_TAIL_ATTEMPTS} attempts \
                         (range={range}): {err:#}",
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
/// bounded by [`WARM_DRAIN_BYTES`], and per chunk by `idle_timeout` when one
/// is given (otherwise by the client's own total timeout), so a torrent with
/// no peers still cannot hang playback.
async fn fetch_first_chunk(
    client: &reqwest::Client,
    url: &str,
    range: &str,
    idle_timeout: Option<std::time::Duration>,
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
    // waiting for the whole (possibly huge) remainder of the file. With an
    // idle timeout each wait is capped separately, which is what lets a slow
    // but moving transfer run to completion while a stalled one gives up.
    let mut drained: u64 = 0;
    loop {
        if drained >= WARM_DRAIN_BYTES {
            break;
        }
        let chunk = match idle_timeout {
            Some(idle) => match tokio::time::timeout(idle, response.chunk()).await {
                Ok(inner) => inner,
                Err(_) => {
                    return Err(anyhow!(
                        "warm-up {range} stalled: no bytes for {}s (drained {drained})",
                        idle.as_secs()
                    ));
                }
            },
            None => response.chunk().await,
        };
        match chunk {
            Ok(Some(chunk)) => {
                drained += chunk.len() as u64;
            }
            // Clean end of body: the requested range was fully delivered.
            Ok(None) => break,
            Err(err) => {
                // Partial data is still useful: the pieces already delivered
                // stay in the pool and keep their priority. Report what
                // arrived.
                return Err(anyhow!(
                    "warm-up {range} stopped after {drained} bytes: {err:#}"
                ));
            }
        }
    }

    Ok(drained)
}

/// Move `source` to `target`, across filesystems when necessary.
///
/// `std::fs::rename` fails with `CrossesDevices` when the library lives on a
/// different mount from the download directory, which is exactly the layout a
/// reader with a dedicated media drive has. Falling back to copy + delete keeps
/// that working; the copy goes to a `.part` sibling first so an interrupted
/// copy cannot leave a truncated file that looks like a finished download.
fn move_file(source: &Path, target: &Path) -> Result<()> {
    match std::fs::rename(source, target) {
        Ok(()) => return Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::CrossesDevices => {
            tracing::info!(
                "download crosses filesystems, copying {} instead of renaming",
                source.display()
            );
        }
        Err(err) => return Err(err).context("rename into the library"),
    }

    let partial = target.with_extension("part");
    std::fs::copy(source, &partial)
        .with_context(|| format!("failed to copy into {}", partial.display()))?;
    std::fs::rename(&partial, target).context("failed to publish the copied file")?;
    std::fs::remove_file(source)
        .with_context(|| format!("failed to delete {}", source.display()))?;
    Ok(())
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
            state.session_snapshot().is_none(),
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
            state.session_snapshot().is_none(),
            "polling progress must not start a session"
        );
    }

    /// A hold alone must not report a removal as due: nothing asked for one.
    #[test]
    /// Without a settings store the built-in defaults decide, and those say
    /// RAM: a play must not be quietly downgraded to disk.
    #[test]
    fn the_default_settings_stream_from_ram() {
        let state = PlayerState::new();
        let decision = state.decide_playback_mode(Some(1_000_000));
        assert!(decision.is_ram());
        assert_eq!(
            decision.forced_fallback(),
            None,
            "the reader's own choice is not a fallback"
        );
    }

    /// Classic mode on a volume that cannot spare the space must become a RAM
    /// fallback, and it must be MARKED as one so the UI can say so.
    #[test]
    fn classic_mode_falls_back_when_the_episode_cannot_fit() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = std::sync::Arc::new(SettingsStore::at(dir.path().join("settings.json")));
        let mut settings = crate::settings::Settings::default();
        settings.ram_streaming = false;
        // A reserve no volume can satisfy, so the answer cannot depend on how
        // much space the test machine happens to have.
        settings.disk_reserve_mib = u32::MAX;
        store.write(&settings).expect("write settings");

        let state = PlayerState::with_settings(store);
        let decision = state.decide_playback_mode(Some(1024));

        assert!(decision.is_ram());
        assert_eq!(
            decision.forced_fallback(),
            Some(RamFallbackReason::NoDiskSpace)
        );
    }

    /// And the opposite: with no reserve and a one-byte episode the disk wins,
    /// so the fallback is not simply "always RAM in classic mode".
    #[test]
    fn classic_mode_keeps_disk_when_the_episode_fits() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = std::sync::Arc::new(SettingsStore::at(dir.path().join("settings.json")));
        let mut settings = crate::settings::Settings::default();
        settings.ram_streaming = false;
        settings.disk_reserve_mib = 0;
        store.write(&settings).expect("write settings");

        let state = PlayerState::with_settings(store);
        assert_eq!(state.decide_playback_mode(Some(1)), StreamingDecision::Disk);
    }

    /// Changing the mode with nothing playing is a no-op, because the next
    /// session reads the setting anyway. Pinned so the restart cannot start a
    /// session merely to tear it down.
    #[tokio::test]
    async fn applying_the_mode_before_any_session_is_nothing_to_do() {
        let state = PlayerState::new();

        assert_eq!(
            state.apply_streaming_mode().await.expect("should not fail"),
            StreamingRestart::NothingToDo
        );
        assert!(state.session_snapshot().is_none());
    }

    /// A hold alone must keep the session alive: tearing it down would break
    /// the URL the player is reading.
    #[tokio::test]
    async fn a_held_torrent_blocks_the_restart() {
        let state = PlayerState::new();
        state.hold_torrent(5);

        assert!(state.has_active_playback());
        assert_eq!(
            state.apply_streaming_mode().await.expect("should not fail"),
            StreamingRestart::Busy,
            "a held torrent must refuse the restart before the session is taken"
        );
    }

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
