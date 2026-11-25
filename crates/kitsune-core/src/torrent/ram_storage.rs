//! In-memory piece storage: stream from RAM, never write to disk.
//!
//! This is the Rust counterpart of the RAM streaming the Node prototype
//! (`ramtorrent-stream`) proved out: pieces live in a bounded LRU pool, the
//! buffer never touches the filesystem, and data the reader has moved past is
//! evicted first so memory stays at the configured cap no matter how large
//! the release is.
//!
//! # How it plugs into librqbit
//!
//! librqbit owns the whole torrent engine and normally writes pieces to
//! files. Storage is pluggable through [`librqbit::storage::StorageFactory`]:
//! a factory creates one [`TorrentStorage`] per torrent, and librqbit calls
//! `pwrite_all`/`pread_exact` on file-relative offsets. This module maps
//! those file offsets back to torrent pieces and stores whole pieces in a
//! process-wide [`RamPool`].
//!
//! One [`RamPool`] backs every RAM torrent of a session, so the cap (256 MiB
//! by default) applies to the whole app, not per torrent.
//!
//! # Eviction and re-download
//!
//! Eviction is tiered rather than plain LRU, because a single LRU order
//! cannot protect both ends of the reader's working set at once: mpv keeps a
//! ~64 MB demuxer cache and re-reads pieces up to a minute behind the read
//! head, while librqbit's stream window keeps writing 32 MiB of fresh
//! look-ahead. The tiers, in eviction order:
//!
//! 1. never-read pieces that have sat unread past [`UNREAD_STALE`] --
//!    abandoned preload data (e.g. a hidden player that jumped to a distant
//!    position and stopped reading);
//! 2. read pieces whose last read is older than [`READ_GRACE`] -- the
//!    played-through region, oldest read first;
//! 3. never-read pieces that are still fresh -- the look-ahead window;
//! 4. read pieces still inside the grace window, as an absolute last resort
//!    to keep the cap invariant.
//!
//! Tiers 3-4 preserve forward progress; tiers 1-2 sacrifice data the reader
//! has visibly moved away from. If a reader nevertheless touches an evicted
//! piece, the read fails with a "not resident" error and the read is counted
//! (`ghost_reads`); the vendored librqbit then clears the stale have-bit,
//! re-queues the piece and waits for the re-download instead of killing the
//! stream, so a seek back into dropped data costs a buffer rather than the
//! whole episode.
//!
//! # Disk hygiene
//!
//! A RAM torrent writes NOTHING: no pieces, no fastresume, no session
//! state. `ensure_file_length`, `remove_file` and friends are no-ops, and
//! the engine pairs this storage with `fastresume: false` and disabled
//! session persistence so librqbit itself never serialises anything either.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use librqbit::storage::{BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage};
use librqbit::{FileInfos, ManagedTorrentShared, TorrentMetadata};
use librqbit_core::hash_id::Id20;
use librqbit_core::lengths::{Lengths, ValidPieceIndex};
use parking_lot::{Mutex, MutexGuard};
use serde::Serialize;

/// Default streaming buffer cap: 256 MiB, matching the prototype.
pub const DEFAULT_BUFFER_BYTES: u64 = 256 * 1024 * 1024;

/// Settings for the RAM streaming buffer.
///
/// `buffer_bytes` is the hard cap on the pool (all RAM torrents combined);
/// it is enforced synchronously on every write. The floor exists because a
/// single piece must always fit -- piece lengths of 1-16 MiB are typical, so
/// anything below would thrash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RamBufferConfig {
    /// Hard cap on bytes the pool may hold, across all RAM torrents.
    pub buffer_bytes: u64,
    /// How long a never-read piece may sit before eviction treats it as
    /// abandoned preload data. Defaults to [`UNREAD_STALE`].
    pub unread_stale: Duration,
    /// How long a read piece stays eviction-protected after its last read.
    /// Defaults to [`READ_GRACE`]; zero disables the protection.
    pub read_grace: Duration,
}

/// The tier windows every config gets unless overridden.
const fn default_tuning() -> (Duration, Duration) {
    (UNREAD_STALE, READ_GRACE)
}

impl RamBufferConfig {
    /// The prototype default: 256 MiB.
    pub fn default_buffer() -> Self {
        let (unread_stale, read_grace) = default_tuning();
        Self {
            buffer_bytes: DEFAULT_BUFFER_BYTES,
            unread_stale,
            read_grace,
        }
    }

    /// The config for `mib`, clamped to a usable range.
    ///
    /// The floor is 48 MiB: librqbit's streaming window is a hard-coded
    /// 32 MiB per open reader, and a buffer smaller than one window cannot
    /// hold the look-ahead it downloads -- unread pieces would be evicted
    /// ahead of the reader and re-fetched endlessly. The ceiling is 4 GiB
    /// (more than any reader needs and a plausible typo for a smaller
    /// value).
    pub fn from_mib(mib: u32) -> Self {
        let mib = mib.clamp(48, 4096);
        let (unread_stale, read_grace) = default_tuning();
        Self {
            buffer_bytes: u64::from(mib) * 1024 * 1024,
            unread_stale,
            read_grace,
        }
    }
}

/// Everything the pool knows about one torrent's RAM storage.
struct TorrentEntry {
    /// Pieces EVICTED after having been resident. A pread miss for one of
    /// these is a real "reader went back into dropped data" event; a miss
    /// for anything else is a piece that simply was not downloaded yet
    /// (normal during the initial hash check of a fresh torrent). Entries
    /// are removed when the piece is written again.
    ghosts: BTreeSet<u32>,
    ghost_reads: u64,
}

