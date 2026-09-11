//! Thin wrapper over a `librqbit` session.
//!
//! Deliberately small: everything that can be decided without touching the
//! network lives in [`super::magnet`] or in the option-builder function
//! below, both of which are unit tested. This type only owns the session
//! lifecycle, so its untested surface stays as close to zero as possible.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session};

/// Configuration for a streaming session.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Where downloaded pieces are written.
    pub download_dir: PathBuf,
}

impl EngineConfig {
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            download_dir: download_dir.into(),
        }
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

/// Ensure `dir` exists, creating parents as needed.
///
/// Separate from [`TorrentEngine::start`] so the filesystem behaviour can be
/// tested on its own.
pub fn ensure_download_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create download dir {}", dir.display()))
}

/// A running torrent session.
pub struct TorrentEngine {
    session: Arc<Session>,
    download_dir: PathBuf,
}

impl TorrentEngine {
    /// Start a session, creating `download_dir` if needed.
    ///
    /// This binds sockets and starts the DHT, so it is exercised by the
    /// `#[ignore]`d test below rather than the default suite.
    pub async fn start(config: EngineConfig) -> Result<Self> {
        ensure_download_dir(&config.download_dir)?;

        let session = Session::new(config.download_dir.clone())
            .await
            .context("failed to start torrent session")?;

        Ok(Self {
            session,
            download_dir: config.download_dir,
        })
    }

    /// Add a magnet URI and return the engine's torrent id.
    ///
    /// The magnet is validated first so a malformed link produces a clear
    /// error instead of an opaque engine failure.
    pub async fn add_magnet(&self, magnet_uri: &str) -> Result<usize> {
        super::magnet::parse_info_hash(magnet_uri)
            .map_err(|e| anyhow!("invalid magnet URI: {e}"))?;

        let response = self
            .session
            .add_torrent(
                AddTorrent::from_url(magnet_uri),
                Some(playback_options(None)),
            )
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

    /// Stop the session and all managed tasks.
    pub async fn stop(&self) {
        self.session.stop().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_options_enable_overwrite() {
        assert!(playback_options(None).overwrite);
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