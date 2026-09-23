//! Where the AniList access token is kept between runs.
//!
//! A plain JSON file in the OS data directory, read and written by Rust only.
//! The token is never exposed to the webview: nothing in the frontend needs to
//! see it, and a plugin that mirrors it into JS would be a larger surface for
//! no benefit.
//!
//! The token is a bearer credential for the reader's AniList account, so the
//! file is written 0600 on Unix. That is not encryption -- anyone who can read
//! the user's home directory can read it -- but it keeps the token out of the
//! hands of other users on a shared machine, which is the realistic threat for
//! a desktop app.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// The on-disk shape.
///
/// An object rather than a bare string so a later field (a refresh token, an
/// expiry, the account name) can be added without a migration. AniList tokens
/// last a year with no refresh, so none of those are needed yet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

/// Reads and writes the AniList access token.
pub struct TokenStore {
    path: PathBuf,
}

impl TokenStore {
    /// The store at the default location for this platform.
    ///
    /// Falls back to the temp directory when the OS reports no data directory,
    /// which keeps the app working rather than panicking on an unusual
    /// environment. The token then simply does not survive a reboot.
    pub fn new() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("kitsune")
            .join("auth.json");

        Self { path }
    }

    /// A store at an explicit path, for tests.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The token, or `None` when nobody has signed in.
    ///
    /// A missing file, unreadable file, or malformed JSON all mean "no token"
    /// rather than an error. There is nothing a caller could usefully do about
    /// a corrupt credential file except sign in again, and treating it as an
    /// error would surface a parse failure to the reader for no reason.
    pub fn read(&self) -> Option<String> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        let stored: Stored = serde_json::from_str(&text).ok()?;
        let token = stored.token?;

        // A blank string is not a credential, and passing it on would make
        // every request 401 instead of prompting a sign-in.
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return None;
        }

        Some(trimmed.to_string())
    }

    /// Whether a token is stored.
    pub fn has_token(&self) -> bool {
        self.read().is_some()
    }

    /// Store a token, replacing any existing one.
    pub fn write(&self, token: &str) -> Result<()> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            anyhow::bail!("refusing to store a blank token");
        }

        self.write_bytes(&Stored {
            token: Some(trimmed.to_string()),
        })
    }

    /// Forget the stored token.
    ///
    /// Deleting the file rather than writing an empty object: an absent file
    /// and a file with no token mean the same thing to [`Self::read`], and
    /// removing the credential from disk is the honest way to sign out.
    ///
    /// A missing file is success, so signing out twice is harmless.
    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err)
                .with_context(|| format!("failed to remove {}", self.path.display())),
        }
    }

    /// Serialise and write, creating the directory if needed.
    ///
    /// Written to a sibling temp file and renamed into place, so a crash
    /// mid-write cannot leave a half-written credential file behind. Rename is
    /// atomic within a directory on every platform this targets.
    fn write_bytes(&self, stored: &Stored) -> Result<()> {
        let dir = self
            .path
            .parent()
            .context("token store path has no parent directory")?;

        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let json = serde_json::to_string_pretty(stored).context("failed to encode token")?;

        let temp = self.path.with_extension("json.tmp");
        std::fs::write(&temp, json)
            .with_context(|| format!("failed to write {}", temp.display()))?;

        restrict_permissions(&temp);

        std::fs::rename(&temp, &self.path).with_context(|| {
            format!(
                "failed to move {} into {}",
                temp.display(),
                self.path.display()
            )
        })?;

        Ok(())
    }

    /// Path the store reads and writes, for tests and diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Make a file readable only by its owner, on platforms that have the notion.
///
/// Best effort: a failure here is not worth failing a sign-in over, and on
/// Windows the default ACL already scopes the user's own profile.
#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, TokenStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = TokenStore::at(dir.path().join("nested").join("auth.json"));
        (dir, store)
    }

    #[test]
    fn a_fresh_store_has_no_token() {
        let (_dir, store) = store();

        assert_eq!(store.read(), None);
        assert!(!store.has_token());
    }

    #[test]
    fn a_token_round_trips() {
        let (_dir, store) = store();

        store.write("secret-token").expect("write");

        assert_eq!(store.read().as_deref(), Some("secret-token"));
        assert!(store.has_token());
    }

    #[test]
    fn writing_creates_missing_directories() {
        let (_dir, store) = store();

        // The store path is two levels below the temp dir, neither of which
        // exists yet.
        store.write("token").expect("write should create parents");

        assert!(store.path().exists());
    }

    #[test]
    fn writing_replaces_an_existing_token() {
        let (_dir, store) = store();

        store.write("first").expect("first write");
        store.write("second").expect("second write");

        assert_eq!(store.read().as_deref(), Some("second"));
    }

    #[test]
    fn a_blank_token_is_refused() {
        let (_dir, store) = store();

        // Storing a blank string would make every request 401 instead of
        // prompting a sign-in, so it is rejected at the boundary.
        assert!(store.write("   ").is_err());
        assert_eq!(store.read(), None);
    }

    #[test]
    fn the_token_is_trimmed() {
        let (_dir, store) = store();

        store.write("  padded  ").expect("write");

        assert_eq!(store.read().as_deref(), Some("padded"));
    }

    #[test]
    fn clear_removes_the_token() {
        let (_dir, store) = store();
        store.write("token").expect("write");

        store.clear().expect("clear");

        assert_eq!(store.read(), None);
    }

    #[test]
    fn clearing_twice_is_not_an_error() {
        let (_dir, store) = store();
        store.write("token").expect("write");
        store.clear().expect("first clear");

        // Signing out twice must not fail; there is nothing left to remove.
        store.clear().expect("second clear");
    }

    #[test]
    fn a_corrupt_file_reads_as_no_token() {
        let (dir, store) = store();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        std::fs::write(store.path(), b"not json at all").unwrap();

        // A damaged credential file means "sign in again", not a crash.
        assert_eq!(store.read(), None);
    }

    #[test]
    fn a_blank_token_in_the_file_reads_as_none() {
        let (dir, store) = store();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        std::fs::write(store.path(), br#"{"token":"  "}"#).unwrap();

        assert_eq!(store.read(), None);
    }

    #[test]
    fn the_file_holds_a_json_object() {
        let (_dir, store) = store();
        store.write("token").expect("write");

        let text = std::fs::read_to_string(store.path()).expect("read");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");

        // An object rather than a bare string, so a later field can be added
        // without a migration.
        assert_eq!(parsed["token"], "token");
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_not_world_readable() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, store) = store();
        store.write("token").expect("write");

        let mode = std::fs::metadata(store.path())
            .expect("metadata")
            .permissions()
            .mode();

        assert_eq!(mode & 0o077, 0, "group and other bits should be clear");
    }
}