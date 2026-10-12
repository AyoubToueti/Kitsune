//! Torrent engine and the local HTTP bridge used to stream to a player.

pub mod engine;
pub mod http_bridge;
pub mod magnet;

pub use engine::{EngineConfig, TorrentEngine, TorrentProgress};
pub use http_bridge::{api_for, stream_url, HttpBridge, DEFAULT_STREAM_PORT};
pub use magnet::{build_magnet, parse_info_hash, MagnetError};