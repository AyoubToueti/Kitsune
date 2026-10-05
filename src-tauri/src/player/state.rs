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
use serde::Serialize;
use tokio::sync::OnceCell;

use crate::settings::SettingsStore;
use crate::torrent::{api_for, EngineConfig, HttpBridge, TorrentEngine, TorrentProgress};

use super::launch::{resolve_player, spawn_player};

/// How long to wait for a torrent's metadata before giving up.
///
/// A local `.torrent` carries its own metadata, so this normally resolves on
/// the first poll. The wait exists so a magnet that never finds peers fails
/// with a message rather than hanging the command forever.
const METADATA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// How often to re-check for metadata while waiting.
const METADATA_POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// How often to check whether an external player has exited.
///
/// A second is imperceptible for cleanup that happens after a window closes,
/// and the check is a non-blocking `try_wait`, so the cost is a timer tick
/// rather than a thread.
const PLAYER_POLL: std::time::Duration = std::time::Duration::from_secs(1);

/// One file inside a torrent, as the frontend picks from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                session: OnceCell::new(),
                player: Mutex::new(resolve_player(None)),
                holds: Mutex::new(Holds::default()),
                settings: None,
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
    fn resolved_player(&self) -> String {
        match self.inner.settings.as_ref() {
            Some(store) => resolve_player(Some(&store.read().player)),
            None => self.player(),
        }
    }

    /// Extra arguments for the player, from settings when a store is present.
    fn player_args(&self) -> Vec<String> {
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

    /// The loopback URL that streams one file of one torrent.
    pub async fn stream_url(&self, torrent_id: usize, file_idx: usize) -> Result<String> {
        let session = self.session().await?;
        Ok(session.bridge.url_for(torrent_id, file_idx))
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

    /// Open a URL in an external player, holding `torrent_id` while it runs.
    ///
    /// `player` overrides the stored preference for this one call; passing
    /// `None` uses whatever the user chose.
    ///
    /// Async so the wait for the player to exit can be spawned: a sync Tauri
    /// command runs on the main thread, where `tokio::spawn` panics. The hold
    /// is taken BEFORE the spawn, so a removal racing the launch cannot slip in
    /// between.
    ///
    /// `on_exit` is called with the torrent id once the player process exits.
    /// It is how the frontend learns playback ended -- so it can stop showing
    /// "playing" and release the torrent. Kept as a plain closure rather than
    /// an `AppHandle` so this module stays free of Tauri and the policy can be
    /// driven in tests.
    pub async fn open_in_player(
        &self,
        url: &str,
        player: Option<&str>,
        torrent_id: Option<usize>,
        on_exit: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    ) -> Result<String> {
        let name = match player {
            Some(chosen) => resolve_player(Some(chosen)),
            // No override for this call: use the reader's stored choice, read
            // live so a change in settings applies to the very next launch.
            None => self.resolved_player(),
        };
        let extra = self.player_args();

        self.launch(url, &name, &extra, torrent_id, on_exit).await
    }

    /// Open a URL in the player the reader picked from the app chooser.
    ///
    /// The counterpart of [`Self::open_in_player`] for a desktop application
    /// chosen at launch time: it uses the program and arguments the chooser
    /// resolved, ignoring the stored preference entirely. Everything after the
    /// choice -- the hold, the exit watcher, the torrent release -- is shared
    /// with the default path, so a picked player cannot leak the torrent any
    /// more than the default can.
    pub async fn open_in_player_choice(
        &self,
        url: &str,
        choice: &super::chooser::PlayerChoice,
        torrent_id: Option<usize>,
        on_exit: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    ) -> Result<String> {
        self.launch(url, &choice.program, &choice.extra_args, torrent_id, on_exit)
            .await
    }

    /// The shared launch path: hold, spawn, watch for exit, release.
    ///
    /// Both public entry points differ only in which program and arguments
    /// they pass. Keeping the rest in one place is what guarantees the
    /// picked-player path holds and releases the torrent exactly like the
    /// default path. Returns the program name so the caller knows what opened.
    async fn launch(
        &self,
        url: &str,
        program: &str,
        extra_args: &[String],
        torrent_id: Option<usize>,
        on_exit: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    ) -> Result<String> {
        if let Some(id) = torrent_id {
            self.inner
                .holds
                .lock()
                .expect("holds mutex poisoned")
                .hold(id);
        }

        let child = match spawn_player(program, extra_args, url) {
            Ok(child) => child,
            Err(err) => {
                // The hold must not outlive a launch that never happened, or
                // the torrent could never be removed.
                if let Some(id) = torrent_id {
                    let _ = self.release_torrent(id).await;
                }
                return Err(err).with_context(|| format!("failed to launch {program}"));
            }
        };

        if let Some(id) = torrent_id {
            let state = self.clone();
            tokio::spawn(async move {
                wait_for_exit(child).await;

                // Tell the frontend playback ended BEFORE releasing the hold, so
                // it can stop polling and show the release list again. Fired for
                // every exit, including one where the page is still open and no
                // removal was ever requested.
                if let Some(callback) = on_exit {
                    callback(id);
                }

                if let Err(err) = state.release_torrent(id).await {
                    // The page is gone by now, so there is no UI to show this
                    // to. A failure leaves the torrent running, which is the
                    // lesser evil against removing one still in use.
                    tracing::warn!("failed to release torrent {id} after player exit: {err:#}");
                }
            });
        }

        Ok(program.to_string())
    }
}

/// Wait for a launched player to exit.
///
/// Polls with `try_wait` rather than awaiting the child: `tokio`'s `process`
/// feature is not enabled, and `tokio::process::Command` does not expose
/// `get_program`/`get_args`, which [`super::launch`]'s tests rely on. A
/// non-blocking check once a second costs a timer tick, not a thread.
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
    use crate::player::launch::DEFAULT_PLAYER;

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
