//! Manual smoke test: side-load a real `.torrent`, start the stream bridge,
//! launch a player, and print the URL.
//!
//! This is the only test that needs real content and network access, so it
//! is `#[ignore]`d and lives outside the unit suite. It proves the whole
//! Phase 2 chain actually works: parse -> session -> metadata -> stream URL
//! -> playback.
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
//! - `KITSUNE_PLAYER`       player binary to launch (default `mpv`)
//!
//! The bridge stays up until the player exits, or for `KITSUNE_HOLD_SECS`
//! if the player is not installed or does not exit. Exit mpv to end the
//! test early.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
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

/// Player binary to launch. Defaults to `mpv`; override with
/// `KITSUNE_PLAYER=vlc` (or any player that accepts a URL as its first arg).
fn player_command() -> String {
    std::env::var("KITSUNE_PLAYER").unwrap_or_else(|_| "mpv".to_string())
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

/// True for containers a browser `<video>` element will refuse, so we can
/// warn instead of leaving the user staring at a black frame at 0:00.
fn browser_unfriendly(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".mkv", ".avi", ".ts", ".m2ts", ".wmv"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// Spawn the player against the stream URL.
///
/// Returns `None` (rather than panicking) if the binary is missing, so the
/// test still prints the URL and holds the bridge — the user can then open
/// it manually.
fn launch_player(player: &str, url: &str) -> Option<Child> {
    match Command::new(player)
        .arg(url)
        // Detach from the test's stdio so mpv owns its terminal; without
        // this, mpv's keybindings would fight with cargo's output.
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => Some(child),
        Err(err) => {
            eprintln!("could not launch `{player}`: {err}");
            None
        }
    }
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
    println!("  URL: {url}");
    println!("  largest file: {vname} ({})", human(*vlen));
    if browser_unfriendly(vname) {
        println!(
            "  note: this container is not playable in a browser <video> element;"
        );
        println!("        use mpv/VLC, not Chrome/Firefox.");
    }

    let player = player_command();
    println!("\nlaunching `{player}`...");
    let mut child = launch_player(&player, &url);
    if child.is_none() {
        println!("(player not found — open the URL above manually)");
    }

    println!("bridge port : {}", bridge.port());
    println!(
        "holding bridge up for up to {}s — close the player to end early...",
        hold_secs()
    );

    // Wait for the player to exit, or for the hold deadline, whichever
    // comes first. Without this the test would end and drop the server
    // before anything could stream.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(hold_secs());
    loop {
        if let Some(c) = child.as_mut() {
            match c.try_wait() {
                Ok(Some(status)) => {
                    println!("\nplayer exited: {status}");
                    break;
                }
                Ok(None) => {}
                Err(err) => {
                    eprintln!("failed to poll player: {err}");
                    break;
                }
            }
        }

        if tokio::time::Instant::now() >= deadline {
            println!("\nhold time elapsed; stopping player if still running");
            if let Some(c) = child.as_mut() {
                let _ = c.kill();
                let _ = c.wait();
            }
            break;
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    bridge.shutdown();
    engine.stop().await;
}