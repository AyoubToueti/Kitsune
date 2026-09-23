//! Signing in to AniList.
//!
//! AniList requires OAuth for anything that touches a reader's own list:
//! queries are public, but mutations need a bearer token. This module owns the
//! token's lifetime -- storing it, handing it to the provider, and clearing it
//! on sign-out.
//!
//! The interactive part of the flow (opening the authorise URL and catching the
//! redirect) is deliberately NOT here yet: it needs the deep-link plugin, which
//! is a separate change. What exists now is the storage the rest of the flow
//! will build on, so it can be tested without a network or a browser.

pub mod commands;
pub mod flow;
pub mod list;
pub mod store;

pub use commands::{apply_token, restore_token, SharedAniList, AUTH_CHANGED_EVENT};
pub use flow::{authorize_url, parse_token, CLIENT_ID};
pub use store::TokenStore;