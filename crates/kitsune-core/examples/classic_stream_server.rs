//! Serves a real video file through kitsune's classic (disk) streaming
//! bridge, so external probes (headless mpv) can hit the REAL endpoint --
//! axum keep-alive, the streaming FileStream, piece fetching, all of it --
//! without launching the full GUI.
//!
//! Usage:
//!
//! ```text
//! cargo run -p kitsune-core --example classic_stream_server -- <video file>
//! ```
//!
//! A one-shot seeder session shares the file on loopback, a classic-mode
//! leecher streams it, and the bridge prints its URL. Ctrl-C tears down.

use std::net::SocketAddr;
use std::path::Path;

use anyhow::{Context, Result};

use kitsune_core::torrent::{api_for, EngineConfig, HttpBridge, TorrentEngine};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("librqbit=warn")),
        )
        .with_target(false)
        .try_init();

    let source = std::env::args()
        .nth(1)
        .context("usage: classic_stream_server <video file>")?;
    let source_path = Path::new(&source).canonicalize()?;
    let seed_dir = source_path
        .parent()
        .context("source has no parent dir")?
        .to_path_buf();

    // The torrent is rebuilt each run; the seeder looks for the file by its
    // name in `output_folder`, so the torrent's info name must stay the file
    // name (no `name` override for single-file torrents).
    let spawner = librqbit::spawn_utils::BlockingSpawner::new(4);
    let created = librqbit::create_torrent(
        &source_path,
        librqbit::CreateTorrentOptions {
            name: None,
            trackers: Vec::new(),
            piece_length: None,
        },
        &spawner,
    )
    .await
    .context("failed to create the torrent")?;
    let torrent_bytes = created
        .as_bytes()
        .context("failed to serialise the torrent")?;
    let torrent_path = std::env::temp_dir().join("kitsune-classic-probe.torrent");
    std::fs::write(&torrent_path, &torrent_bytes)?;

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

    let response = seed_session
        .add_torrent(
            librqbit::AddTorrent::from_bytes(torrent_bytes.clone()),
            Some(librqbit::AddTorrentOptions {
                overwrite: true,
                output_folder: Some(seed_dir.display().to_string()),
                ..Default::default()
            }),
        )
        .await
        .context("failed to add the torrent to the seeder")?;
    let seed_handle = response
        .into_handle()
        .context("seeder got no torrent handle")?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
    while !seed_handle.stats().finished {
        anyhow::ensure!(
            tokio::time::Instant::now() < deadline,
            "seeder never finished checking its file"
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    println!("seeder ready on {seeder_addr}");

    // Classic mode: no RAM buffer, pieces land on the disk like a real
    // episode stream.
    let leech_dir = std::env::temp_dir().join("kitsune-classic-probe-leech");
    std::fs::create_dir_all(&leech_dir)?;
    let engine = TorrentEngine::start(
        EngineConfig::new(&leech_dir)
            .with_initial_peers(vec![seeder_addr])
            .without_dht(),
    )
    .await
    .context("leecher engine failed to start")?;
    let id = engine
        .add_torrent_file(&torrent_path)
        .await
        .context("failed to add the torrent in classic mode")?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    while engine.torrent_files(id).is_empty() {
        anyhow::ensure!(
            tokio::time::Instant::now() < deadline,
            "torrent metadata never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let bridge = HttpBridge::start(api_for(engine.session()), 0).await?;
    let url = bridge.url_for(id, 0);
    println!("PROBE_URL={url}");
    println!("streaming; Ctrl-C to stop (the runtime dies with the process)");

    // Park forever: SIGINT terminates the process, which drops everything.
    // pending() never resolves, so nothing follows.
    #[allow(unreachable_code)]
    {
        std::future::pending::<()>().await;
    }

    engine.remove_torrent(id).await?;
    bridge.shutdown();
    engine.stop().await;
    seed_session.stop().await;
    Ok(())
}
