//! End-to-end proof of RAM streaming: seed a real torrent in-process,
//! stream it through the RAM-mode engine and the HTTP bridge, and verify
//! the bytes, the buffer cap and the empty download directory.
//!
//! This is the Rust counterpart of the prototype's test suite. It binds
//! loopback sockets (an in-process seeder and leecher talk directly), so it
//! is `#[ignore]`d like the other integration tests:
//!
//! ```text
//! cargo test --test ram_stream_e2e -- --ignored --nocapture
//! ```
//!
//! What it pins down, in order:
//!
//! 1. A torrent added through the playback path in RAM mode registers with
//!    the pool and reports no on-disk path.
//! 2. Reading the WHOLE file over HTTP yields bytes identical to the source
//!    (FNV-1a 64 over both), with zero ghost reads.
//! 3. A mid-file seek (a separate Range request) reads the exact bytes at
//!    that offset -- the piece-mapping math, exercised live.
//! 4. The pool never exceeds its configured cap, and the LRU has actually
//!    evicted (the file is larger than the buffer).
//! 5. The leecher's download directory contains no files at any point:
//!    nothing was ever written to disk.

use std::net::SocketAddr;
use std::path::Path;

use anyhow::{Context, Result};
use kitsune_core::torrent::{
    api_for, EngineConfig, HttpBridge, RamBufferConfig, TorrentEngine, DEFAULT_STREAM_PORT,
};

/// Size of the test payload: 64 MiB, comfortably above the 24 MiB buffer so
/// eviction is exercised continuously during the read.
const SOURCE_BYTES: u64 = 64 * 1024 * 1024;

/// Piece length for the test torrent: 1 MiB -> 64 pieces.
const PIECE_LEN: u32 = 1024 * 1024;

/// The leecher's buffer: 48 MiB. Above librqbit's hard-coded 32 MiB stream
/// window (a smaller buffer thrashes: the look-ahead is evicted before the
/// reader reaches it), yet well below the 64 MiB file, so eviction is
/// exercised continuously during the read.
const BUFFER_BYTES: u64 = 48 * 1024 * 1024;

/// FNV-1a 64: dependency-free, order-sensitive, good enough to prove two
/// byte streams are identical.
fn fnv1a(data: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;

/// Write the source file with a deterministic (xorshift64*) byte pattern so
/// the expected content is reproducible without keeping it in memory.
fn write_source(path: &Path) -> Result<u64> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    let mut state = 0x9E3779B97F4A7C15u64;
    let mut written = 0u64;
    let mut buf = vec![0u8; 64 * 1024];

    while written < SOURCE_BYTES {
        for b in buf.iter_mut() {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            *b = (state.wrapping_mul(0x2545F4914F6CDD1D) >> 24) as u8;
        }
        let remaining = (SOURCE_BYTES - written) as usize;
        let n = remaining.min(buf.len());
        file.write_all(&buf[..n])?;
        written += n as u64;
    }

    file.flush()?;
    Ok(fnv1a(&std::fs::read(path)?, FNV_OFFSET))
}

/// FNV over a reqwest response body, consumed in chunks.
async fn fnv_of_response(response: &mut reqwest::Response) -> Result<(u64, u64)> {
    let mut hash = FNV_OFFSET;
    let mut total = 0u64;
    while let Some(chunk) = response.chunk().await? {
        hash = fnv1a(&chunk, hash);
        total += chunk.len() as u64;
    }
    Ok((hash, total))
}

