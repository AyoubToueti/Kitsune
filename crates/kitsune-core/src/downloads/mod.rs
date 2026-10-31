//! Persistent tracking of downloaded episodes.
//!
//! A JSON file in the OS data directory, following the same pattern as
//! settings and last-played stores. Tracks what episodes have been downloaded,
//! their status, and where the files are stored.

mod organize;
mod queue;
mod store;
mod torrent_cache;

pub use organize::{extension_of, library_path};
pub use queue::{plan_queue, QueueAction};
pub use store::{DownloadPriority, DownloadRecord, DownloadStatus, DownloadsStore};
pub use torrent_cache::{
    cached_path as cached_torrent_path, data_dir_in as download_cache_dir,
    load as load_cached_torrent, remove as remove_cached_torrent,
    remove_data_dir as remove_download_cache, save as save_cached_torrent,
};
