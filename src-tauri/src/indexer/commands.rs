//! Tauri commands for finding releases.
//!
//! Thin wrappers, like [`crate::player::commands`]: the work lives in
//! [`super::search`], so the command body only marshals arguments and maps the
//! error into a `String`.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use super::probe_rank::ProbeOutcome;
use super::{search, Indexer, ReleaseRequest};
use crate::types::{Release, ReleasePreference};

/// The indexer the app searches with.
///
/// A `Vec<Arc<dyn Indexer>>` rather than one indexer, so adding a second source
/// later is a change in this file alone. Held behind an `Arc` so a command can
/// clone it out of Tauri's state and search without holding a lock across the
/// await.
pub struct IndexerRegistry {
    indexers: Vec<Arc<dyn Indexer>>,
}

impl IndexerRegistry {
    /// A registry holding one Nyaa indexer.
    pub fn new() -> Self {
        Self {
            indexers: vec![Arc::new(super::NyaaIndexer::new())],
        }
    }

    /// A registry over an explicit set, for tests.
    pub fn with_indexers(indexers: Vec<Arc<dyn Indexer>>) -> Self {
        Self { indexers }
    }

    /// The indexers, in search order.
    pub fn indexers(&self) -> &[Arc<dyn Indexer>] {
        &self.indexers
    }
}

impl Default for IndexerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Search every indexer, returning the merged, ranked results.
///
/// Indexers are asked in parallel and merged afterwards: one slow source must
/// not delay the others, and a source that fails is skipped rather than failing
/// the whole search, so one broken mirror does not hide the results from the
/// rest.
#[tauri::command]
pub async fn search_releases(
    registry: State<'_, IndexerRegistry>,
    titles: Vec<String>,
    episode: Option<u32>,
    absolute_episode: Option<u32>,
    preference: Option<ReleasePreference>,
) -> Result<Vec<Release>, String> {
    let indexers: Vec<Arc<dyn Indexer>> = registry.indexers().to_vec();
    let request = ReleaseRequest {
        titles,
        episode,
        absolute_episode,
    };
    let preference = preference.unwrap_or_default();
    Ok(search_all(&indexers, &request, &preference).await)
}

/// Query every indexer concurrently and return one ranked list.
///
/// Split from the command so the fan-out can be tested with stub indexers. A
/// failing indexer contributes nothing rather than an error: a single dead
/// mirror should not turn a working search into a failure.
pub async fn search_all(
    indexers: &[Arc<dyn Indexer>],
    request: &ReleaseRequest,
    preference: &ReleasePreference,
) -> Vec<Release> {
    let mut handles = Vec::with_capacity(indexers.len());
    for indexer in indexers {
        let indexer = Arc::clone(indexer);
        let request = request.clone();
        handles.push(async move { search(indexer.as_ref(), &request, preference).await });
    }

    let mut merged: Vec<Release> = Vec::new();
    for handle in handles {
        if let Ok(mut found) = handle.await {
            merged.append(&mut found);
        }
    }

    // Each indexer already ranked its own results; re-ranking the merged set
    // puts a better release from the second indexer above a worse one from the
    // first, which per-indexer ranking cannot do.
    super::rank::rank(&mut merged, preference);
    merged
}

/// Download a `.torrent` file from an indexer to a path the user chose.
///
/// The fetch happens here rather than in the webview: the frontend cannot
/// reach nyaa.si directly (CORS), and a native request also lets the bytes be
/// written straight to disk instead of being held in a JS string.
///
/// `url` is validated to be `http(s)` before the request, so a `file://` or
/// other scheme smuggled in from a feed cannot be used to read local files.
#[tauri::command]
pub async fn download_torrent(url: String, path: String) -> Result<(), String> {
    download_to(&url, &PathBuf::from(path))
        .await
        .map_err(|err| err.to_string())
}

/// Fetch `url` and write the body to `path`.
///
/// Split from the command so the fetch-and-write can be tested against a mock
/// server without a Tauri app.
pub async fn download_to(url: &str, path: &std::path::Path) -> Result<(), DownloadError> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(DownloadError::InvalidUrl(url.to_string()));
    }

    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| DownloadError::Transport(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(DownloadError::Status(status.as_u16()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| DownloadError::Transport(e.to_string()))?;

    std::fs::write(path, &bytes).map_err(|e| DownloadError::Write(e.to_string()))?;
    Ok(())
}

/// Why a torrent download failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    /// The URL was not `http(s)`.
    InvalidUrl(String),
    /// The request itself failed (DNS, connection, timeout).
    Transport(String),
    /// The server answered with a non-success status.
    Status(u16),
    /// The bytes could not be written to disk.
    Write(String),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl(url) => write!(f, "not a downloadable URL: {url}"),
            Self::Transport(msg) => write!(f, "could not reach the indexer: {msg}"),
            Self::Status(code) => write!(f, "the indexer answered with status {code}"),
            Self::Write(msg) => write!(f, "could not write the torrent file: {msg}"),
        }
    }
}

impl std::error::Error for DownloadError {}