/// Wait until the torrent reports at least one file (metadata resolved).
async fn wait_for_metadata(engine: &TorrentEngine, id: usize) -> Result<()> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if !engine.torrent_files(id).is_empty() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("torrent {id} did not resolve metadata in time");
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// Count files under `dir`, recursively. A RAM session must always report 0.
fn count_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let path = e.path();
                    if path.is_dir() {
                        count_files(&path)
                    } else {
                        1
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "binds loopback sockets; run explicitly"]
async fn ram_stream_delivers_exact_bytes_without_touching_the_disk() -> Result<()> {
    // Engine logs are the only way to attribute a stall; tests do not run
    // the app's subscriber, so install a minimal one here.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("librqbit=info")),
        )
        .with_target(false)
        .try_init();

    let tmp = tempfile::tempdir()?;
    let seed_dir = tmp.path().join("seed");
    let leech_dir = tmp.path().join("leech");
    std::fs::create_dir_all(&seed_dir)?;
    std::fs::create_dir_all(&leech_dir)?;

    // ---- 1. Source content and .torrent ----
    let source_path = seed_dir.join("episode.bin");
    let expected_hash = write_source(&source_path)?;
    println!("source written: {SOURCE_BYTES} bytes, fnv={expected_hash:#x}");

    let spawner = librqbit::spawn_utils::BlockingSpawner::new(4);
    let created = librqbit::create_torrent(
        &source_path,
        // No `name` override: for a single-file torrent the info name IS the
        // file name, and renaming it would make the seeder look for a file
        // that does not exist.
        librqbit::CreateTorrentOptions {
            name: None,
            trackers: Vec::new(),
            piece_length: Some(PIECE_LEN),
        },
        &spawner,
    )
    .await
    .context("failed to create the test torrent")?;
    let torrent_bytes = created
        .as_bytes()
        .context("failed to serialise the test torrent")?;
    let torrent_path = tmp.path().join("test.torrent");
    std::fs::write(&torrent_path, &torrent_bytes)?;
    println!("torrent created: info_hash={:?}", created.info_hash());

    // ---- 2. The seeder: its own session, DHT off, fixed discovery ----
    let seed_session = librqbit::Session::new_with_opts(
        seed_dir.clone(),
        librqbit::SessionOptions {
            dht: None,
            disable_trackers: true,
            listen: Some(librqbit::ListenerOptions {
                listen_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .context("seeder session failed to start")?;

    let seeder_addr = seed_session
        .listen_addr()
        .context("seeder did not report a listen address")?;
    println!("seeder listening on {seeder_addr}");

    let response = seed_session
        .add_torrent(
            librqbit::AddTorrent::from_bytes(torrent_bytes.clone()),
            Some(librqbit::AddTorrentOptions {
                overwrite: true,
                // The torrent's own name would become a subfolder; the file
                // lives directly in seed_dir, so point at it explicitly.
                output_folder: Some(seed_dir.display().to_string()),
                ..Default::default()
            }),
        )
        .await
        .context("failed to add the torrent to the seeder")?;
    let seed_handle = response
        .into_handle()
        .context("seeder got no torrent handle")?;

    // The seeder must have the file complete before the leecher starts, or
    // the read would stall waiting for pieces nobody has.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let stats = seed_handle.stats();
        if stats.finished {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "seeder never finished checking its file: state={:?} progress={}/{} total={}/{} error={:?}",
                stats.state,
                stats.progress_bytes,
                stats.total_bytes,
                stats.file_progress.first().copied().unwrap_or(0),
                SOURCE_BYTES,
                stats.error
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    println!("seeder ready");

    // ---- 3. The leecher: the RAM-mode engine under test ----
    let engine = TorrentEngine::start(
        EngineConfig::new(&leech_dir)
            .with_ram(RamBufferConfig { buffer_bytes: BUFFER_BYTES })
            .with_initial_peers(vec![seeder_addr])
            .without_dht(),
    )
    .await
    .context("leecher engine failed to start")?;

    assert!(engine.ram_mode(), "RAM mode should be active");
    let id = engine
        .add_torrent_file(&torrent_path)
        .await
        .context("failed to add the torrent in RAM mode")?;

    wait_for_metadata(&engine, id).await?;
    let files = engine.torrent_files(id);
    assert_eq!(files.len(), 1, "one file expected");
    assert_eq!(files[0].2, SOURCE_BYTES, "file length from metadata");
    println!("leecher added torrent {id}: {:?}", files);

    // A RAM-backed torrent has no on-disk path -- the guard the download
    // path and finish_download rely on.
    assert!(
        engine.file_disk_path(id, 0).is_none(),
        "a RAM-backed torrent must report no disk path"
    );

    // ---- 4. Stream the whole file through the bridge ----
    let bridge = HttpBridge::start(api_for(engine.session()), 0).await?;
    let url = bridge.url_for(id, 0);
    println!("stream url: {url}");

    let client = reqwest::Client::new();

    // The full-file GET mirrors what a player does from 0:00 to EOF.
    let transfer = async {
        let mut response = client.get(&url).send().await?.error_for_status()?;
        assert_eq!(
            response.status(),
            200,
            "a request without Range must answer 200"
        );
        let (hash, total) = fnv_of_response(&mut response).await?;
        Ok::<_, anyhow::Error>((hash, total))
    };
    let (full_hash, total) = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        transfer,
    )
    .await
    .context("full-file transfer timed out")??;

    assert_eq!(total, SOURCE_BYTES, "every byte must arrive");
    assert_eq!(
        full_hash, expected_hash,
        "streamed bytes differ from the source"
    );
    println!("full read OK: {total} bytes, fnv={full_hash:#x}");
    // ---- 5. Seek: a mid-file Range request must read the exact bytes ----
    let seek_offset = SOURCE_BYTES / 2 + 123_456; // deliberately not piece-aligned
    let seek_url = url.clone();
    let range = format!("bytes={seek_offset}-{}", seek_offset + 2 * 1024 * 1024 - 1);
    let seek_transfer = async {
        let mut response = client
            .get(&seek_url)
            .header(reqwest::header::RANGE, &range)
            .send()
            .await?
            .error_for_status()?;
        assert_eq!(
            response.status(),
            206,
            "a ranged request must answer 206 Partial Content"
        );
        let (hash, total) = fnv_of_response(&mut response).await?;
        Ok::<_, anyhow::Error>((hash, total))
    };
    let (seek_hash, seek_total) = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        seek_transfer,
    )
    .await
    .context("seek transfer timed out")??;

    assert_eq!(seek_total, 2 * 1024 * 1024);
    let expected_slice: Vec<u8> = {
        let file = std::fs::File::open(&source_path)?;
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::io::BufReader::new(file);
        file.seek(SeekFrom::Start(seek_offset))?;
        let mut buf = vec![0u8; 2 * 1024 * 1024];
        file.read_exact(&mut buf)?;
        buf
    };
    assert_eq!(
        seek_hash,
        fnv1a(&expected_slice, FNV_OFFSET),
        "mid-file seek read different bytes"
    );
    println!("seek read OK: {seek_total} bytes at offset {seek_offset}");

    // ---- 6. Buffer accounting ----
    let pool = engine.ram_pool().expect("RAM mode must expose the pool");
    let stats = pool.stats();
    println!(
        "pool: {}/{} bytes, peak={}, pieces={}, evictions={}, ghosts={}",
        stats.stored_bytes, stats.limit_bytes, stats.peak_bytes, stats.pieces,
        stats.evictions, stats.ghost_reads
    );

    assert!(
        stats.peak_bytes <= BUFFER_BYTES,
        "the pool must never exceed its cap"
    );
    assert!(
        stats.evictions > 0,
        "a 64 MiB read through a 48 MiB buffer must have evicted"
    );
    if stats.ghost_reads != 0 {
        // Print WHICH pieces ghosted before failing: the indices say whether
        // the miss was the mid-file seek back into an evicted region (the
        // seek's own stream re-reads pieces the sequential pass dropped) or
        // something in the sequential pass itself (a bug).
        let handle = engine.session().get(id.into()).expect("handle");
        println!(
            "ghost pieces: {:?}",
            pool.ghost_pieces(handle.info_hash())
        );
    }
    assert_eq!(stats.ghost_reads, 0, "sequential playback must not hit ghosts");

    // ---- 7. Zero disk writes ----
    let files_on_disk = count_files(&leech_dir);
    assert_eq!(
        files_on_disk, 0,
        "RAM streaming must not write anything to the download dir"
    );

    // ---- 8. Cleanup tears the pool down with the torrent ----
    engine.remove_torrent(id).await?;
    assert_eq!(pool.stats().stored_bytes, 0, "removal must free the buffer");
    assert_eq!(pool.stats().torrents, 0, "removal must drop the registry entry");

    bridge.shutdown();
    engine.stop().await;
    seed_session.stop().await;
    Ok(())
}

/// The bridge's default port constant is what non-test callers bind; keep
/// the import honest even though this test asks the OS for a port.
#[allow(dead_code)]
const _PORT_SANITY: u16 = DEFAULT_STREAM_PORT;
