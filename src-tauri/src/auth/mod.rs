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

pub mod store;

pub use store::TokenStore;