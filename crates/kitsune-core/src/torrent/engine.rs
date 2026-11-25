//! Thin wrapper over a `librqbit` session.
//!
//! Deliberately small: everything that can be decided without touching the
//! network lives in [`super::magnet`] or in the option-builder function
//! below, both of which are unit tested. This type only owns the session
//! lifecycle, so its untested surface stays as close to zero as possible.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use librqbit::storage::StorageFactoryExt;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session};
use serde::Serialize;

use super::ram_storage::{RamBufferConfig, RamPool, RamPoolStats, RamStorageFactory};

/// How long [`TorrentEngine::set_only_files`] keeps retrying before giving up.
///
/// librqbit refuses a file selection while it is hashing existing data, and
/// that hash can take a while on a large resumed release. The wait is bounded
/// so a torrent that never leaves `Initializing` cannot hang the play path.
const ONLY_FILES_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// How often [`TorrentEngine::set_only_files`] re-checks the torrent state.
const ONLY_FILES_POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// A download-progress snapshot for one torrent, as the watch page polls it.
///
/// Built from librqbit's own `TorrentStats` by [`summarize`], which is pure so
/// the mapping is unit tested without a session. The live-only fields (speed,
/// peers, ETA) read as zero/`None` while the torrent is not running, so the
/// caller always gets a complete snapshot instead of an `Option` to unwrap.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentProgress {
    /// `"initializing"`, `"live"`, `"paused"` or `"error"`.
    pub state: String,
    /// Bytes of the whole torrent downloaded so far.
    pub progress_bytes: u64,
    /// Size of the whole torrent.
    pub total_bytes: u64,
    /// Bytes downloaded for each file, indexed by the file idx used in the
    /// stream URL. Lets the frontend show the chosen episode's own progress.
    pub file_progress: Vec<u64>,
    /// True once every selected piece has been downloaded.
    pub finished: bool,
    /// The torrent's failure message, when it is in the error state.
    pub error: Option<String>,
    /// Current download rate in MiB/s. Zero when not live.
    pub download_mbps: f64,
    /// Current upload rate in MiB/s. Zero when not live.
    pub upload_mbps: f64,
    /// Estimated seconds until the torrent completes, when it can be derived.
    pub eta_seconds: Option<u64>,
    /// Peers currently transferring pieces.
    pub peers_live: u32,
    /// Peers we are handshaking with.
    pub peers_connecting: u32,
    /// Peers waiting for a free slot.
    pub peers_queued: u32,
    /// Peers known but not yet connected.
    pub peers_seen: u32,
    /// The shared RAM streaming buffer's stats, when this torrent streams
    /// from RAM. `None` for disk downloads. The frontend uses this for a
    /// "buffer 84 MiB / 256 MiB" readout and to surface ghost re-downloads.
    pub ram_buffer: Option<RamPoolStats>,
}

/// Map librqbit's `TorrentStats` onto the frontend-facing [`TorrentProgress`].
///
/// Pure, so the whole mapping is testable without starting a session. The only
/// piece computed rather than copied is the ETA: librqbit's own
/// `time_remaining` hides its seconds field, so the estimate is derived from
/// the remaining bytes and the live download rate.
pub fn summarize(stats: &librqbit::TorrentStats) -> TorrentProgress {
    use librqbit::TorrentStatsState;

    let state = match &stats.state {
        TorrentStatsState::Initializing { .. } => "initializing",
        TorrentStatsState::Live => "live",
        TorrentStatsState::Paused => "paused",
        TorrentStatsState::Error => "error",
    }
    .to_string();

    // Live-only fields default to zero, so a paused or errored torrent still
    // reports a complete snapshot rather than forcing the caller to handle a
    // half-populated one.
    let mut download_mbps = 0.0;
    let mut upload_mbps = 0.0;
    let mut eta_seconds = None;
    let mut peers_live = 0;
    let mut peers_connecting = 0;
    let mut peers_queued = 0;
    let mut peers_seen = 0;

    if let Some(live) = &stats.live {
        download_mbps = live.download_speed.mbps;
        upload_mbps = live.upload_speed.mbps;

        let peers = &live.snapshot.peer_stats;
        peers_live = peers.live;
        peers_connecting = peers.connecting;
        peers_queued = peers.queued;
        peers_seen = peers.seen;

        let remaining = stats.total_bytes.saturating_sub(stats.progress_bytes);
        let bytes_per_second = live.download_speed.as_bytes();
        if bytes_per_second > 0 && remaining > 0 {
            eta_seconds = Some(remaining / bytes_per_second);
        }
    }

    TorrentProgress {
        state,
        progress_bytes: stats.progress_bytes,
        total_bytes: stats.total_bytes,
        file_progress: stats.file_progress.clone(),
        finished: stats.finished,
        error: stats.error.clone(),
        download_mbps,
        upload_mbps,
        eta_seconds,
        peers_live,
        peers_connecting,
        peers_queued,
        peers_seen,
        ram_buffer: None,
    }
}

/// Configuration for a streaming session.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Where downloaded pieces are written.
    ///
    /// Unused by RAM torrents (nothing is written), but still created: the
    /// session root is where librqbit keeps its (empty) bookkeeping, and a
    /// mixed session needs it for the disk torrents.
    pub download_dir: PathBuf,
    /// Download cap in bytes per second, or `None` for unlimited.
    ///
    /// Set at session start because librqbit's live `Session.ratelimits` has
    /// private fields and no setters -- the only way to change a cap is to
    /// build a new session. The queue's concurrency limit is the lever that
    /// takes effect without a restart.
    pub download_bps: Option<u32>,
    /// Upload cap in bytes per second, or `None` for unlimited.
    pub upload_bps: Option<u32>,
    /// Stream from a bounded RAM buffer instead of writing pieces to disk.
    ///
    /// When `Some`, torrents added through the PLAYBACK path
    /// ([`TorrentEngine::add_magnet`] / [`TorrentEngine::add_torrent_file`])
    /// get the in-memory LRU storage and start with every file deselected,
    /// so only the pieces around each open reader are ever downloaded.
    /// Permanent downloads ([`Self::start_download_in`] in player state)
    /// keep writing to disk regardless. `None` keeps the historical
    /// disk-backed behaviour for everything.
    pub ram: Option<RamBufferConfig>,
    /// Peers to try connecting to first, before any tracker or DHT answer.
    ///
    /// Empty by default. Exists for tests (an in-process seeder discovered
    /// without network) and for a future "play over LAN" feature.
    pub initial_peers: Vec<SocketAddr>,
    /// Bind incoming peer connections to a fixed port instead of librqbit's
    /// ephemeral default. Also for tests and LAN setups.
    pub listen_port: Option<u16>,
    /// Leave the DHT alone. Tests set this so a hermetic run neither waits
    /// on DHT bootstrap nor reaches the public network.
    pub disable_dht: bool,
}