/// A whole piece held in RAM.
#[derive(Clone)]
struct PieceEntry {
    data: Vec<u8>,
    /// Logical clock stamp of the last write/read/completion. Orders pieces
    /// within an eviction tier; pools hold a few hundred pieces at most
    /// (256 MiB of 1-4 MiB pieces), so the O(n) scan is noise.
    last_use: u64,
    /// Wall clock of the most recent write. Pieces that were written but
    /// never read are look-ahead or idle-reader preloads; how long they have
    /// sat unread decides how expendable they are (see `make_room`).
    written_at: Instant,
    /// Wall clock of the most recent read, if the piece was ever read.
    read_at: Option<Instant>,
}

/// How long a never-read piece may sit before it is treated as abandoned
/// preload data and evicted ahead of everything else. A stream look-ahead
/// (32 MiB window, consumed at reading speed) is read well inside this; a
/// parked hidden player preloading a distant region is not.
const UNREAD_STALE: Duration = Duration::from_secs(60);

/// How long a read piece stays eviction-protected after its last read.
///
/// mpv keeps a demuxer cache (~64 MB in practice) and re-reads pieces it has
/// already streamed when its own cache no longer covers them -- up to about a
/// minute behind the read head. Evicting anything read more recently than
/// this is what produced the mid-episode black screen (ghost reads on the
/// piece the reader was still consuming); this grace keeps such pieces
/// resident, sized so that mpv's re-reads hit live data.
const READ_GRACE: Duration = Duration::from_secs(75);

/// A snapshot of pool counters, for progress UI and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamPoolStats {
    /// Configured cap.
    pub limit_bytes: u64,
    /// Bytes currently held.
    pub stored_bytes: u64,
    /// Highest `stored_bytes` ever reached.
    pub peak_bytes: u64,
    /// Pieces currently held.
    pub pieces: usize,
    /// Pieces evicted so far.
    pub evictions: u64,
    /// Reads that hit an evicted piece librqbit still believed it had.
    pub ghost_reads: u64,
    /// Torrents currently backed by the pool.
    pub torrents: usize,
}

/// The shared, bounded LRU pool behind every RAM torrent.
pub struct RamPool {
    limit_bytes: u64,
    /// Guards all mutable state. Contention is a non-issue: librqbit calls
    /// the storage from its blocking pool, a handful of times per piece.
    inner: Mutex<PoolInner>,
    /// Monotonic clock for LRU stamps, outside the lock (cheap atomic).
    clock: AtomicU64,
    /// Eviction tiers, mirroring [`UNREAD_STALE`] and [`READ_GRACE`], in
    /// milliseconds. Atomics rather than plain fields because the pool lives
    /// behind an `Arc`; tests shrink the windows instead of sleeping minutes.
    unread_stale_ms: AtomicU64,
    read_grace_ms: AtomicU64,
}

struct PoolInner {
    stored_bytes: u64,
    peak_bytes: u64,
    evictions: u64,
    /// Pieces keyed by `(torrent key, piece index)`.
    pieces: HashMap<(u64, u32), PieceEntry>,
    /// Per-torrent bookkeeping, keyed by the same torrent key.
    torrents: HashMap<u64, TorrentEntry>,
    /// info hash -> torrent key, so the engine can ask "is this torrent RAM
    /// backed?" and keep disk downloads away from RAM torrents.
    by_info_hash: HashMap<[u8; 20], u64>,
}

impl RamPool {
    /// A pool capped at `config.buffer_bytes`.
    pub fn new(config: RamBufferConfig) -> Self {
        Self {
            limit_bytes: config.buffer_bytes,
            inner: Mutex::new(PoolInner {
                stored_bytes: 0,
                peak_bytes: 0,
                evictions: 0,
                pieces: HashMap::new(),
                torrents: HashMap::new(),
                by_info_hash: HashMap::new(),
            }),
            clock: AtomicU64::new(1),
            unread_stale_ms: AtomicU64::new(config.unread_stale.as_millis() as u64),
            read_grace_ms: AtomicU64::new(config.read_grace.as_millis() as u64),
        }
    }

    /// The configured cap.
    pub fn limit_bytes(&self) -> u64 {
        self.limit_bytes
    }

    /// A counter snapshot.
    pub fn stats(&self) -> RamPoolStats {
        let g = self.lock();
        RamPoolStats {
            limit_bytes: self.limit_bytes,
            stored_bytes: g.stored_bytes,
            peak_bytes: g.peak_bytes,
            pieces: g.pieces.len(),
            evictions: g.evictions,
            ghost_reads: g.torrents.values().map(|t| t.ghost_reads).sum(),
            torrents: g.torrents.len(),
        }
    }

    /// Whether `info_hash` is currently backed by RAM storage.
    ///
    /// The download path uses this to refuse mixing modes: a torrent that is
    /// streaming from RAM must not be re-added as a disk download, because
    /// librqbit would keep the existing RAM storage and the "downloaded"
    /// file would not exist on disk.
    pub fn is_ram_backed(&self, info_hash: Id20) -> bool {
        self.lock().by_info_hash.contains_key(&info_hash.0)
    }

