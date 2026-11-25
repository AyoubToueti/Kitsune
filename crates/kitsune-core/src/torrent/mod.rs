//! Torrent engine and the local HTTP bridge used to stream to a player.

pub mod engine;
pub mod http_bridge;
pub mod magnet;
pub mod ram_storage;

pub use engine::{
    available_space, download_options, ensure_download_dir, fits_on_disk, playback_options,
    EngineConfig, TorrentEngine, TorrentProgress,
};
pub use http_bridge::{api_for, stream_url, HttpBridge, DEFAULT_STREAM_PORT};
pub use magnet::{build_magnet, parse_info_hash, MagnetError};
pub use ram_storage::{
    RamBufferConfig, RamPool, RamPoolStats, RamStorageFactory, DEFAULT_BUFFER_BYTES,
};