impl EngineConfig {
    /// Unlimited rates on disk, which is the historical behaviour.
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            download_dir: download_dir.into(),
            download_bps: None,
            upload_bps: None,
            ram: None,
            initial_peers: Vec::new(),
            listen_port: None,
            disable_dht: false,
        }
    }

    /// The same config with bandwidth caps applied.
    pub fn with_limits(mut self, download_bps: Option<u32>, upload_bps: Option<u32>) -> Self {
        self.download_bps = download_bps;
        self.upload_bps = upload_bps;
        self
    }

    /// The same config with RAM streaming configured.
    pub fn with_ram(mut self, ram: RamBufferConfig) -> Self {
        self.ram = Some(ram);
        self
    }

    /// The same config with initial peers to try first.
    pub fn with_initial_peers(mut self, peers: Vec<SocketAddr>) -> Self {
        self.initial_peers = peers;
        self
    }

    /// The same config bound to a fixed listen port.
    pub fn with_listen_port(mut self, port: u16) -> Self {
        self.listen_port = Some(port);
        self
    }

    /// The same config with the DHT disabled.
    pub fn without_dht(mut self) -> Self {
        self.disable_dht = true;
        self
    }

    /// The `SessionOptions` these settings imply.
    ///
    /// Built with `..Default::default()` on purpose: `SessionOptions` has 21
    /// fields covering the DHT, persistence, listeners and peer limits, and
    /// naming them all here would silently change every one of those defaults
    /// the moment librqbit added another.
    ///
    /// librqbit takes `NonZeroU32`, so a limit of zero means "unlimited" rather
    /// than a dead transfer.
    pub fn session_options(&self) -> librqbit::SessionOptions {
        use std::num::NonZeroU32;

        let mut options = librqbit::SessionOptions {
            ratelimits: librqbit::limits::LimitsConfig {
                download_bps: self.download_bps.and_then(NonZeroU32::new),
                upload_bps: self.upload_bps.and_then(NonZeroU32::new),
            },
            ..Default::default()
        };

        if self.disable_dht {
            options.dht = None;
        }

        if let Some(port) = self.listen_port {
            options.listen = Some(librqbit::ListenerOptions {
                listen_addr: SocketAddr::from(([0, 0, 0, 0], port)),
                ..Default::default()
            });
        }

        options
    }
}

/// Build the `AddTorrentOptions` used when adding a torrent for playback.
///
/// Split out as a pure function so the option policy is asserted in tests
/// without starting a session. Sequential streaming is librqbit's default,
/// so these options only cover resume behaviour and output location.
pub fn playback_options(output_folder: Option<String>) -> AddTorrentOptions {
    AddTorrentOptions {
        // Resume/seed on top of existing files instead of refusing to start.
        // Needed for replay, where pieces from a previous run are on disk.
        overwrite: true,
        output_folder,
        // We intend to download and stream, so never list-only.
        list_only: false,
        ..Default::default()
    }
}

/// Build the `AddTorrentOptions` for a RAM-streamed playback torrent.
///
/// Two choices make "stream from RAM" work on librqbit 9 without touching
/// its internals:
///
/// 1. **The storage factory.** A per-session [`RamStorageFactory`] shares
///    one bounded LRU pool across every RAM torrent, so the cap applies to
///    the app, not per torrent.
/// 2. **Every file deselected.** With an empty selection librqbit's own
///    download queue stays empty, and pieces are pulled ONLY by its
///    streaming window -- the look-ahead around each open reader. That is
///    the reader-driven selection the prototype used; without it librqbit
///    would bulk-download the whole file into RAM from piece 0.
pub fn ram_playback_options(storage_factory: RamStorageFactory) -> AddTorrentOptions {
    AddTorrentOptions {
        overwrite: true,
        list_only: false,
        // "Select nothing": the streaming window does the selecting.
        only_files: Some(Vec::new()),
        storage_factory: Some(storage_factory.boxed()),
        ..Default::default()
    }
}

/// Options for a permanent episode download.
///
/// The output folder is the torrent's OWN cache directory, not the session
/// root: stable per-info-hash paths are what let a restart find the pieces it
/// left behind even after the reader changes the download setting.
pub fn download_options(
    output_folder: String,
    only_files: Vec<usize>,
    paused: bool,
) -> AddTorrentOptions {
    AddTorrentOptions {
        paused,
        only_files: Some(only_files),
        overwrite: true,
        list_only: false,
        output_folder: Some(output_folder),
        ..Default::default()
    }
}

/// Whether librqbit will accept a file selection in this state.
///
/// Only a RUNNING torrent has piece trackers to re-prioritise; during
/// `Initializing` it is still hashing what is already on disk, and after an
/// error there is nothing to prioritise. Pure, so the wait loop's exit
/// conditions are asserted without starting a session.
pub fn accepts_file_selection(state: &librqbit::TorrentStatsState) -> bool {
    matches!(
        state,
        librqbit::TorrentStatsState::Live | librqbit::TorrentStatsState::Paused
    )
}

/// Whether the engine's selection now matches what was asked for.
///
/// `only_files()` builds a `Vec` from a set, so the order is arbitrary; the
/// comparison must be by membership. Also pure, so the verification step is
/// tested rather than trusted.
pub fn selection_matches(wanted: &std::collections::HashSet<usize>, applied: &[usize]) -> bool {
    applied.len() == wanted.len() && applied.iter().all(|idx| wanted.contains(idx))
}

/// Ensure `dir` exists, creating parents as needed.
///
/// Separate from [`TorrentEngine::start`] so the filesystem behaviour can be
/// tested on its own.
pub fn ensure_download_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create download dir {}", dir.display()))
}

