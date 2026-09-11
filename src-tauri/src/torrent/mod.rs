//! Torrent engine and the local HTTP bridge used to stream to a player.

pub mod engine;
pub mod magnet;

pub use engine::{EngineConfig, TorrentEngine};
pub use magnet::{build_magnet, parse_info_hash, MagnetError};