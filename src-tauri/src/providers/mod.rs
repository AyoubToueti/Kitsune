//! Metadata providers.
//!
//! Each source implements [`traits::AnimeProvider`], so the rest of the app
//! depends on the trait rather than a specific API.

pub mod anilist;
pub mod traits;

pub use anilist::{AniListProvider, ANILIST_ENDPOINT};
pub use traits::{AnimeProvider, ProviderError};