/// Bytes available on the filesystem that holds `path`.
///
/// `None` means "cannot tell" -- a path that does not exist yet, a platform
/// whose disk list came back empty, or a volume the probe could not match.
/// Callers must treat `None` as "assume there is room": refusing to play
/// because a probe failed would be worse than trying and reporting a real
/// error later.
///
/// The disk whose mount point is the LONGEST prefix of `path` is the right
/// one: `/` is a prefix of every absolute path, so a short match would report
/// the root volume for a file on a mounted drive.
///
/// `KITSUNE_FAKE_AVAILABLE_BYTES`, when set, clamps the reported free space
/// to that value. It is a dev knob for scripts/dev/low-disk.sh's no-root
/// mode: the refusal path can be exercised on a healthy disk without sudo.
pub fn available_space(path: &Path) -> Option<u64> {
    available_space_floor(
        probe_available_space(path),
        fake_available_bytes(),
    )
}

/// The sysinfo walk: the free bytes of the volume with the longest mount
/// matching `path`. Split out so the override clamping is testable pure.
fn probe_available_space(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();

    let mut best: Option<(usize, u64)> = None;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if path.starts_with(mount) {
            let len = mount.as_os_str().len();
            if best.is_none_or(|(best_len, _)| len > best_len) {
                best = Some((len, disk.available_space()));
            }
        }
    }

    best.map(|(_, free)| free)
}

/// Combine the probe with the dev override. The fake can only make a volume
/// look LESS full than reality; where the probe saw nothing at all, the fake
/// stands in entirely.
fn available_space_floor(real: Option<u64>, fake: Option<u64>) -> Option<u64> {
    match (fake, real) {
        (Some(fake), Some(real)) => Some(fake.min(real)),
        (Some(fake), None) => Some(fake),
        (None, real) => real,
    }
}

/// The `KITSUNE_FAKE_AVAILABLE_BYTES` dev override, read once.
///
/// `None` when unset or unparseable: a bad value must not silently lie about
/// every volume for the whole run, it should behave as if unset.
fn fake_available_bytes() -> Option<u64> {
    static FAKE: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    *FAKE.get_or_init(|| {
        std::env::var("KITSUNE_FAKE_AVAILABLE_BYTES")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
    })
}

/// Whether `needed_bytes` fits on `path`'s volume while keeping `reserve`
/// bytes free.
///
/// Returns `true` when the free space cannot be determined, so an unreadable
/// volume degrades to the historical behaviour instead of blocking playback.
pub fn fits_on_disk(path: &Path, needed_bytes: u64, reserve_bytes: u64) -> bool {
    match available_space(path) {
        Some(free) => free >= needed_bytes.saturating_add(reserve_bytes),
        None => true,
    }
}

/// A running torrent session.
pub struct TorrentEngine {
    session: Arc<Session>,
    download_dir: PathBuf,
    /// The shared RAM pool when the session streams in RAM mode.
    ///
    /// `None` means every torrent writes to disk (the historical mode).
    ram: Option<Arc<RamPool>>,
    /// A RAM pool created on demand in a DISK-mode session.
    ///
    /// Playback falls back to RAM when the download volume cannot spare the
    /// episode's bytes, and a disk-mode session has no pool to hand. This
    /// holds the one created for such a fallback, so every forced torrent
    /// shares the same bounded buffer instead of one pool per episode.
    lazy_ram: std::sync::OnceLock<Arc<RamPool>>,
    /// Peers to try before tracker/DHT answers, applied to playback adds.
    initial_peers: Vec<SocketAddr>,
}

impl TorrentEngine {
    /// Start a session, creating `download_dir` if needed.
    ///
    /// This binds sockets and starts the DHT, so it is exercised by the
    /// `#[ignore]`d test below rather than the default suite.
    pub async fn start(config: EngineConfig) -> Result<Self> {
        ensure_download_dir(&config.download_dir)?;

        // `new_with_opts` rather than `new`: the rate limits only exist on the
        // options struct, so `new` would silently ignore the reader's cap.
        let session = Session::new_with_opts(config.download_dir.clone(), config.session_options())
            .await
            .context("failed to start torrent session")?;

        // One pool per SESSION, shared by every RAM torrent it manages: the
        // buffer cap is a promise about the app's memory, not per episode.
        let ram = config.ram.map(|buffer| {
            tracing::info!(
                buffer_mib = buffer.buffer_bytes / 1024 / 1024,
                "RAM streaming enabled; pieces are held in memory only"
            );
            Arc::new(RamPool::new(buffer))
        });

        Ok(Self {
            session,
            download_dir: config.download_dir,
            ram,
            lazy_ram: std::sync::OnceLock::new(),
            initial_peers: config.initial_peers,
        })
    }

    /// The pool forced-RAM torrents share, creating it on first use.
    ///
    /// In a RAM-mode session this is the session pool, so a fallback and a
    /// normal play draw on the same buffer. In a disk-mode session it is
    /// created lazily with the prototype's default size, because the reader
    /// never chose a size for a mode they did not pick.
    pub fn ensure_ram_pool(&self) -> Arc<RamPool> {
        if let Some(pool) = &self.ram {
            return pool.clone();
        }
        self.lazy_ram
            .get_or_init(|| {
                tracing::info!(
                    "creating a RAM pool for a forced playback fallback in a disk-mode session"
                );
                Arc::new(RamPool::new(RamBufferConfig::default_buffer()))
            })
            .clone()
    }

    /// Whether this engine can hand out a RAM pool.
    ///
    /// True for a RAM-mode session and for a disk-mode one after a fallback
    /// created the lazy pool. Distinct from [`Self::ram_mode`], which answers
    /// what the session was STARTED as.
    pub fn has_ram_pool(&self) -> bool {
        self.ram.is_some() || self.lazy_ram.get().is_some()
    }

    /// The playback add options for THIS engine.
    ///
    /// `force_ram` sends this one torrent to the RAM pool even when the
    /// session was started in disk mode -- the automatic fallback for a
    /// volume without room. Without it the session's own choice applies.
    fn playback_options_for(&self, force_ram: bool) -> AddTorrentOptions {
        let pool = if force_ram {
            Some(self.ensure_ram_pool())
        } else {
            self.ram.clone()
        };

        let mut options = match pool {
            Some(pool) => ram_playback_options(RamStorageFactory::new(pool)),
            None => playback_options(None),
        };

        if !self.initial_peers.is_empty() {
            options.initial_peers = Some(self.initial_peers.clone());
        }

        options
    }

    /// Add a magnet URI and return the engine's torrent id.
    ///
    /// The magnet is validated first so a malformed link produces a clear
    /// error instead of an opaque engine failure.
    pub async fn add_magnet(&self, magnet_uri: &str) -> Result<usize> {
        self.add_magnet_with_force(magnet_uri, false).await
    }

