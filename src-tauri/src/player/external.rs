//! Handing a stream URL to a desktop video player.
//!
//! This used to live on `PlayerState` in `kitsune-core`, but the Qt frontend
//! plays in-process through libmpv and never launches anything, so the code
//! belongs to the one frontend that does.
//!
//! What stays in core is the part both frontends share: the torrent session,
//! the stream bridge, and the holds that keep a torrent alive while something
//! outside the process reads it.

use std::sync::Arc;

use anyhow::{Context, Result};
use kitsune_core::player::state::PlayerState;

use super::choice::PlayerChoice;
use super::launch::{resolve_player, spawn_player};

/// How often to check whether the player process has exited.
///
/// `tokio`'s `process` feature is not enabled here, so the child cannot be
/// awaited; polling `try_wait` once a second costs a timer tick, not a thread.
const PLAYER_POLL: std::time::Duration = std::time::Duration::from_secs(1);

/// Open a URL in an external player, holding `torrent_id` while it runs.
///
/// `player` overrides the stored preference for this one call; passing `None`
/// uses whatever the reader chose.
///
/// Async so the wait for the player to exit can be spawned: a sync Tauri
/// command runs on the main thread, where `tokio::spawn` panics. The hold is
/// taken BEFORE the spawn, so a removal racing the launch cannot slip in
/// between.
///
/// `on_exit` is called with the torrent id once the player process exits. It is
/// how the frontend learns playback ended -- so it can stop showing "playing"
/// and release the torrent. Kept as a plain closure rather than an `AppHandle`
/// so the policy can be driven in tests.
pub async fn open_in_player(
    state: &PlayerState,
    url: &str,
    player: Option<&str>,
    torrent_id: Option<usize>,
    on_exit: Option<Arc<dyn Fn(usize) + Send + Sync>>,
) -> Result<String> {
    let name = match player {
        Some(chosen) => resolve_player(Some(chosen)),
        // No override for this call: use the reader's stored choice, read live
        // so a change in settings applies to the very next launch.
        None => state.resolved_player(),
    };
    let extra = state.player_args();

    launch(state, url, &name, &extra, torrent_id, on_exit).await
}

/// Open a URL in the player the reader picked from the app chooser.
///
/// The counterpart of [`open_in_player`] for a desktop application chosen at
/// launch time: it uses the program and arguments the chooser resolved,
/// ignoring the stored preference entirely. Everything after the choice -- the
/// hold, the exit watcher, the torrent release -- is shared with the default
/// path, so a picked player cannot leak the torrent any more than the default
/// can.
pub async fn open_in_player_choice(
    state: &PlayerState,
    url: &str,
    choice: &PlayerChoice,
    torrent_id: Option<usize>,
    on_exit: Option<Arc<dyn Fn(usize) + Send + Sync>>,
) -> Result<String> {
    launch(state, url, &choice.program, &choice.extra_args, torrent_id, on_exit).await
}

/// The shared launch path: hold, spawn, watch for exit, release.
///
/// Both public entry points differ only in which program and arguments they
/// pass. Keeping the rest in one place is what guarantees the picked-player
/// path holds and releases the torrent exactly like the default path. Returns
/// the program name so the caller knows what opened.
async fn launch(
    state: &PlayerState,
    url: &str,
    program: &str,
    extra_args: &[String],
    torrent_id: Option<usize>,
    on_exit: Option<Arc<dyn Fn(usize) + Send + Sync>>,
) -> Result<String> {
    if let Some(id) = torrent_id {
        state.hold_torrent(id);
    }

    let child = match spawn_player(program, extra_args, url) {
        Ok(child) => child,
        Err(err) => {
            // The hold must not outlive a launch that never happened, or the
            // torrent could never be removed.
            if let Some(id) = torrent_id {
                let _ = state.release_torrent(id).await;
            }
            return Err(err).with_context(|| format!("failed to launch {program}"));
        }
    };

    if let Some(id) = torrent_id {
        let state = state.clone();
        tokio::spawn(async move {
            wait_for_exit(child).await;

            // Tell the frontend playback ended BEFORE releasing the hold, so it
            // can stop polling and show the release list again. Fired for every
            // exit, including one where the page is still open and no removal
            // was ever requested.
            if let Some(callback) = on_exit {
                callback(id);
            }

            if let Err(err) = state.release_torrent(id).await {
                // The page is gone by now, so there is no UI to show this to. A
                // failure leaves the torrent running, which is the lesser evil
                // against removing one still in use.
                tracing::warn!("failed to release torrent {id} after player exit: {err:#}");
            }
        });
    }

    Ok(program.to_string())
}

/// Wait for a launched player to exit.
///
/// Polls with `try_wait` rather than awaiting the child: `tokio`'s `process`
/// feature is not enabled, and `tokio::process::Command` does not expose
/// `get_program`/`get_args`, which [`super::launch`]'s tests rely on.
///
/// An error from `try_wait` is treated as an exit: the child can no longer be
/// waited on, so the hold should not be kept.
async fn wait_for_exit(mut child: std::process::Child) {
    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => tokio::time::sleep(PLAYER_POLL).await,
        }
    }
}