    /// Piece indices that were read while not resident ("ghosts"), for
    /// diagnostics. Most useful right after a playback hiccup: the indices
    /// say WHERE the reader sought back into an evicted region.
    pub fn ghost_pieces(&self, info_hash: Id20) -> Vec<u32> {
        let g = self.lock();
        g.by_info_hash
            .get(&info_hash.0)
            .and_then(|key| g.torrents.get(key))
            .map(|t| t.ghosts.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Drop every piece of one torrent from the pool.
    ///
    /// Called when the engine removes a RAM torrent, so an episode that is
    /// done streaming stops occupying the buffer immediately instead of
    /// waiting to age out of the LRU.
    pub fn forget(&self, info_hash: Id20) {
        let mut g = self.lock();
        if let Some(key) = g.by_info_hash.remove(&info_hash.0) {
            g.torrents.remove(&key);
            let doomed: Vec<(u64, u32)> = g
                .pieces
                .keys()
                .filter(|(k, _)| *k == key)
                .copied()
                .collect();
            for k in doomed {
                if let Some(entry) = g.pieces.remove(&k) {
                    g.stored_bytes = g.stored_bytes.saturating_sub(entry.data.len() as u64);
                }
            }
        }
    }

    fn lock(&self) -> MutexGuard<'_, PoolInner> {
        self.inner.lock()
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }
}

/// The per-torrent storage librqbit talks to.
///
/// All methods take `&self` (the trait's shape); mutability lives in the
/// shared [`RamPool`].
pub struct RamStorage {
    pool: Arc<RamPool>,
    /// Unique key for this torrent within the pool.
    torrent_key: u64,
    lengths: Lengths,
    file_infos: FileInfos,
}

impl RamStorage {
    fn file_offset(&self, file_id: usize) -> Result<u64> {
        let fi = self
            .file_infos
            .get(file_id)
            .context("storage got an unknown file id")?;
        Ok(fi.offset_in_torrent)
    }

    /// Real length of `piece` in bytes (the final piece is short).
    fn piece_len_of(&self, piece: u32) -> Result<usize> {
        let idx = self
            .lengths
            .validate_piece_index(piece)
            .with_context(|| format!("piece {piece} is out of range"))?;
        Ok(self.lengths.piece_length(idx) as usize)
    }

    /// Split a `len`-byte transfer at file-relative `offset` into
    /// per-piece spans: `(piece, offset within piece, offset into buf,
    /// span length)`.
    ///
    /// A single call may cross a piece boundary (the final piece is short,
    /// and callers slice by file, not by piece), so every read and write
    /// walks spans instead of assuming one piece.
    fn spans(&self, file_id: usize, offset: u64, len: usize) -> Result<Vec<(u32, u64, usize, usize)>> {
        let file_off = self.file_offset(file_id)?;
        let dpl = u64::from(self.lengths.default_piece_length());
        if dpl == 0 {
            return Err(anyhow!("piece length is zero"));
        }

        let abs_start = file_off
            .checked_add(offset)
            .context("offset overflows the torrent")?;
        let abs_end = abs_start
            .checked_add(len as u64)
            .context("read/write overflows the torrent")?;

        let mut out = Vec::new();
        let mut cursor = abs_start;
        while cursor < abs_end {
            let piece = u32::try_from(cursor / dpl).context("piece index does not fit in u32")?;
            let piece_start = cursor - cursor % dpl;
            let piece_end = piece_start + self.piece_len_of(piece)? as u64;
            let span_end = abs_end.min(piece_end);
            out.push((
                piece,
                cursor - piece_start,
                (cursor - abs_start) as usize,
                (span_end - cursor) as usize,
            ));
            cursor = span_end;
        }
        Ok(out)
    }
}

impl TorrentStorage for RamStorage {
    fn init(
        &mut self,
        _shared: &ManagedTorrentShared,
        metadata: &TorrentMetadata,
    ) -> Result<()> {
        // Nothing to open: the pool entry was registered at create() time.
        // Keep the lengths in sync in case metadata re-resolved.
        self.lengths = *metadata.lengths();
        Ok(())
    }

    fn pread_exact(&self, file_id: usize, offset: u64, buf: &mut [u8]) -> Result<()> {
        for (piece, within, at, len) in self.spans(file_id, offset, buf.len())? {
            let mut g = self.pool.lock();
            let Some(entry) = g.pieces.get_mut(&(self.torrent_key, piece)) else {
                // The piece is not resident. Only a piece that WAS resident
                // and got evicted is a "ghost read" (a reader seeking back
                // into dropped data) worth counting; a miss for a piece that
                // never arrived is normal -- the initial hash check reads
                // every piece of an empty buffer, and librqbit's stream poll
                // probes ahead of the download.
                let was_evicted = g
                    .torrents
                    .get_mut(&self.torrent_key)
                    .is_some_and(|t| {
                        let evicted = t.ghosts.contains(&piece);
                        if evicted {
                            // Count EVERY miss: the reader may hit the same
                            // dropped region repeatedly (mpv retries), and
                            // each one is a playback-visible event.
                            t.ghost_reads += 1;
                        }
                        evicted
                    });
                return Err(anyhow!(
                    "RAM buffer: piece {piece} of torrent {} is not resident{}",
                    self.torrent_key,
                    if was_evicted { " (evicted)" } else { " (never downloaded)" }
                ));
            };

            let within_usz = within as usize;
            let end = within_usz + len;
            if end > entry.data.len() {
                return Err(anyhow!(
                    "RAM buffer: read of {len} bytes at {within} exceeds piece {piece} length {}",
                    entry.data.len()
                ));
            }
            buf[at..at + len].copy_from_slice(&entry.data[within_usz..end]);
            entry.last_use = self.pool.tick();
            entry.read_at = Some(Instant::now());
        }
        Ok(())
    }

