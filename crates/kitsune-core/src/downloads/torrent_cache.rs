//! Cached `.torrent` metadata for restart-safe downloads.
//!
//! A magnet can always be re-resolved, but doing that after a restart depends
//! on finding peers before anything can be selected or resumed. The metadata is
//! tiny, so it is kept beside the download store and preferred when reattaching.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// Default root for cached `.torrent` files.
pub fn default_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("kitsune")
        .join("torrent-cache")
}

/// Validate a v1 info hash before it becomes a filename.
pub fn valid_info_hash(info_hash: &str) -> bool {
    info_hash.len() == 40 && info_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Path for `info_hash` under `root`.
///
/// The hash is validated rather than sanitized: every caller already has one
/// from `parse_info_hash`, and accepting anything else would turn a corrupted
/// store into a path-traversal primitive.
pub fn path_in(root: &Path, info_hash: &str) -> Result<PathBuf> {
    if !valid_info_hash(info_hash) {
        bail!("invalid info hash for torrent cache");
    }
    Ok(root.join(format!("{}.torrent", info_hash.to_ascii_lowercase())))
}

/// Per-torrent partial-data directory under `root`.
pub fn data_dir_in(root: &Path, info_hash: &str) -> Result<PathBuf> {
    if !valid_info_hash(info_hash) {
        bail!("invalid info hash for download cache");
    }
    Ok(root.join(info_hash.to_ascii_lowercase()))
}

/// Delete a per-torrent partial-data directory.
///
/// The recorded path is only trusted when its final component is the info
/// hash: a corrupted store must not be able to turn cleanup into an arbitrary
/// recursive delete.
pub fn remove_data_dir(data_dir: &Path, info_hash: &str) -> Result<()> {
    if !valid_info_hash(info_hash) {
        bail!("invalid info hash for download cleanup");
    }
    let expected = info_hash.to_ascii_lowercase();
    let actual = data_dir.file_name().and_then(|name| name.to_str());
    if actual != Some(expected.as_str()) {
        bail!(
            "refusing to delete unexpected download cache {}",
            data_dir.display()
        );
    }
    match std::fs::remove_dir_all(data_dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => {
            Err(err).with_context(|| format!("remove download cache {}", data_dir.display()))
        }
    }
}

/// Cache `bytes` for `info_hash`, replacing any previous copy.
pub fn save_in(root: &Path, info_hash: &str, bytes: &[u8]) -> Result<PathBuf> {
    let path = path_in(root, info_hash)?;
    if bytes.is_empty() {
        bail!("refusing to cache an empty torrent file");
    }
    std::fs::create_dir_all(root)
        .with_context(|| format!("create torrent cache {}", root.display()))?;
    let temp = path.with_extension("torrent.tmp");
    std::fs::write(&temp, bytes)
        .with_context(|| format!("write torrent cache {}", temp.display()))?;
    std::fs::rename(&temp, &path)
        .with_context(|| format!("rename torrent cache {}", path.display()))?;
    Ok(path)
}

/// Read cached metadata, or `None` when it is absent or invalid.
pub fn load_in(root: &Path, info_hash: &str) -> Option<Vec<u8>> {
    let path = path_in(root, info_hash).ok()?;
    let bytes = std::fs::read(path).ok()?;
    (!bytes.is_empty()).then_some(bytes)
}

/// Remove cached metadata. Missing files are not an error.
pub fn remove_in(root: &Path, info_hash: &str) -> Result<()> {
    let path = path_in(root, info_hash)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("remove torrent cache {}", path.display())),
    }
}

/// Existing cached metadata path in the default location.
///
/// Returns `None` when the hash is invalid or nothing has been cached, so the
/// caller can fall back to the magnet instead of handing the engine a missing
/// path.
pub fn cached_path(info_hash: &str) -> Option<PathBuf> {
    let path = path_in(&default_root(), info_hash).ok()?;
    path.is_file().then_some(path)
}

/// Cache metadata in the default app-data location.
pub fn save(info_hash: &str, bytes: &[u8]) -> Result<PathBuf> {
    save_in(&default_root(), info_hash, bytes)
}

/// Read metadata from the default app-data location.
pub fn load(info_hash: &str) -> Option<Vec<u8>> {
    load_in(&default_root(), info_hash)
}

/// Remove metadata from the default app-data location.
pub fn remove(info_hash: &str) -> Result<()> {
    remove_in(&default_root(), info_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const HASH: &str = "cab507494d02ebb1178b38f2e9d7be299c86b862";

    #[test]
    fn round_trips_metadata() {
        let dir = TempDir::new().unwrap();
        save_in(dir.path(), HASH, b"torrent bytes").unwrap();
        assert_eq!(
            load_in(dir.path(), HASH).as_deref(),
            Some(&b"torrent bytes"[..])
        );
        remove_in(dir.path(), HASH).unwrap();
        assert!(load_in(dir.path(), HASH).is_none());
    }

    #[test]
    fn rejects_non_hash_filenames() {
        let dir = TempDir::new().unwrap();
        assert!(path_in(dir.path(), "../../etc/passwd").is_err());
        assert!(path_in(dir.path(), "not-a-hash").is_err());
    }

    #[test]
    fn refuses_empty_metadata() {
        let dir = TempDir::new().unwrap();
        assert!(save_in(dir.path(), HASH, b"").is_err());
    }

    #[test]
    fn missing_removal_is_ok() {
        let dir = TempDir::new().unwrap();
        remove_in(dir.path(), HASH).unwrap();
    }

    #[test]
    fn removal_guards_the_data_directory_name() {
        let dir = TempDir::new().unwrap();
        let data = data_dir_in(dir.path(), HASH).unwrap();
        std::fs::create_dir_all(data.join("Show")).unwrap();
        std::fs::write(data.join("Show").join("ep.mkv"), b"x").unwrap();

        remove_data_dir(&data, HASH).unwrap();
        assert!(!data.exists());
        assert!(remove_data_dir(dir.path(), HASH).is_err());
        assert!(remove_data_dir(&data, "bad").is_err());
    }
}
