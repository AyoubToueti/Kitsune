//! Tauri commands and handlers for signing in to AniList.
//!
//! The interactive part of the flow is two steps: the frontend opens the
//! authorise URL, and the OS later hands the redirect back to this process as a
//! deep link. Only the second step is here, because only it can touch the token
//! store.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::{flow, TokenStore};
use crate::providers::AniListProvider;

/// Event emitted after the token changes, so the UI can re-read its state.
///
/// Carries a bool -- signed in or not -- rather than the token itself. The
/// frontend never sees the credential, which is the point of keeping it in
/// Rust.
pub const AUTH_CHANGED_EVENT: &str = "auth-changed";

/// The concrete provider handle the auth commands need.
///
/// A `Arc<AniListProvider>` rather than the `Arc<dyn AnimeProvider>` the
/// metadata commands use, because setting a token is not part of the provider
/// trait -- Jikan has no token, and adding one would be a method that does
/// nothing for half the implementations. Both handles point at the same
/// instance, so a token set here is used by every metadata call.
pub type SharedAniList = Arc<AniListProvider>;

/// The URL the reader must open to authorise Kitsune.
///
/// Returned rather than opened here: the frontend already holds the opener
/// plugin, and launching a browser is a UI action rather than backend work.
#[tauri::command]
pub fn begin_login() -> String {
    flow::authorize_url()
}

/// Whether a token is stored.
#[tauri::command]
pub fn auth_status(store: State<'_, TokenStore>) -> bool {
    store.has_token()
}

/// Forget the stored token.
///
/// Also clears it from the live provider, so signing out takes effect on the
/// next request rather than at the next restart.
#[tauri::command]
pub fn logout(
    app: AppHandle,
    store: State<'_, TokenStore>,
    provider: State<'_, SharedAniList>,
) -> Result<(), String> {
    store.clear().map_err(|err| err.to_string())?;
    provider.clear_token();
    let _ = app.emit(AUTH_CHANGED_EVENT, false);
    Ok(())
}

/// Take the token out of a redirect URL and start using it.
///
/// Returns whether a token was applied. A URL that is not ours, has no
/// fragment, or carries no usable token is ignored rather than reported: deep
/// links can arrive from anywhere, and a malformed one is not an error the
/// reader needs to see.
///
/// The store is written BEFORE the provider is updated. Holding a token that
/// failed to persist would work until the next restart and then appear to sign
/// the reader out for no reason, which is worse than not signing in at all.
pub fn apply_token(store: &TokenStore, provider: &AniListProvider, url: &str) -> bool {
    let Some(token) = flow::parse_token(url) else {
        return false;
    };

    if let Err(err) = store.write(&token) {
        // The reader cannot act on this, and the fallback -- sign in again --
        // is the same either way, but it should not be silent in the logs.
        tracing::warn!("could not store the AniList token: {err:#}");
        return false;
    }

    provider.set_token(Some(token));
    true
}

/// Load a stored token into the provider at startup.
///
/// Without this a signed-in reader would appear signed out on every launch:
/// the token is on disk but nothing has told the provider about it.
///
/// Returns whether one was found.
pub fn restore_token(store: &TokenStore, provider: &AniListProvider) -> bool {
    match store.read() {
        Some(token) => {
            provider.set_token(Some(token));
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts() -> (tempfile::TempDir, TokenStore, AniListProvider) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = TokenStore::at(dir.path().join("auth.json"));
        let provider = AniListProvider::with_endpoint("http://localhost");
        (dir, store, provider)
    }

    #[test]
    fn a_redirect_applies_its_token() {
        let (_dir, store, provider) = parts();

        let applied = apply_token(
            &store,
            &provider,
            "kitsune://auth#access_token=secret-token",
        );

        assert!(applied);
        assert_eq!(store.read().as_deref(), Some("secret-token"));
        assert!(provider.has_token());
    }

    #[test]
    fn a_redirect_for_another_scheme_is_ignored() {
        let (_dir, store, provider) = parts();

        let applied = apply_token(&store, &provider, "evil://auth#access_token=stolen");

        // Any handler on the machine can send a deep link, so one that is not
        // ours must never set a credential.
        assert!(!applied);
        assert_eq!(store.read(), None);
        assert!(!provider.has_token());
    }

    #[test]
    fn a_redirect_without_a_token_is_ignored() {
        let (_dir, store, provider) = parts();

        assert!(!apply_token(&store, &provider, "kitsune://auth"));
        assert!(!apply_token(&store, &provider, "kitsune://auth#token_type=Bearer"));
        assert!(!provider.has_token());
    }

    /// A later sign-in must replace an earlier one rather than being ignored.
    #[test]
    fn a_second_redirect_replaces_the_first_token() {
        let (_dir, store, provider) = parts();
        apply_token(&store, &provider, "kitsune://auth#access_token=first");

        apply_token(&store, &provider, "kitsune://auth#access_token=second");

        assert_eq!(store.read().as_deref(), Some("second"));
    }

    #[test]
    fn restore_puts_a_stored_token_back_into_the_provider() {
        let (_dir, store, provider) = parts();
        store.write("stored-token").expect("write");

        let restored = restore_token(&store, &provider);

        assert!(restored);
        assert!(provider.has_token());
    }

    #[test]
    fn restore_reports_nothing_when_signed_out() {
        let (_dir, store, provider) = parts();

        assert!(!restore_token(&store, &provider));
        assert!(!provider.has_token());
    }
}