    fn pwrite_all(&self, file_id: usize, offset: u64, buf: &[u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        for (piece, within, at, len) in self.spans(file_id, offset, buf.len())? {
            let mut g = self.pool.lock();
            let key = (self.torrent_key, piece);

            if !g.pieces.contains_key(&key) {
                // Size to the REAL piece length (the final piece is short)
                // so the piece holds every chunk librqbit sends for it.
                let full_len = self.piece_len_of(piece)?;
                self.make_room(&mut g, full_len, piece);
                g.pieces.insert(
                    key,
                    PieceEntry {
                        data: vec![0u8; full_len],
                        last_use: 0,
                        written_at: Instant::now(),
                        read_at: None,
                    },
                );
                g.stored_bytes += full_len as u64;
                // The piece is resident again; it is no longer a ghost.
                if let Some(t) = g.torrents.get_mut(&self.torrent_key) {
                    t.ghosts.remove(&piece);
                }
            }

            let entry = g
                .pieces
                .get_mut(&key)
                .expect("piece was just inserted");
            let within_usz = within as usize;
            let end = within_usz + len;
            if end > entry.data.len() {
                return Err(anyhow!(
                    "RAM buffer: write of {len} bytes at {within} exceeds piece {piece} length {}",
                    entry.data.len()
                ));
            }
            entry.data[within_usz..end].copy_from_slice(&buf[at..at + len]);
            entry.last_use = self.pool.tick();
            entry.written_at = Instant::now();
        }
        Ok(())
    }

    fn remove_file(&self, _file_id: usize, _filename: &std::path::Path) -> Result<()> {
        // Nothing on disk to remove. Piece data is accounted per piece, not
        // per file, so a file removal (deselection) must not drop pieces a
        // neighbouring file still covers.
        Ok(())
    }

    fn remove_directory_if_empty(&self, _path: &std::path::Path) -> Result<()> {
        Ok(())
    }

    fn ensure_file_length(&self, _file_id: usize, _length: u64) -> Result<()> {
        // Pieces are allocated on first write; there is no file to size.
        Ok(())
    }

    fn take(&self) -> Result<Box<dyn TorrentStorage>> {
        // librqbit calls this when a torrent moves states (Initializing ->
        // Paused on every add, and Live -> Paused on pause) and KEEPS USING
        // THE RETURN VALUE as the torrent's storage. The filesystem backend
        // returns a freshly reopened handle here; the pool equivalent is a
        // fresh storage sharing the same pool and torrent key, so the data
        // stays where it is and the torrent keeps working. The object it is
        // called on is dropped by librqbit.
        Ok(Box::new(RamStorage {
            pool: self.pool.clone(),
            torrent_key: self.torrent_key,
            lengths: self.lengths,
            file_infos: self.file_infos.clone(),
        }))
    }

    fn on_piece_completed(&self, piece: ValidPieceIndex) -> Result<()> {
        // Touch so freshly verified pieces are the last eviction candidates.
        // For a never-read piece this also restarts its unread-stale clock:
        // the look-ahead age that matters starts when the piece is complete,
        // not when its first chunk landed.
        let mut g = self.pool.lock();
        if let Some(entry) = g.pieces.get_mut(&(self.torrent_key, piece.get())) {
            entry.last_use = self.pool.tick();
            if entry.read_at.is_none() {
                entry.written_at = Instant::now();
            }
        }
        Ok(())
    }
}

impl RamStorage {
    /// Evict pieces until `incoming` bytes fit under the cap.
    ///
    /// `avoid` is the piece being written; it is never evicted by its own
    /// allocation. Victims are chosen by tier (see the module docs): abandoned
    /// preload first, then the played-through region, then the fresh
    /// look-ahead, then grace-protected reads as a last resort. The tiers are
    /// wall-clock based, intra-tier order is the logical LRU stamp. If
    /// everything resident is protected (a flooded swarm outrunning the
    /// reader), the oldest piece regardless -- the cap is a hard invariant,
    /// and the alternative (failing the write) would stall the download loop.
    fn make_room(&self, g: &mut MutexGuard<'_, PoolInner>, incoming: usize, avoid: u32) {
        let incoming = incoming as u64;
        let limit = self.pool.limit_bytes;
        // One piece larger than the whole pool is a configuration error the
        // caller cannot recover from; the write would never fit.
        if incoming > limit {
            return;
        }

        let unread_stale = Duration::from_millis(self.pool.unread_stale_ms.load(Ordering::Relaxed));
        let read_grace = Duration::from_millis(self.pool.read_grace_ms.load(Ordering::Relaxed));
        while g.stored_bytes + incoming > limit {
            let now = Instant::now();
            let candidates = g
                .pieces
                .iter()
                .filter(|((_, piece), _)| *piece != avoid);

            // Tier 1: never read, and has sat unread long enough to be
            // abandoned preload rather than a live look-ahead.
            let victim = candidates
                .clone()
                .filter(|(_, e)| {
                    e.read_at.is_none() && now.duration_since(e.written_at) > unread_stale
                })
                .min_by_key(|(_, e)| e.last_use)
                .map(|(k, _)| *k)
                // Tier 2: read pieces the reader has visibly moved past --
                // nothing has read them within the grace window.
                .or_else(|| {
                    candidates
                        .clone()
                        .filter(|(_, e)| {
                            e.read_at.is_some_and(|r| now.duration_since(r) > read_grace)
                        })
                        .min_by_key(|(_, e)| e.last_use)
                        .map(|(k, _)| *k)
                })
                // Tier 3: fresh look-ahead, oldest write first.
                .or_else(|| {
                    candidates
                        .clone()
                        .filter(|(_, e)| e.read_at.is_none())
                        .min_by_key(|(_, e)| e.last_use)
                        .map(|(k, _)| *k)
                })
                // Tier 4: grace-protected reads -- absolute last resort.
                .or_else(|| candidates.min_by_key(|(_, e)| e.last_use).map(|(k, _)| *k));

            let Some(victim) = victim else {
                break;
            };

            if let Some(entry) = g.pieces.remove(&victim) {
                g.stored_bytes = g.stored_bytes.saturating_sub(entry.data.len() as u64);
                g.evictions += 1;
                // Remember the eviction: a later read of this piece is the
                // "seek back into dropped data" case the ghost counter
                // exists to surface.
                if let Some(t) = g.torrents.get_mut(&victim.0) {
                    t.ghosts.insert(victim.1);
                }
            }
        }

        g.peak_bytes = g.peak_bytes.max(g.stored_bytes + incoming);
    }
}

/// Builds one [`RamStorage`] per RAM torrent, all sharing a pool.
#[derive(Clone)]
pub struct RamStorageFactory {
    pool: Arc<RamPool>,
}

impl RamStorageFactory {
    /// A factory whose storages share `pool`.
    pub fn new(pool: Arc<RamPool>) -> Self {
        Self { pool }
    }