    /// Add a magnet for playback, optionally forcing it into the RAM pool.
    ///
    /// `force_ram` is the disk-full fallback: the caller has already decided
    /// this torrent cannot be written to disk and wants the in-memory
    /// storage whatever the session's own mode is.
    pub async fn add_magnet_with_force(&self, magnet_uri: &str, force_ram: bool) -> Result<usize> {
        super::magnet::parse_info_hash(magnet_uri)
            .map_err(|e| anyhow!("invalid magnet URI: {e}"))?;

        self.add_parsed(
            AddTorrent::from_url(magnet_uri),
            self.playback_options_for(force_ram),
        )
        .await
    }

    /// Add a magnet with caller-supplied options.
    pub async fn add_magnet_with_options(
        &self,
        magnet_uri: &str,
        options: AddTorrentOptions,
    ) -> Result<usize> {
        super::magnet::parse_info_hash(magnet_uri)
            .map_err(|e| anyhow!("invalid magnet URI: {e}"))?;

        self.add_parsed(AddTorrent::from_url(magnet_uri), options)
            .await
    }

    /// Add a local `.torrent` file and return the engine's torrent id.
    ///
    /// Used for side-loading a torrent the user already has, and for
    /// smoke tests that must not depend on a live swarm.
    pub async fn add_torrent_file(&self, path: &Path) -> Result<usize> {
        self.add_torrent_file_with_force(path, false).await
    }

    /// Add a local `.torrent` for playback, optionally forcing RAM storage.
    ///
    /// The file counterpart of [`Self::add_magnet_with_force`], for the
    /// fallback when the reader side-loaded a torrent instead of a magnet.
    pub async fn add_torrent_file_with_force(
        &self,
        path: &Path,
        force_ram: bool,
    ) -> Result<usize> {
        self.add_torrent_file_with_options(path, self.playback_options_for(force_ram))
            .await
    }

