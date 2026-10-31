//! Signing in to AniList.
//!
//! AniList requires OAuth for anything that touches a reader's own list:
//! queries are public, but mutations need a bearer token. This module owns the
//! token's lifetime -- storing it, handing it to the provider, and clearing it
//! on sign-out.
//!
//! Only the storage/flow parts live here. The Tauri command wrappers and the
//! list mutations (`commands`, `list`) stay in the frontend, since they read
//! `State` and emit events.

pub mod flow;
pub mod last_played;
pub mod store;

pub use flow::{authorize_url, parse_token, CLIENT_ID};
pub use last_played::{LastPlayed, LastPlayedStore};
pub use store::TokenStore;