    /// The shared pool, for stats and lifetime checks.
    pub fn pool(&self) -> &Arc<RamPool> {
        &self.pool
    }
}

static NEXT_TORRENT_KEY: AtomicU64 = AtomicU64::new(1);

impl StorageFactory for RamStorageFactory {
    type Storage = RamStorage;

    fn create(
        &self,
        shared: &ManagedTorrentShared,
        metadata: &TorrentMetadata,
    ) -> Result<Self::Storage> {
        let torrent_key = NEXT_TORRENT_KEY.fetch_add(1, Ordering::Relaxed);
        let info_hash = shared.info_hash.0;

        {
            let mut g = self.pool.lock();
            g.by_info_hash.insert(info_hash, torrent_key);
            g.torrents.insert(
                torrent_key,
                TorrentEntry {
                    ghosts: BTreeSet::new(),
                    ghost_reads: 0,
                },
            );
        }

        Ok(RamStorage {
            pool: self.pool.clone(),
            torrent_key,
            lengths: *metadata.lengths(),
            file_infos: metadata.file_infos.clone(),
        })
    }

    fn clone_box(&self) -> BoxStorageFactory {
        self.clone().boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librqbit::file_info::FileInfo;
    use librqbit_core::hash_id::Id20;
    use std::path::PathBuf;

    const PIECE: u32 = 16 * 1024;

    /// Build a REAL `RamStorage` over a synthetic torrent of `file_lens`
    /// files laid back to back, backed by a pool capped at `buffer_bytes`.
    fn make_storage(
        file_lens: &[u64],
        piece_len: u32,
        buffer_bytes: u64,
    ) -> (Arc<RamPool>, RamStorage) {
        let total: u64 = file_lens.iter().sum();
        let lengths = Lengths::new(total, piece_len).expect("valid lengths");

        let mut file_infos = Vec::new();
        let mut acc = 0u64;
        for (i, len) in file_lens.iter().enumerate() {
            let start = acc / u64::from(piece_len);
            let end = (acc + len).div_ceil(u64::from(piece_len));
            file_infos.push(FileInfo {
                relative_filename: PathBuf::from(format!("file{i}.bin")),
                offset_in_torrent: acc,
                piece_range: start as u32..end as u32,
                attrs: librqbit_core::torrent_metainfo::FileDetailsAttrs::default(),
                len: *len,
            });
            acc += len;
        }

        let pool = Arc::new(RamPool::new(RamBufferConfig {
            buffer_bytes,
            ..RamBufferConfig::default_buffer()
        }));
        let key = NEXT_TORRENT_KEY.fetch_add(1, Ordering::Relaxed);
        let storage = RamStorage {
            pool: pool.clone(),
            torrent_key: key,
            lengths,
            file_infos,
        };
        (pool, storage)
    }

    /// A deterministic byte pattern for offset `i`, so expected contents
    /// need no storage.
    fn pattern(i: u64) -> u8 {
        (i % 251) as u8
    }

    fn fill(len: usize, base: u64) -> Vec<u8> {
        (0..len).map(|j| pattern(base + j as u64)).collect()
    }

    /// Map a file-relative offset to a torrent piece: the single-piece case
    /// of [`RamStorage::spans`], kept here so the boundary math of the span
    /// walk has a directly assertable form.
    fn locate_piece(
        file_offset_in_torrent: u64,
        offset: u64,
        piece_len: u32,
    ) -> Result<(u32, u64)> {
        if piece_len == 0 {
            return Err(anyhow!("piece length is zero"));
        }
        let abs = file_offset_in_torrent
            .checked_add(offset)
            .context("offset overflows the torrent")?;
        let piece = (abs / u64::from(piece_len))
            .try_into()
            .context("piece index does not fit in u32")?;
        let within = abs % u64::from(piece_len);
        Ok((piece, within))
    }

    #[test]
    fn locate_piece_maps_single_file_offsets() {
        assert_eq!(locate_piece(0, 0, PIECE).unwrap(), (0, 0));
        assert_eq!(locate_piece(0, 1, PIECE).unwrap(), (0, 1));
        assert_eq!(locate_piece(0, 16 * 1024, PIECE).unwrap(), (1, 0));
        assert_eq!(locate_piece(0, 40 * 1024, PIECE).unwrap(), (2, 8 * 1024));
    }

    #[test]
    fn locate_piece_across_files_uses_torrent_absolute_offset() {
        // File 1 starts at 100 KiB: its byte 0 is piece 6, offset 4 KiB.
        assert_eq!(locate_piece(100 * 1024, 0, PIECE).unwrap(), (6, 4 * 1024));
        // File 2 starts at 50 KiB (mid-piece 3, since 48 KiB starts piece 3):
        // its byte 16 lands in piece 3 at 2 KiB + 16.
        assert_eq!(
            locate_piece(50 * 1024, 16, PIECE).unwrap(),
            (3, 2 * 1024 + 16)
        );
    }

    #[test]
    fn locate_piece_rejects_zero_piece_length() {
        assert!(locate_piece(0, 0, 0).is_err());
    }

    #[test]
    fn write_then_read_roundtrips_bytes() {
        let (_pool, storage) = make_storage(&[64 * 1024], PIECE, 1024 * 1024);
        let payload = fill(4096, 0);

        storage.pwrite_all(0, 4096, &payload).unwrap();

        let mut out = vec![0u8; 4096];
        storage.pread_exact(0, 4096, &mut out).unwrap();
        assert_eq!(out, payload);
    }

    #[test]
    fn write_spanning_a_piece_boundary_lands_in_both_pieces() {
        let (_pool, storage) = make_storage(&[64 * 1024], PIECE, 1024 * 1024);

        // 4 bytes straddling the piece 0/1 boundary.
        let payload = fill(4, 1000);
        storage.pwrite_all(0, (PIECE as u64) - 2, &payload).unwrap();

        let mut head = [0u8; 2];
        storage.pread_exact(0, (PIECE as u64) - 2, &mut head).unwrap();
        assert_eq!(&head, &payload[..2]);

        let mut tail = [0u8; 2];
        storage.pread_exact(0, PIECE as u64, &mut tail).unwrap();
        assert_eq!(&tail, &payload[2..]);
    }

    #[test]
    fn reads_and_writes_cross_file_boundaries() {
        // Two files of 32 KiB each = 64 KiB = 4 pieces. A write issued to
        // file 0 near its end spills into file 1's first piece spans.
        let (_pool, storage) = make_storage(&[32 * 1024, 32 * 1024], PIECE, 1024 * 1024);

        let payload = fill(300, 7);
        storage.pwrite_all(0, 32 * 1024 - 100, &payload).unwrap();

        let mut out = vec![0u8; 300];
        storage.pread_exact(0, 32 * 1024 - 100, &mut out).unwrap();
        assert_eq!(out, payload);

        // The same bytes are visible through file 1 at its own offsets.
        let mut out2 = vec![0u8; 200];
        storage.pread_exact(1, 0, &mut out2).unwrap();
        assert_eq!(out2, payload[100..]);
    }

    #[test]
    fn final_piece_is_sized_to_the_real_length() {
        // 40 KiB total with 16 KiB pieces -> final piece is 8 KiB.
        let (pool, storage) = make_storage(&[40 * 1024], PIECE, 1024 * 1024);

        // Write the tail byte of the torrent (file-relative 40 KiB - 1);
        // only the final piece is allocated, at its REAL 8 KiB length.
        storage.pwrite_all(0, 40 * 1024 - 1, &[0xAB]).unwrap();

        let mut out = [0u8; 1];
        storage.pread_exact(0, 40 * 1024 - 1, &mut out).unwrap();
        assert_eq!(out[0], 0xAB);

        assert_eq!(pool.stats().stored_bytes, 8 * 1024);
    }

    #[test]
    fn pool_never_exceeds_its_cap() {
        // Cap fits exactly 3 pieces; write 10.
        let cap = 3 * PIECE as u64;
        let (pool, storage) = make_storage(&[1 << 20], PIECE, cap);

        for piece in 0..10u32 {
            let buf = fill(PIECE as usize, u64::from(piece));
            storage
                .pwrite_all(0, piece as u64 * PIECE as u64, &buf)
                .unwrap();
            let stats = pool.stats();
            assert!(
                stats.stored_bytes <= cap,
                "cap exceeded after writing piece {piece}: {}",
                stats.stored_bytes
            );
        }

        let stats = pool.stats();
        assert_eq!(stats.stored_bytes, cap, "pool should sit exactly at cap");
        assert_eq!(stats.peak_bytes, cap);
        assert!(stats.evictions >= 7, "expected evictions, got {}", stats.evictions);
    }

    #[test]
    fn eviction_is_lru_by_last_use() {
        let cap = 2 * PIECE as u64;
        let (_pool, storage) = make_storage(&[1 << 20], PIECE, cap);

        // Piece 0 (oldest), then piece 1, then re-READ piece 0 (freshens it).
        storage.pwrite_all(0, 0, &fill(PIECE as usize, 1)).unwrap();
        storage
            .pwrite_all(0, PIECE as u64, &fill(PIECE as usize, 2))
            .unwrap();
        let mut scratch = [0u8; 4];
        storage.pread_exact(0, 0, &mut scratch).unwrap();

        // Writing piece 2 must evict piece 1 (LRU), keeping the freshly
        // read piece 0.
        storage
            .pwrite_all(0, 2 * PIECE as u64, &fill(PIECE as usize, 3))
            .unwrap();

        // Piece 0 still readable, piece 1 is now a ghost read.
        let mut probe = [0u8; 4];
        storage.pread_exact(0, 0, &mut probe).unwrap();
        assert!(storage.pread_exact(0, PIECE as u64, &mut probe).is_err());
    }

    #[test]
    fn unread_pieces_evict_in_write_order() {
        // With files deselected the look-ahead is written but never read;
        // the played-through region was read longer ago. LRU by last use
        // must evict the played-through region FIRST (older stamps), which
        // this pins down: writes only, no reads -- oldest write goes first.
        let cap = 2 * PIECE as u64;
        let (_pool, storage) = make_storage(&[1 << 20], PIECE, cap);

        storage.pwrite_all(0, 0, &fill(PIECE as usize, 1)).unwrap();
        storage
            .pwrite_all(0, PIECE as u64, &fill(PIECE as usize, 2))
            .unwrap();
        storage
            .pwrite_all(0, 2 * PIECE as u64, &fill(PIECE as usize, 3))
            .unwrap();

        let mut probe = [0u8; 4];
        assert!(storage.pread_exact(0, 0, &mut probe).is_err(), "oldest write evicted");
        storage.pread_exact(0, PIECE as u64, &mut probe).unwrap();
        storage.pread_exact(0, 2 * PIECE as u64, &mut probe).unwrap();
    }

    #[test]
    fn stale_unread_preload_evicts_before_fresh_lookahead() {
        // The mid-episode black screen scenario: a hidden player parked at a
        // distant position preloads region A (written once, never read),
        // while the active reader's stream window (region B, also unread) is
        // being downloaded. Once region A has sat unread past the stale
        // window it must evict first, even though region B was written
        // EARLIER (older LRU stamp).
        let cap = 2 * PIECE as u64;
        let (pool, storage) = make_storage(&[1 << 20], PIECE, cap);
        pool.unread_stale_ms.store(50, Ordering::Relaxed);

        // Region A: the hidden player's preload (written first).
        storage.pwrite_all(0, 0, &fill(PIECE as usize, 1)).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        // Region B: the active reader's fresh look-ahead.
        storage
            .pwrite_all(0, PIECE as u64, &fill(PIECE as usize, 2))
            .unwrap();

        // Overflow: a third write must evict the abandoned preload (piece 0),
        // keeping the fresh look-ahead (piece 1) the reader is about to
        // consume.
        storage
            .pwrite_all(0, 2 * PIECE as u64, &fill(PIECE as usize, 3))
            .unwrap();

        let mut probe = [0u8; 4];
        assert!(storage.pread_exact(0, 0, &mut probe).is_err(), "stale preload evicted");
        storage.pread_exact(0, PIECE as u64, &mut probe).unwrap();
        storage.pread_exact(0, 2 * PIECE as u64, &mut probe).unwrap();
    }

    #[test]
    fn recently_read_piece_survives_unread_pressure() {
        // The actual crash: mpv re-read a piece it had consumed ~60s earlier,
        // but the pool evicted it for newer unread writes. A read piece
        // within the grace window must be the LAST resort, evicted only after
        // every unread piece is gone.
        let cap = 2 * PIECE as u64;
        let (pool, storage) = make_storage(&[1 << 20], PIECE, cap);
        pool.read_grace_ms.store(75_000, Ordering::Relaxed);

        storage.pwrite_all(0, 0, &fill(PIECE as usize, 1)).unwrap();
        let mut probe = [0u8; 4];
        storage.pread_exact(0, 0, &mut probe).unwrap();

        // Two unread writes overflow the 2-piece cap. Both are unread and
        // fresh: tier 3 must eat them (oldest first) BEFORE the grace-
        // protected read piece.
        storage
            .pwrite_all(0, PIECE as u64, &fill(PIECE as usize, 2))
            .unwrap();
        storage
            .pwrite_all(0, 2 * PIECE as u64, &fill(PIECE as usize, 3))
            .unwrap();
        storage
            .pwrite_all(0, 3 * PIECE as u64, &fill(PIECE as usize, 4))
            .unwrap();

        // The read piece is still resident: mpv's re-read hits live data.
        storage.pread_exact(0, 0, &mut probe).unwrap();
    }

    #[test]
    fn stale_reads_evict_after_stale_preload_but_before_fresh_lookahead() {
        // Once a read piece ages past the grace window it becomes tier 2:
        // evicted after abandoned preload (tier 1) but before the fresh
        // look-ahead (tier 3), because losing the look-ahead means a
        // guaranteed re-download while the stale read is only speculative.
        let cap = 3 * PIECE as u64;
        let (pool, storage) = make_storage(&[1 << 20], PIECE, cap);
        pool.read_grace_ms.store(50, Ordering::Relaxed);
        pool.unread_stale_ms.store(3_600_000, Ordering::Relaxed); // keep tier 1 empty

        // Piece 0: read, then left to age past the grace window.
        storage.pwrite_all(0, 0, &fill(PIECE as usize, 1)).unwrap();
        let mut probe = [0u8; 4];
        storage.pread_exact(0, 0, &mut probe).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        // Pieces 1-2: fresh look-ahead, written after the read.
        storage
            .pwrite_all(0, PIECE as u64, &fill(PIECE as usize, 2))
            .unwrap();
        storage
            .pwrite_all(0, 2 * PIECE as u64, &fill(PIECE as usize, 3))
            .unwrap();

        // Overflow with a fresh unread write: tier 1 is empty, tier 2 (the
        // stale read, piece 0) must go before tier 3 (fresh look-ahead).
        storage
            .pwrite_all(0, 3 * PIECE as u64, &fill(PIECE as usize, 4))
            .unwrap();

        assert!(storage.pread_exact(0, 0, &mut probe).is_err(), "stale read evicted");
        storage.pread_exact(0, PIECE as u64, &mut probe).unwrap();
        storage.pread_exact(0, 2 * PIECE as u64, &mut probe).unwrap();
        storage.pread_exact(0, 3 * PIECE as u64, &mut probe).unwrap();
    }

    #[test]
    fn ghost_reads_are_counted_per_torrent() {
        let cap = PIECE as u64;
        let (pool, storage) = make_storage(&[1 << 20], PIECE, cap);

        // The registry entry a real torrent gets from the factory; ghost
        // accounting is per registered torrent.
        let info_hash = [3u8; 20];
        pool.lock().by_info_hash.insert(info_hash, storage.torrent_key);
        pool.lock().torrents.insert(
            storage.torrent_key,
            TorrentEntry {
                ghosts: BTreeSet::new(),
                ghost_reads: 0,
            },
        );

        storage.pwrite_all(0, 0, &fill(PIECE as usize, 1)).unwrap();
        storage
            .pwrite_all(0, PIECE as u64, &fill(PIECE as usize, 2))
            .unwrap();

        let mut probe = [0u8; 4];
        assert!(storage.pread_exact(0, 0, &mut probe).is_err());
        assert!(storage.pread_exact(0, 0, &mut probe).is_err());

        let stats = pool.stats();
        assert_eq!(stats.ghost_reads, 2);
    }

    #[test]
    fn forget_drops_all_pieces_of_a_torrent() {
        let (pool, storage) = make_storage(&[64 * 1024], PIECE, 1024 * 1024);

        // Touch all four pieces.
        for p in 0..4u32 {
            storage
                .pwrite_all(0, p as u64 * PIECE as u64, &fill(100, p as u64))
                .unwrap();
        }
        assert_eq!(pool.stats().pieces, 4);

        // The factory registers by info hash; the harness builds the
        // storage directly, so emulate the registry entry.
        let info_hash = [7u8; 20];
        pool.lock()
            .by_info_hash
            .insert(info_hash, storage.torrent_key);
        pool.forget(Id20::new(info_hash));

        let stats = pool.stats();
        assert_eq!(stats.pieces, 0);
        assert_eq!(stats.stored_bytes, 0);
        assert!(!pool.is_ram_backed(Id20::new(info_hash)));
    }

    #[test]
    fn is_ram_backed_tracks_registered_torrents() {
        let (pool, _storage) = make_storage(&[64 * 1024], PIECE, 1024 * 1024);
        assert_eq!(pool.stats().torrents, 0, "harness registers no torrent");

        // Registration happens in the factory; simulate one entry.
        let info_hash = [9u8; 20];
        pool.lock()
            .by_info_hash
            .insert(info_hash, 42);
        pool.lock().torrents.insert(
            42,
            TorrentEntry {
                ghosts: BTreeSet::new(),
                ghost_reads: 0,
            },
        );

        assert!(pool.is_ram_backed(Id20::new(info_hash)));
        assert_eq!(pool.stats().torrents, 1);
    }

    #[test]
    fn take_returns_a_fully_functional_storage_over_the_same_pool() {
        let (pool, storage) = make_storage(&[64 * 1024], PIECE, 1024 * 1024);

        storage.pwrite_all(0, 0, &fill(4096, 5)).unwrap();
        // One piece allocated at its full length.
        assert_eq!(pool.stats().stored_bytes, PIECE as u64);

        // librqbit keeps using the RETURN value of take() as the torrent's
        // storage, so it must be able to read AND write, over the same data.
        let taken = storage.take().unwrap();
        assert_eq!(
            pool.stats().stored_bytes,
            PIECE as u64,
            "data stays in the pool across a state change"
        );

        let mut out = [0u8; 16];
        taken.pread_exact(0, 0, &mut out).unwrap();
        assert_eq!(&out, &fill(16, 5)[..]);

        taken.pwrite_all(0, 4096, &[9, 9, 9]).unwrap();
        let mut out2 = [0u8; 3];
        taken.pread_exact(0, 4096, &mut out2).unwrap();
        assert_eq!(out2, [9, 9, 9]);

        // And the pool's counters survive unchanged by the swap.
        assert_eq!(pool.stats().pieces, 1);
    }

    #[test]
    fn stats_report_counters() {
        let cap = PIECE as u64;
        let (pool, storage) = make_storage(&[1 << 20], PIECE, cap);

        storage.pwrite_all(0, 0, &fill(PIECE as usize, 1)).unwrap();
        storage
            .pwrite_all(0, PIECE as u64, &fill(PIECE as usize, 2))
            .unwrap();

        let stats = pool.stats();
        assert_eq!(stats.limit_bytes, cap);
        assert_eq!(stats.stored_bytes, cap);
        assert_eq!(stats.pieces, 1);
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn ram_buffer_config_clamps_mib() {
        assert_eq!(RamBufferConfig::from_mib(0).buffer_bytes, 48 * 1024 * 1024);
        assert_eq!(RamBufferConfig::from_mib(1).buffer_bytes, 48 * 1024 * 1024);
        assert_eq!(RamBufferConfig::from_mib(256).buffer_bytes, 256 * 1024 * 1024);
        assert_eq!(
            RamBufferConfig::from_mib(99999).buffer_bytes,
            4096 * 1024 * 1024
        );
        assert_eq!(
            RamBufferConfig::default_buffer().buffer_bytes,
            DEFAULT_BUFFER_BYTES
        );
    }

    #[test]
    fn empty_writes_are_noops() {
        let (_pool, storage) = make_storage(&[64 * 1024], PIECE, 1024 * 1024);
        storage.pwrite_all(0, 0, &[]).unwrap();
        assert_eq!(pool_stats_stored(&storage), 0);
    }

    fn pool_stats_stored(storage: &RamStorage) -> u64 {
        storage.pool.stats().stored_bytes
    }
}
