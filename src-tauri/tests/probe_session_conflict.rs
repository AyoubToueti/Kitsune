//! Proves whether a probe and a playback can share one librqbit session.
//!
//! The probe registers each candidate torrent with an empty file selection so
//! nothing downloads. If the player then asks for the same torrent on the same
//! session, librqbit's `add_torrent` may short-circuit on the already-managed
//! info hash -- in which case the playback request's options are discarded and
//! the empty selection persists, so nothing would ever download.
//!
//! That would make a shared session unusable, so the wiring in
//! `indexer::commands` depends on the answer. This test establishes it
//! against the real library rather than trusting a reading of its source.
//!
//! Uses the local Big Buck Bunny `.torrent`, so it needs no network and
//! resolves metadata immediately.
//!
//! ```text
//! cargo test --test probe_session_conflict
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use librqbit::{AddTorrent, AddTorrentOptions, Session};

/// The repository's bundled well-known torrent.
fn torrent_path() -> PathBuf {
    PathBuf::from("../big-buck-bunny.torrent")
}

/// Add a torrent with an explicit file selection, returning its id.
///
/// `only_files: Some(vec![])` is what the probe uses: no file selected, so
/// nothing is fetched.
async fn add_with_empty_selection(
    session: &Arc<Session>,
    path: &std::path::Path,
) -> usize {
    let add = AddTorrent::from_local_filename(path.to_str().expect("utf-8 path"))
        .expect("read torrent file");

    let response = session
        .add_torrent(
            add,
            Some(AddTorrentOptions {
                overwrite: true,
                only_files: Some(Vec::new()),
                ..Default::default()
            }),
        )
        .await
        .expect("add torrent");

    response.into_handle().expect("handle").id()
}

/// Re-add the same torrent the way playback would: no file restriction.
async fn add_for_playback(
    session: &Arc<Session>,
    path: &std::path::Path,
) -> usize {
    let add = AddTorrent::from_local_filename(path.to_str().expect("utf-8 path"))
        .expect("read torrent file");

    let response = session
        .add_torrent(add, Some(AddTorrentOptions::default()))
        .await
        .expect("add torrent");

    response.into_handle().expect("handle").id()
}

/// The empirical question: does re-adding on one session apply new options?
///
/// If `only_files` is still an empty list after the second add, the playback
/// request was ignored and a shared session cannot work.
#[tokio::test]
async fn re_adding_a_torrent_keeps_the_first_file_selection() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session = Session::new(dir.path().to_path_buf())
        .await
        .expect("session should start");

    let path = torrent_path();
    assert!(path.exists(), "expected the bundled torrent at {path:?}");

    let probe_id = add_with_empty_selection(&session, &path).await;
    let probe_files = session
        .get(probe_id.into())
        .and_then(|handle| handle.only_files());
    assert_eq!(
        probe_files,
        Some(Vec::new()),
        "the probe's add should have selected no files"
    );

    let playback_id = add_for_playback(&session, &path).await;
    assert_eq!(
        probe_id, playback_id,
        "re-adding the same torrent should return the same id"
    );

    let after_playback = session
        .get(playback_id.into())
        .and_then(|handle| handle.only_files());

    // This is the assertion that decides the design. If it holds, playback's
    // options were discarded and a shared session is unusable: the probe's
    // empty selection would silently prevent the episode from downloading.
    assert_eq!(
        after_playback,
        Some(Vec::new()),
        "re-adding reused the probe's empty file selection; a shared session \
         would break playback, so probes need their own session"
    );

    session.stop().await;
}