/// Probe a list of releases and report each result as it lands.
///
/// The releases are the ones a search just returned, so the frontend already
/// has them; sending them back costs one round trip but keeps the command
/// stateless.
///
/// Progress arrives as `probe-result` events, one per release, so the list can
/// re-rank while slower probes are still running. The return value is the whole
/// set, sorted by index, for a caller that would rather wait.
///
/// Probing never fails the command: a session that will not start, or a release
/// nobody will talk to, simply yields no result for that release and the static
/// ranking stands.
#[tauri::command]
pub async fn probe_releases(
    app: tauri::AppHandle,
    releases: Vec<Release>,
    preference: Option<ReleasePreference>,
) -> Result<Vec<ProbeOutcome>, String> {
    use tauri::Emitter;

    let preference = preference.unwrap_or_default();

    // The static half of every score, computed once. A probe only adds to it,
    // so re-deriving it per event would be wasted work.
    let base: Arc<Vec<i64>> = Arc::new(
        releases
            .iter()
            .map(|release| super::rank::score(release, &preference))
            .collect(),
    );

    let for_events = Arc::clone(&base);
    let events = app.clone();

    let probed = super::probe::probe_releases_on_own_session(
        probe_download_dir(),
        reqwest::Client::new(),
        &releases,
        move |index, probe| {
            let outcome = super::probe_rank::outcome(
                index,
                for_events.get(index).copied().unwrap_or(0),
                probe,
            );
            // A closed window makes emit fail; the probe is still valid, so
            // the error is dropped rather than aborting the batch.
            let _ = events.emit("probe-result", &outcome);
        },
    )
    .await;

    Ok(probed
        .into_iter()
        .map(|(index, probe)| {
            super::probe_rank::outcome(index, base.get(index).copied().unwrap_or(0), &probe)
        })
        .collect())
}

/// Where a probe batch writes its (empty) downloads.
///
/// The probe selects no files, so nothing is written; the directory exists only
/// because a librqbit session requires an output folder. Kept apart from the
/// player's directory so a probe can never be mistaken for downloaded content.
fn probe_download_dir() -> PathBuf {
    std::env::temp_dir().join("kitsune-probes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::traits::IndexerError;
    use crate::types::{ProviderId, ReleaseSource, Resolution};

    fn release_named(title: &str) -> Release {
        Release {
            title: title.to_string(),
            indexer: ProviderId::Nyaa,
            magnet_uri: "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862".into(),
            torrent_url: None,
            info_hash: None,
            size_bytes: None,
            seeders: Some(10),
            leechers: None,
            resolution: Resolution::R1080p,
            source: ReleaseSource::WebDl,
            remux: false,
            trusted: false,
            parsed: crate::indexer::parse::parse_release(title),
            score: 0,
        }
    }

    struct GoodIndexer(Vec<Release>);

    #[async_trait::async_trait]
    impl Indexer for GoodIndexer {
        fn name(&self) -> &str {
            "good"
        }

        async fn search(&self, _query: &str) -> Result<Vec<Release>, IndexerError> {
            Ok(self.0.clone())
        }
    }

    struct BadIndexer;

    #[async_trait::async_trait]
    impl Indexer for BadIndexer {
        fn name(&self) -> &str {
            "bad"
        }

        async fn search(&self, _query: &str) -> Result<Vec<Release>, IndexerError> {
            Err(IndexerError::Transport("down".into()))
        }
    }

    #[tokio::test]
    async fn a_failing_indexer_does_not_hide_a_working_one() {
        let indexers: Vec<Arc<dyn Indexer>> = vec![
            Arc::new(BadIndexer),
            Arc::new(GoodIndexer(vec![release_named("[G] Show - 05 [1080p]")])),
        ];

        let request = ReleaseRequest {
            titles: vec!["Show".into()],
            episode: Some(5),
            absolute_episode: None,
        };

        let found = search_all(&indexers, &request, &ReleasePreference::default()).await;
        assert_eq!(found.len(), 1);
    }

    #[tokio::test]
    async fn results_from_every_indexer_are_merged_and_ranked() {
        let indexers: Vec<Arc<dyn Indexer>> = vec![
            Arc::new(GoodIndexer(vec![release_named("[G] Show - 05 [480p]")])),
            Arc::new(GoodIndexer(vec![release_named("[G] Show - 05 [1080p]")])),
        ];

        let request = ReleaseRequest {
            titles: vec!["Show".into()],
            episode: Some(5),
            absolute_episode: None,
        };

        let found = search_all(&indexers, &request, &preference_default()).await;

        assert_eq!(found.len(), 2);
        assert!(found[0].title.contains("1080p"));
    }

    fn preference_default() -> ReleasePreference {
        ReleasePreference::default()
    }

    #[test]
    fn the_default_registry_holds_nyaa() {
        let registry = IndexerRegistry::new();
        assert_eq!(registry.indexers().len(), 1);
        assert_eq!(registry.indexers()[0].name(), "nyaa");
    }

    #[tokio::test]
    async fn download_writes_the_response_body_to_disk() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"d8:announce".to_vec()))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("release.torrent");

        download_to(&format!("{}/download/1.torrent", server.uri()), &path)
            .await
            .expect("should download");

        let written = std::fs::read(&path).expect("read back");
        assert_eq!(written, b"d8:announce");
    }

    #[tokio::test]
    async fn download_rejects_a_non_http_url() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("release.torrent");

        let result = download_to("file:///etc/passwd", &path).await;
        assert!(matches!(result, Err(DownloadError::InvalidUrl(_))));
        // Nothing was written.
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn download_surfaces_a_non_success_status() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("release.torrent");

        let result = download_to(&format!("{}/missing.torrent", server.uri()), &path).await;
        assert!(matches!(result, Err(DownloadError::Status(404))));
        assert!(!path.exists());
    }
}
