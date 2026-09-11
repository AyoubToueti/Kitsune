//! Manual smoke test: side-load a real `.torrent`, start the stream bridge,
//! and print the URL to open in a player.
//!
//! This is the only test that needs real content and network access, so it
//! is `#[ignore]`d and lives outside the unit suite. It proves the whole
//! Phase 2 chain actually works: parse -> session -> metadata -> stream URL.
//!
//! Run it with:
//!
//! ```text
//! cargo test --test smoke_stream -- --ignored --nocapture
//! ```
//!
//! Optional environment overrides:
//! - `KITSUNE_TORRENT`      path to the .torrent (default ../big-buck-bunny.torrent)
//! - `KITSUNE_DOWNLOAD_DIR` where pieces are written (default /tmp/kitsune-smoke)
//! - `KITSUNE_HOLD_SECS`    how long to keep the bridge up (default 600)

use std::path::PathBuf;
use std::time::Duration;

use kitsune_lib::torrent::{api_for, EngineConfig, HttpBridge, TorrentEngine};

fn torrent_path() -> PathBuf {
    std::env::var("KITSUNE_TORRENT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../big-buck-bunny.torrent"))
}

fn download_dir() -> PathBuf {
    std::env::var("KITSUNE_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/kitsune-smoke"))
}

fn hold_secs() -> u64 {
    std::env::var("KITSUNE_HOLD_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600)
}

/// Human-readable byte size, for the printed file list.
fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[tokio::test]
#[ignore = "needs a real .torrent file and network access"]
async fn side_load_torrent_and_serve_stream() {
    let path = torrent_path();
    assert!(
        path.exists(),
        "torrent not found at {}\nset KITSUNE_TORRENT to point at one",
        path.display()
    );

    let dir = download_dir();
    println!("\n=== Kitsune stream smoke test ===");
    println!("torrent      : {}", path.display());
    println!("download dir : {}", dir.display());

    let engine = TorrentEngine::start(EngineConfig::new(&dir))
        .await
        .expect("engine should start");

    let id = engine
        .add_torrent_file(&path)
        .await
        .expect("torrent should be accepted");

    println!("torrent id   : {id}");

    // A local .torrent carries its metadata, so files should resolve almost
    // immediately. Poll briefly rather than assuming.
    let mut files = Vec::new();
    for _ in 0..50 {
        files = engine.torrent_files(id);
        if !files.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(
        !files.is_empty(),
        "no files resolved for torrent {id} after 5s"
    );

    let bridge = HttpBridge::start(api_for(engine.session()), 0)
        .await
        .expect("bridge should start");

    println!("\nfiles:");
    for (idx, name, len) in &files {
        println!("  [{idx}] {name} ({})", human(*len));
        println!("       {}", bridge.url_for(id, *idx));
    }

    // Pick the largest file: in a real release that is the video, not a
    // subtitle or cover image. Good enough for a smoke test.
    let (vidx, vname, vlen) = files
        .iter()
        .max_by_key(|(_, _, len)| *len)
        .expect("files is non-empty");

    let url = bridge.url_for(id, *vidx);

    println!("\n=== play it ===");
    println!("  mpv '{url}'");
    println!("\nlargest file: {vname} ({})", human(*vlen));
    println!("bridge port : {}", bridge.port());
    println!(
        "\nkeeping the bridge up for {}s so you can play it...",
        hold_secs()
    );

    // Hold so the bridge stays alive while a player connects. Without this
    // the test would end and drop the server before anything could stream.
    tokio::time::sleep(Duration::from_secs(hold_secs())).await;

    bridge.shutdown();
    engine.stop().await;
}