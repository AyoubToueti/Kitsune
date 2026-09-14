//! The player session: a torrent engine, its stream bridge, and the chosen
//! external player.
//!
//! The session is started lazily on the first command that needs it, so
//! launching the app does not bind sockets or join the DHT until the reader
//! actually plays something.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use tokio::sync::OnceCell;

use crate::torrent::{api_for, EngineConfig, HttpBridge, TorrentEngine};

use super::launch::{resolve_player, spawn_player};

/// How long to wait for a torrent's metadata before giving up.
///
/// A local `.torrent` carries its own metadata, so this normally resolves on
/// the first poll. The wait exists so a magnet that never finds peers fails
/// with a message rather than hanging the command forever.
const METADATA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// How often to re-check for metadata while waiting.
const METADATA_POLL: std::time::Duration = std::time::Duration::from_millis(100);

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

/// Player state shared across the Tauri commands.
pub struct PlayerState {
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
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            session: OnceCell::new(),
            player: Mutex::new(resolve_player(None)),
        }
    }

    /// The chosen external player.
    pub fn player(&self) -> String {
        self.player.lock().expect("player mutex poisoned").clone()
    }

    /// Choose the external player. A blank name restores the default.
    pub fn set_player(&self, name: &str) {
        *self.player.lock().expect("player mutex poisoned") = resolve_player(Some(name));
    }

    /// Start the torrent engine and stream bridge, once.
    async fn start_session() -> Result<Session> {
        let engine = TorrentEngine::start(EngineConfig::new(download_dir()))
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
        self.session.get_or_try_init(Self::start_session).await
    }

    /// Add a `.torrent` file and wait for the files it contains.
    pub async fn add_torrent(&self, path: &Path) -> Result<TorrentHandle> {
        let session = self.session().await?;

        let id = session
            .engine
            .add_torrent_file(path)
            .await
            .with_context(|| format!("failed to add {}", path.display()))?;

        let files = wait_for_files(&session.engine, id).await?;
        Ok(TorrentHandle { id, files })
    }

    /// The loopback URL that streams one file of one torrent.
    pub async fn stream_url(&self, torrent_id: usize, file_idx: usize) -> Result<String> {
        let session = self.session().await?;
        Ok(session.bridge.url_for(torrent_id, file_idx))
    }

    /// Open a URL in an external player.
    ///
    /// `player` overrides the stored preference for this one call; passing
    /// `None` uses whatever the user chose.
    pub fn open_in_player(&self, url: &str, player: Option<&str>) -> Result<String> {
        let name = match player {
            Some(chosen) => resolve_player(Some(chosen)),
            None => self.player(),
        };

        spawn_player(&name, url).with_context(|| format!("failed to launch `{name}`"))?;
        Ok(name)
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
