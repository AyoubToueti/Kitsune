//! The indexer abstraction.
//!
//! An indexer finds candidate releases for a query. It is deliberately
//! separate from [`crate::providers::AnimeProvider`]: a metadata provider
//! describes a work, an indexer finds copies of it, and the two have nothing
//! in common beyond both being remote.
//!
//! The trait is async and object-safe so the app can hold several indexers and
//! treat them interchangeably, exactly as it does providers.

use async_trait::async_trait;

use crate::types::Release;

/// Why an indexer call failed.
///
/// Mirrors [`crate::providers::ProviderError`] so the frontend can render both
/// the same way, but kept distinct because the causes differ: an indexer has
/// no GraphQL envelope to report errors in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexerError {
    /// The request never completed (DNS, connect, timeout).
    Transport(String),
    /// The server answered with a non-success status.
    Status { status: u16, body: String },
    /// The response was not the shape we expect.
    Decode(String),
    /// The query itself was rejected before any request was made.
    InvalidQuery(String),
}

impl std::fmt::Display for IndexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(msg) => write!(f, "indexer request failed: {msg}"),
            Self::Status { status, body } => {
                write!(f, "indexer returned HTTP {status}: {body}")
            }
            Self::Decode(msg) => write!(f, "could not decode indexer response: {msg}"),
            Self::InvalidQuery(msg) => write!(f, "invalid search query: {msg}"),
        }
    }
}

impl std::error::Error for IndexerError {}

/// A source of release candidates.
#[async_trait]
pub trait Indexer: Send + Sync {
    /// A short name for logs and errors, e.g. "nyaa".
    fn name(&self) -> &str;

    /// Find releases matching a free-text query.
    ///
    /// The query is the work's title, optionally with an episode number; the
    /// indexer does not parse it, it just searches. Filtering and ranking are
    /// the caller's job, so an indexer stays a thin transport.
    async fn search(&self, query: &str) -> Result<Vec<Release>, IndexerError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_variant() {
        let cases = [
            IndexerError::Transport("timeout".into()),
            IndexerError::Status {
                status: 503,
                body: "unavailable".into(),
            },
            IndexerError::Decode("bad xml".into()),
            IndexerError::InvalidQuery("empty".into()),
        ];

        for case in cases {
            assert!(!case.to_string().trim().is_empty(), "empty display for {case:?}");
        }
    }

    #[test]
    fn status_includes_the_code() {
        let err = IndexerError::Status {
            status: 404,
            body: String::new(),
        };
        assert!(err.to_string().contains("404"));
    }

    #[test]
    fn indexer_error_is_a_std_error() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<IndexerError>();
    }
}