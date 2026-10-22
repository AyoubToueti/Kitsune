//! Metadata providers.
//!
//! Each source implements [`traits::AnimeProvider`], so the rest of the app
//! depends on the trait rather than a specific API.

pub mod anilist;
pub mod cache;
pub mod failover;
pub mod jikan;
pub mod kitsu;
pub mod traits;

pub use anilist::{AniListProvider, ANILIST_ENDPOINT};
pub use cache::{CachedAccount, CachedProvider};
pub use failover::FailoverProvider;
pub use jikan::{JikanProvider, JIKAN_ENDPOINT};
pub use kitsu::{KitsuProvider, KITSU_ENDPOINT};
pub use traits::{AnimeProvider, ProviderError};