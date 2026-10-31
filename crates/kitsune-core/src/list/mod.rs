//! The reader's media list, local-first.
//!
//! [`store::MediaListStore`] is the durable record; sync against AniList is a
//! separate concern. Splitting them keeps the storage rules testable without a
//! network, and keeps the sync engine from reaching into the file format.

pub mod store;
pub mod sync;

pub use store::{now_seconds, MediaListRecord, MediaListStore};
pub use sync::{local_change, ListSync, SyncReport};

/// A minimal work for tests.
///
/// `Anime` has many fields and a list test cares about none of them, so building
/// one inline in every test would bury the assertion. Compiled only for tests so
/// it cannot drift into a production path.
#[cfg(test)]
pub(crate) fn test_anime(provider: crate::types::ProviderId, id: i64) -> crate::types::Anime {
    crate::types::Anime {
        id,
        provider,
        title: crate::types::Title::default(),
        id_mal: None,
        cover_image: None,
        banner_image: None,
        description: None,
        episode_count: None,
        aired_episodes: None,
        next_airing: None,
        duration_minutes: None,
        format: None,
        genres: Vec::new(),
        average_score: None,
        popularity: None,
        status: None,
        season: None,
        season_year: None,
        streaming_episodes: Vec::new(),
        relations: Vec::new(),
        recommendations: Vec::new(),
        trailer: None,
    }
}