    /// Add a cached `.torrent` file with caller-supplied options.
    pub async fn add_torrent_file_with_options(
        &self,
        path: &Path,
        options: AddTorrentOptions,
    ) -> Result<usize> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow!("torrent path is not valid UTF-8"))?;

        let add = AddTorrent::from_local_filename(path_str)
            .with_context(|| format!("failed to read torrent file {}", path.display()))?;

        self.add_parsed(add, options).await
    }

    /// Shared add path for magnets and local torrent files.
    async fn add_parsed(&self, add: AddTorrent<'_>, options: AddTorrentOptions) -> Result<usize> {
        let response = self
            .session
            .add_torrent(add, Some(options))
            .await
            .context("failed to add torrent")?;

        match response {
            AddTorrentResponse::ListOnly(_) => {
                Err(anyhow!("torrent was added in list-only mode unexpectedly"))
            }
            other => {
                let handle = other
                    .into_handle()
                    .ok_or_else(|| anyhow!("torrent was added but no handle was returned"))?;
                Ok(handle.id())
            }
        }
    }

    /// Files inside a torrent, once metadata has been resolved.
    ///
    /// Returns `(file_idx, name, length_bytes)`. `file_idx` is what the
    /// stream URL needs; without metadata this is empty rather than an
    /// error, since magnet metadata arrives asynchronously.
    pub fn torrent_files(&self, id: usize) -> Vec<(usize, String, u64)> {
        let Some(handle) = self.session.get(id.into()) else {
            return Vec::new();
        };

        let mut files = Vec::new();
        let _ = handle.with_metadata(|meta| {
            for (idx, info) in meta.file_infos.iter().enumerate() {
                files.push((idx, info.relative_filename.display().to_string(), info.len));
            }
        });

        files
    }

    /// A download-progress snapshot for `id`, or `None` if it is not managed.
    ///
    /// The watch page polls this while waiting for enough of an episode to be
    /// downloaded before the player can start. A missing id reports `None`
    /// rather than an error: the torrent may already have been removed, and a
    /// removed torrent is not a failure to report.
    pub fn progress(&self, id: usize) -> Option<TorrentProgress> {
        let handle = self.session.get(id.into())?;
        let ram_buffer = if self.is_ram_backed(id) {
            self.ram.as_ref().map(|pool| pool.stats())
        } else {
            None
        };
        Some(TorrentProgress {
            ram_buffer,
            ..summarize(&handle.stats())
        })
    }

    /// The underlying session, for building the HTTP `Api` facade.
    pub fn session(&self) -> Arc<Session> {
        self.session.clone()
    }

    /// Display name of a torrent, once its metadata has been resolved.
    pub fn torrent_name(&self, id: usize) -> Option<String> {
        self.session.get(id.into()).and_then(|handle| handle.name())
    }

    /// Path this session writes downloads to.
    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }

    /// Whether this session streams playback from RAM.
    pub fn ram_mode(&self) -> bool {
        self.ram.is_some()
    }

    /// The shared RAM pool, when streaming in RAM mode.
    pub fn ram_pool(&self) -> Option<Arc<RamPool>> {
        self.ram.clone()
    }

    /// A snapshot of the RAM pool, or `None` in disk mode.
    pub fn ram_stats(&self) -> Option<RamPoolStats> {
        self.ram.as_ref().map(|pool| pool.stats())
    }

    /// Whether torrent `id` is currently backed by the RAM pool.
    ///
    /// False in disk mode, and false for a torrent the pool does not know
    /// (metadata not yet resolved, or already removed).
    pub fn is_ram_backed(&self, id: usize) -> bool {
        let Some(pool) = &self.ram else {
            return false;
        };
        let Some(handle) = self.session.get(id.into()) else {
            return false;
        };
        pool.is_ram_backed(handle.info_hash())
    }

    /// On-disk path where librqbit writes file `file_idx` of torrent `id`.
    ///
    /// The handle's own output folder is authoritative. A torrent added with
    /// an explicit `output_folder` writes `{output_folder}/{relative_name}`;
    /// one added without it gets a session-generated subfolder that is already
    /// baked into `output_folder`. Deriving the base from the session root was
    /// wrong for per-torrent cache directories and for single-file torrents.
    ///
    /// `None` for a RAM-backed torrent: it has no on-disk path, and callers
    /// must not treat its buffer as a file.
    pub fn file_disk_path(&self, id: usize, file_idx: usize) -> Option<PathBuf> {
        let handle = self.session.get(id.into())?;
        if self
            .ram
            .as_ref()
            .is_some_and(|pool| pool.is_ram_backed(handle.info_hash()))
        {
            return None;
        }
        let base = handle.output_folder().to_path_buf();
        let relative = handle
            .with_metadata(|meta| {
                meta.file_infos
                    .iter()
                    .nth(file_idx)
                    .map(|info| info.relative_filename.clone())
            })
            .ok()??;
        Some(base.join(relative))
    }

    /// The raw `.torrent` bytes, once metadata has resolved.
    ///
    /// Saved beside the download store so a restart can reattach without
    /// asking the swarm for metadata again.
    pub fn torrent_bytes(&self, id: usize) -> Option<Vec<u8>> {
        let handle = self.session.get(id.into())?;
        handle
            .with_metadata(|meta| meta.torrent_bytes.to_vec())
            .ok()
    }

    /// Whether every byte of `file_idx` is on disk.
    ///
    /// Used by the permanent-download path, which waits for one FILE rather
    /// than the whole torrent: a season pack restricted with
    /// [`Self::set_only_files`] reports `finished` only after every selected
    /// piece lands, and the selected set is normally this one file, but
    /// checking the file directly is what makes the answer right even when the
    /// restriction did not stick.
    ///
    /// An unknown torrent, unknown file, or unresolved metadata all read as
    /// "not complete" rather than an error: the caller is polling.
    pub fn file_complete(&self, id: usize, file_idx: usize) -> bool {
        let Some(handle) = self.session.get(id.into()) else {
            return false;
        };

        let stats = summarize(&handle.stats());

        let Some((_, _, len)) = self
            .torrent_files(id)
            .into_iter()
            .find(|(idx, _, _)| *idx == file_idx)
        else {
            return false;
        };

        // Deliberately does NOT shortcut on `stats.finished`. A torrent whose
        // selected set is EMPTY reports `finished` immediately -- 0 needed, 0
        // selected -- which is exactly the state a download sits in for the
        // moment before `set_only_files` is accepted, and also the state it is
        // stuck in if that call never lands. Trusting it marked downloads
        // complete with no bytes written, and the finisher then failed with
        // "nothing is there".
        //
        // `file_progress` is bytes ON DISK per file, so comparing it against
        // the metadata length is the only check that answers the real question.
        let got = stats.file_progress.get(file_idx).copied().unwrap_or(0);
        len > 0 && got >= len
    }

    /// Drop a torrent from the session, KEEPING the pieces it downloaded.
    ///
    /// Removing the torrent is what stops the transfer: the session keeps
    /// running, but the bridge stops serving this id and no peer connections
    /// remain for it. Called when the reader leaves the watch page, so an
    /// episode does not keep downloading after it has been closed.
    ///
    /// The files stay on disk and act as a replay cache: adding the same
    /// magnet again reuses them (`playback_options` sets `overwrite: true`),
    /// so a second watch resumes from where the first left off -- or starts
    /// instantly if the file was already complete. Deleting here is what made
    /// every replay re-fetch from zero.
    ///
    /// An unknown id is not an error: librqbit reports a missing torrent as
    /// `Ok`, so a double removal is harmless.
    pub async fn remove_torrent(&self, id: usize) -> Result<()> {
        // Read the info hash BEFORE deleting: after `delete` the handle is
        // gone. Forgetting the pool entry is what releases a RAM torrent's
        // pieces immediately instead of leaving them to age out of the LRU.
        let info_hash = self.session.get(id.into()).map(|h| h.info_hash());

        self.session
            .delete(id.into(), false)
            .await
            .with_context(|| format!("failed to remove torrent {id}"))?;

        if let (Some(pool), Some(hash)) = (&self.ram, info_hash) {
            pool.forget(hash);
        }

        Ok(())
    }

    /// Pause a torrent's transfer, keeping its partial data.
    ///
    /// Used by the buffering panel's pause button: the reader stops the
    /// download without giving up the pieces already fetched, so resuming
    /// continues from where it left off. An unknown id is not an error -- the
    /// torrent may already have been removed, and a removed torrent is not a
    /// failure to pause.
    pub async fn pause_torrent(&self, id: usize) -> Result<()> {
        let Some(handle) = self.session.get(id.into()) else {
            return Ok(());
        };

        self.session
            .pause(&handle)
            .await
            .with_context(|| format!("failed to pause torrent {id}"))
    }

    /// Resume a paused torrent's transfer.
    ///
    /// The counterpart of [`Self::pause_torrent`]. An unknown id is a no-op for
    /// the same reason; a torrent that is not paused is left as it is.
    pub async fn resume_torrent(&self, id: usize) -> Result<()> {
        let Some(handle) = self.session.get(id.into()) else {
            return Ok(());
        };

        self.session
            .unpause(&handle)
            .await
            .with_context(|| format!("failed to resume torrent {id}"))
    }

    /// Restrict a torrent's download to a single file index.
    ///
    /// librqbit otherwise downloads every file from the start in order, so a
    /// season pack spends its time on episode 1 while the reader waits on
    /// episode 21 and sees no progress. Picking a file calls this so only that
    /// file's pieces are fetched.
    ///
    /// An unknown id is a no-op, like the other per-torrent calls.
    ///
    /// WAITS for the torrent to leave `Initializing` first, then RETRIES on
    /// failure. librqbit rejects `update_only_files` while it is still hashing
    /// existing data, and that window is open for the whole time metadata is
    /// resolving -- which, for a magnet, is exactly when the watch page calls
    /// this. Without the wait the call failed silently and librqbit kept
    /// downloading files in ITS order, so the episode being watched might sit
    /// at 0% while the torrent as a whole passed 20% (and every other file in
    /// a batch release) -- the reader saw progress and no playback.
    ///
    /// The selection is READ BACK before returning, so a caller that awaits
    /// this knows the restriction is really in force rather than assuming the
    /// engine honoured it.
    pub async fn set_only_files(&self, id: usize, only: &[usize]) -> Result<()> {
        // Transitions are logged as they happen (not once at the end) because
        // the interesting case is a torrent that sits in `initializing` for
        // seconds: the timestamp of the FIRST attempt and of the state change
        // is what says whether the wait is the cost or the call itself.
        let started = tokio::time::Instant::now();
        let deadline = started + ONLY_FILES_TIMEOUT;
        let wanted: std::collections::HashSet<usize> = only.iter().copied().collect();

        // The last state name logged, so a 30s wait prints one line per
        // transition instead of ~300 identical ones at a 100ms poll.
        let mut last_state = String::new();
        let mut attempts: u32 = 0;

        loop {
            let Some(handle) = self.session.get(id.into()) else {
                // The torrent was removed while we waited; nothing to restrict
                // and nothing to report. Matches the other no-op cases.
                tracing::info!(
                    "set_only_files: torrent {id} vanished after {}ms, nothing to restrict",
                    started.elapsed().as_millis()
                );
                return Ok(());
            };

            let state = handle.stats().state;
            let state_name = match &state {
                librqbit::TorrentStatsState::Initializing { .. } => "initializing",
                librqbit::TorrentStatsState::Live => "live",
                librqbit::TorrentStatsState::Paused => "paused",
                librqbit::TorrentStatsState::Error => "error",
            };
            if state_name != last_state {
                tracing::info!(
                    "set_only_files: torrent {id} is {state_name} after {}ms",
                    started.elapsed().as_millis()
                );
                last_state = state_name.to_string();
            }

            // Only try while the engine will accept it. An errored torrent is
            // left alone too: retrying against a dead handle cannot succeed
            // and the caller reports the error through the stats.
            let ready = accepts_file_selection(&state);

            if ready {
                attempts += 1;
                let attempt_at = tokio::time::Instant::now();
                let result = self
                    .session
                    .update_only_files(&handle, &wanted)
                    .await
                    .with_context(|| format!("failed to restrict torrent {id} to {only:?}"));

                match result {
                    Ok(()) => {
                        // Verify: a re-added torrent keeps its ORIGINAL
                        // options, so a silently-ignored call would otherwise
                        // look like success. `only_files()` reports faithfully
                        // (asserted by `probe_session_conflict`), and it
                        // returns a `Vec` built from a set, so the comparison
                        // is by membership.
                        let applied = handle.only_files();
                        if applied
                            .as_ref()
                            .is_some_and(|applied| selection_matches(&wanted, applied))
                        {
                            tracing::info!(
                                "set_only_files: torrent {id} VERIFIED only_files={:?} \
                                 on attempt {attempts} after {}ms",
                                applied,
                                started.elapsed().as_millis()
                            );
                            return Ok(());
                        }
                        tracing::warn!(
                            "set_only_files: torrent {id} call succeeded but the selection \
                             did not stick (reads back {applied:?}), retrying"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            "set_only_files: torrent {id} attempt {attempts} failed after \
                             {}ms: {err:#}",
                            attempt_at.elapsed().as_millis()
                        );
                        if tokio::time::Instant::now() >= deadline {
                            return Err(err);
                        }
                    }
                }
            }

            if tokio::time::Instant::now() >= deadline {
                // Out of time: report it rather than pretending the selection
                // took effect. Playback still works, just with the engine's
                // own file order, so the caller treats this as a warning.
                return Err(anyhow!(
                    "torrent {id} did not accept a file selection within {}s",
                    ONLY_FILES_TIMEOUT.as_secs()
                ));
            }

            tokio::time::sleep(ONLY_FILES_POLL).await;
        }
    }

    /// Stop the session and all managed tasks.
    pub async fn stop(&self) {
        self.session.stop().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `TorrentStats` with no live section.
    ///
    /// librqbit does not re-export `LiveStats`, so a unit test cannot populate
    /// `live`; the live fields (speed, peers, ETA) are therefore exercised by
    /// the ignored integration test against a real session, and the defaults
    /// are pinned here.
    fn stats(
        state: librqbit::TorrentStatsState,
        progress_bytes: u64,
        total_bytes: u64,
    ) -> librqbit::TorrentStats {
        librqbit::TorrentStats {
            state,
            file_progress: vec![],
            error: None,
            progress_bytes,
            uploaded_bytes: 0,
            total_bytes,
            finished: false,
            live: None,
        }
    }

    /// A running torrent accepts a file selection; that is what the wait loop
    /// is waiting FOR. Getting this wrong in the permissive direction would
    /// make the loop fire the call during `Initializing`, where librqbit
    /// rejects it -- the bug this whole path exists to fix.
    #[test]
    fn only_a_live_or_paused_torrent_accepts_a_file_selection() {
        use librqbit::TorrentStatsState;

        assert!(accepts_file_selection(&TorrentStatsState::Live));
        assert!(accepts_file_selection(&TorrentStatsState::Paused));
        assert!(!accepts_file_selection(&TorrentStatsState::Initializing {
            paused: false
        }));
        assert!(!accepts_file_selection(&TorrentStatsState::Initializing {
            paused: true
        }));
        assert!(!accepts_file_selection(&TorrentStatsState::Error));
    }

    /// `only_files()` is built from a set, so its ORDER is arbitrary. The
    /// verification must not depend on it, or a correct selection would look
    /// wrong and be retried until the deadline.
    #[test]
    fn selection_matches_ignores_order() {
        let wanted: std::collections::HashSet<usize> = [3, 7].into_iter().collect();

        assert!(selection_matches(&wanted, &[3, 7]));
        assert!(selection_matches(&wanted, &[7, 3]));
        assert!(!selection_matches(&wanted, &[3]));
        assert!(!selection_matches(&wanted, &[3, 7, 9]));
        assert!(!selection_matches(&wanted, &[3, 8]));
        assert!(selection_matches(&std::collections::HashSet::new(), &[]));
    }

    #[test]
    fn summarize_maps_the_live_state_name() {
        let out = summarize(&stats(librqbit::TorrentStatsState::Live, 0, 100));
        assert_eq!(out.state, "live");
    }

    #[test]
    fn summarize_maps_a_paused_state() {
        let out = summarize(&stats(librqbit::TorrentStatsState::Paused, 0, 100));
        assert_eq!(out.state, "paused");
    }

    #[test]
    fn summarize_maps_an_initializing_state_regardless_of_paused_flag() {
        let out = summarize(&stats(
            librqbit::TorrentStatsState::Initializing { paused: true },
            0,
            100,
        ));
        assert_eq!(out.state, "initializing");
    }

    #[test]
    fn summarize_maps_an_error_state() {
        let out = summarize(&stats(librqbit::TorrentStatsState::Error, 0, 100));
        assert_eq!(out.state, "error");
    }

    #[test]
    fn summarize_copies_the_byte_counts_and_progress() {
        let mut s = stats(librqbit::TorrentStatsState::Live, 30, 100);
        s.file_progress = vec![10, 20];

        let out = summarize(&s);

        assert_eq!(out.progress_bytes, 30);
        assert_eq!(out.total_bytes, 100);
        assert_eq!(out.file_progress, vec![10, 20]);
    }

    #[test]
    fn summarize_carries_the_error_message() {
        let mut s = stats(librqbit::TorrentStatsState::Error, 0, 100);
        s.error = Some("tracker unreachable".into());

        assert_eq!(summarize(&s).error.as_deref(), Some("tracker unreachable"));
    }

    /// With no live section every live-only field must read as zero/None, so
    /// the frontend can render a snapshot without special-casing a missing
    /// section.
    #[test]
    fn summarize_defaults_live_fields_when_not_live() {
        let out = summarize(&stats(librqbit::TorrentStatsState::Paused, 0, 100));

        assert_eq!(out.download_mbps, 0.0);
        assert_eq!(out.upload_mbps, 0.0);
        assert_eq!(out.eta_seconds, None);
        assert_eq!(out.peers_live, 0);
        assert_eq!(out.peers_connecting, 0);
        assert_eq!(out.peers_queued, 0);
        assert_eq!(out.peers_seen, 0);
    }

    #[test]
    fn summarize_reports_finished() {
        let mut s = stats(librqbit::TorrentStatsState::Live, 100, 100);
        s.finished = true;

        assert!(summarize(&s).finished);
    }

    #[test]
    fn playback_options_enable_overwrite() {
        assert!(playback_options(None).overwrite);
    }

    #[test]
    fn download_options_pin_every_download_choice() {
        let opts = download_options("/tmp/hash".into(), vec![2, 5], true);
        assert!(opts.overwrite);
        assert!(opts.paused);
        assert!(!opts.list_only);
        assert_eq!(opts.output_folder.as_deref(), Some("/tmp/hash"));
        assert_eq!(opts.only_files, Some(vec![2, 5]));
        assert!(opts.only_files_regex.is_none());
    }

    #[test]
    fn playback_options_are_not_list_only() {
        assert!(!playback_options(None).list_only);
    }

    #[test]
    fn playback_options_carry_the_output_folder() {
        let opts = playback_options(Some("/tmp/kitsune".into()));
        assert_eq!(opts.output_folder.as_deref(), Some("/tmp/kitsune"));
    }

    #[test]
    fn playback_options_leave_output_folder_unset_when_none() {
        assert!(playback_options(None).output_folder.is_none());
    }

    #[test]
    fn ram_playback_options_deselect_every_file_and_attach_the_factory() {
        let pool = std::sync::Arc::new(super::super::ram_storage::RamPool::new(
            RamBufferConfig::default_buffer(),
        ));
        let opts = ram_playback_options(super::super::ram_storage::RamStorageFactory::new(pool));

        // The WHOLE point: with nothing selected, librqbit's download queue
        // is empty and only the streaming window pulls pieces. If this
        // regressed to `None`, the whole file would bulk-download into RAM.
        assert_eq!(opts.only_files, Some(Vec::new()));
        assert!(opts.storage_factory.is_some());
        assert!(opts.overwrite, "replay must resume on top of live state");
        assert!(!opts.list_only);
    }

    #[test]
    fn disk_mode_playback_options_have_no_storage_factory_or_selection() {
        let opts = playback_options(None);
        assert!(opts.storage_factory.is_none());
        assert!(opts.only_files.is_none(), "disk playback keeps the historical whole-torrent selection");
    }

    #[test]
    fn engine_config_defaults_to_disk_mode_with_dht() {
        let config = EngineConfig::new("/tmp/x");
        assert!(config.ram.is_none());
        assert!(!config.disable_dht);
        assert!(config.initial_peers.is_empty());
        assert!(config.listen_port.is_none());

        // librqbit's defaults join the DHT and bind an ephemeral port; the
        // mapping must not have changed them.
        let opts = config.session_options();
        assert!(opts.dht.is_some());
        assert!(opts.listen.is_none());
        // librqbit's default session writes no state itself (fastresume off,
        // persistence off) -- disk writes are pieces only.
        assert!(!opts.fastresume);
        assert!(opts.persistence.is_none());
    }

    #[test]
    fn engine_config_maps_the_test_and_lan_knobs() {
        let peers = vec!["127.0.0.1:4240".parse().unwrap()];
        let config = EngineConfig::new("/tmp/x")
            .without_dht()
            .with_listen_port(4242)
            .with_initial_peers(peers.clone())
            .with_ram(RamBufferConfig::from_mib(64));

        assert!(config.disable_dht);
        assert_eq!(config.listen_port, Some(4242));
        assert_eq!(config.initial_peers, peers);
        assert_eq!(config.ram.as_ref().map(|r| r.buffer_bytes), Some(64 * 1024 * 1024));

        let opts = config.session_options();
        assert!(opts.dht.is_none(), "disable_dht must clear the DHT config");
        let listen = opts.listen.expect("listen_port must map to a listener");
        assert_eq!(listen.listen_addr.port(), 4242);
    }

    #[test]
    fn engine_config_keeps_the_download_dir() {
        let config = EngineConfig::new("/tmp/kitsune-dl");
        assert_eq!(config.download_dir, PathBuf::from("/tmp/kitsune-dl"));
    }

    #[test]
    fn ensure_download_dir_creates_nested_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a").join("b").join("c");

        ensure_download_dir(&nested).expect("should create nested dirs");

        assert!(nested.is_dir());
    }

    #[test]
    fn ensure_download_dir_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();

        ensure_download_dir(tmp.path()).expect("first call");
        ensure_download_dir(tmp.path()).expect("second call");

        assert!(tmp.path().is_dir());
    }

    /// The probe is only informative; it must not invent a number for a path
    /// it cannot place on a known volume.
    #[test]
    fn available_space_reads_a_real_volume() {
        let tmp = tempfile::tempdir().unwrap();
        let free = available_space(tmp.path());

        // Some CI containers report no disks at all; that is a `None`, not a
        // failure, so only assert the positive case when the probe works.
        if let Some(bytes) = free {
            assert!(bytes > 0, "a mounted writable volume has some free space");
        }
    }

    /// The regression behind the first low-disk exercise: sysinfo filters
    /// tmpfs unless the `linux-tmpfs` feature is on, the probe then reported
    /// `None` for scripts/dev/low-disk.sh's mount, `fits_on_disk` degraded to
    /// "assume there is room", and a full RAM disk was discovered by ENOSPC
    /// mid-download instead of refused up front. Run against a real mount:
    /// `scripts/dev/low-disk.sh --no-run` sets KITSUNE_LOWDISK_ROOT, then
    /// `cargo test -p kitsune-core --lib tmpfs -- --ignored`.
    #[test]
    #[ignore = "needs scripts/dev/low-disk.sh's tmpfs mount (KITSUNE_LOWDISK_ROOT)"]
    fn tmpfs_mounts_are_visible_to_the_probe() {
        let root = std::env::var("KITSUNE_LOWDISK_ROOT").unwrap_or_else(|_| "/tmp/kitsune-lowdisk".into());
        let downloads = Path::new(&root).join("downloads");
        assert!(
            downloads.is_dir(),
            "run scripts/dev/low-disk.sh --no-run first; {} is not set up",
            downloads.display()
        );
        let free = available_space(&downloads);
        assert!(
            free.is_some(),
            "the probe must see the tmpfs at {}; linux-tmpfs feature lost?",
            downloads.display()
        );
        assert!(free.unwrap() > 0);
    }

    /// The `KITSUNE_FAKE_AVAILABLE_BYTES` override clamps the probe (it can
    /// only make a volume look LESS full) and stands in for one the probe
    /// cannot see, so scripts/dev/low-disk.sh's no-root mode can exercise the
    /// refusal path on a healthy disk. The value is read once per process, so
    /// this test pins parse-and-clamp rather than the env read itself.
    #[test]
    fn fake_available_clamps_the_probe() {
        assert_eq!(
            available_space_floor(Some(1_000_000), Some(500_000)),
            Some(500_000),
            "a fake smaller than reality clamps the report"
        );
        assert_eq!(
            available_space_floor(Some(400_000), Some(500_000)),
            Some(400_000),
            "a fake larger than reality changes nothing"
        );
        assert_eq!(
            available_space_floor(Some(u64::MAX), Some(2_000_000)),
            Some(2_000_000),
            "a probe that cannot see the volume takes the fake"
        );
        assert_eq!(
            available_space_floor(Some(900_000), None),
            Some(900_000),
            "no override means the real number"
        );
    }

    /// The floor is what the caller relies on, so pin both directions and the
    /// unknown-space default in one place.
    #[test]
    fn fits_on_disk_honours_the_reserve() {
        let tmp = tempfile::tempdir().unwrap();
        let Some(free) = available_space(tmp.path()) else {
            // No probe on this platform: the only meaningful assertion is the
            // documented "assume it fits" default.
            assert!(fits_on_disk(tmp.path(), u64::MAX, 0));
            return;
        };

        // The exact-boundary assertions below re-probe the volume inside
        // `fits_on_disk`, and the rest of the suite creates and deletes
        // tempdirs on the same volume concurrently -- the free bytes can move
        // between the snapshot above and the re-probe, turning "exactly free"
        // into a spurious failure. Probe again: only assert exact boundaries
        // when the volume is quiet enough for two probes to agree. The
        // race-immune extremes always hold regardless.
        assert!(fits_on_disk(tmp.path(), 0, 0));
        assert!(!fits_on_disk(tmp.path(), u64::MAX, 0));
        assert!(!fits_on_disk(tmp.path(), 0, u64::MAX));
        if available_space(tmp.path()) != Some(free) {
            return;
        }

        // Exactly the free space still fits with no reserve...
        assert!(fits_on_disk(tmp.path(), free, 0));
        // ...but one byte more does not.
        let too_much = free.saturating_add(1);
        assert!(
            !fits_on_disk(tmp.path(), too_much, 0),
            "needing more than the free space cannot fit"
        );
        assert!(
            !fits_on_disk(tmp.path(), 0, too_much),
            "a reserve larger than the free space cannot fit"
        );
    }

    /// Adds a torrent and removes it again, exercising the removal against a
    /// live session. Ignored for the same reason as the test below: it binds
    /// sockets and joins the DHT.
    #[tokio::test]
    #[ignore = "binds sockets and joins the DHT; run explicitly"]
    async fn torrent_can_be_added_and_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = TorrentEngine::start(EngineConfig::new(tmp.path()))
            .await
            .expect("engine should start");

        // A well-formed hash with no swarm behind it: enough to register the
        // torrent, so the removal has something real to act on.
        let id = engine
            .add_magnet("magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862")
            .await
            .expect("a well-formed magnet should be accepted");

        engine
            .remove_torrent(id)
            .await
            .expect("removing an added torrent should succeed");

        assert!(
            engine.torrent_files(id).is_empty(),
            "a removed torrent should report no files"
        );

        engine.stop().await;
    }

    /// Adds a torrent, pauses and resumes it, then removes it. Ignored for the
    /// same reason as the test below: it binds sockets and joins the DHT.
    #[tokio::test]
    #[ignore = "binds sockets and joins the DHT; run explicitly"]
    async fn torrent_can_be_paused_and_resumed() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = TorrentEngine::start(EngineConfig::new(tmp.path()))
            .await
            .expect("engine should start");

        // A well-formed hash with no swarm behind it: enough to register the
        // torrent so pause/resume have something real to act on.
        let id = engine
            .add_magnet("magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862")
            .await
            .expect("a well-formed magnet should be accepted");

        engine
            .pause_torrent(id)
            .await
            .expect("pausing an added torrent should succeed");
        engine
            .resume_torrent(id)
            .await
            .expect("resuming a paused torrent should succeed");

        // An unknown id is a no-op, not an error: the torrent may already be
        // gone, and that is not a failure to pause or resume.
        engine
            .pause_torrent(999_999)
            .await
            .expect("pausing an unknown torrent should be a no-op");
        engine
            .resume_torrent(999_999)
            .await
            .expect("resuming an unknown torrent should be a no-op");

        engine.remove_torrent(id).await.expect("cleanup");
        engine.stop().await;
    }

    /// Starts a real session, which binds sockets and joins the DHT.
    /// Ignored by default so the suite stays hermetic and offline-safe.
    /// Run with: `cargo test -- --ignored`
    #[tokio::test]
    #[ignore = "binds sockets and joins the DHT; run explicitly"]
    async fn session_starts_and_stops() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = TorrentEngine::start(EngineConfig::new(tmp.path()))
            .await
            .expect("engine should start");

        assert_eq!(engine.download_dir(), tmp.path());
        engine.stop().await;
    }
}
