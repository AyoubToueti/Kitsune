//! Pure magnet-link parsing.
//!
//! Kept free of any `librqbit`/network types so it can be tested directly
//! without starting a torrent session.

use crate::types::MagnetLink;

/// Why a magnet URI could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MagnetError {
    /// The string does not start with `magnet:?`.
    NotAMagnet,
    /// The `xt` parameter is missing.
    MissingExactTopic,
    /// The `xt` parameter is present but not a BitTorrent v1 hash.
    NotABitTorrentHash,
    /// The hash is present but not 40 hex characters.
    MalformedInfoHash,
}

impl std::fmt::Display for MagnetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAMagnet => write!(f, "not a magnet URI (expected 'magnet:?')"),
            Self::MissingExactTopic => write!(f, "magnet URI has no 'xt' parameter"),
            Self::NotABitTorrentHash => {
                write!(f, "'xt' is not a BitTorrent v1 hash (expected urn:btih:)")
            }
            Self::MalformedInfoHash => write!(f, "info hash must be 40 hex characters"),
        }
    }
}

impl std::error::Error for MagnetError {}

/// Extract the BitTorrent v1 info hash from a magnet URI.
///
/// Returns the hash lowercased, so equal torrents compare equal regardless
/// of the casing used by the indexer that produced the link.
///
/// Only `urn:btih:` (v1) is supported. `urn:btmh:` (v2) is deliberately not
/// accepted: the torrent engine and trackers in use are v1.
pub fn parse_info_hash(uri: &str) -> Result<String, MagnetError> {
    if !uri.starts_with("magnet:?") {
        return Err(MagnetError::NotAMagnet);
    }

    let query = &uri["magnet:?".len()..];

    // Hand-rolled splitting rather than pulling in a URL/query parser: the
    // only parameter we need is `xt`, and magnet URIs in the wild contain
    // unencoded characters (spaces, brackets) that stricter parsers reject.
    let xt = query
        .split('&')
        .find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key == "xt").then_some(value)
        })
        .ok_or(MagnetError::MissingExactTopic)?;

    let hash = xt
        .strip_prefix("urn:btih:")
        .ok_or(MagnetError::NotABitTorrentHash)?;

    // Reject anything that is not exactly 40 ASCII hex characters. Base32
    // infohashes exist in the wild but are not produced by the indexers used
    // here, and accepting them silently would risk sending a bad request to
    // the torrent engine.
    if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(MagnetError::MalformedInfoHash);
    }

    Ok(hash.to_ascii_lowercase())
}

/// Extract the `dn` (display name) parameter, if present.
///
/// The value is percent-decoded for the common cases (`%20` etc.); anything
/// more exotic is left as-is rather than failing the whole parse, since the
/// display name is cosmetic.
pub fn parse_display_name(uri: &str) -> Option<String> {
    let query = uri.strip_prefix("magnet:?")?;

    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key != "dn" {
            return None;
        }
        let cleaned = value.replace('+', " ");
        Some(percent_decode(&cleaned))
    })
}

/// Minimal percent-decoding for `dn` values.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }

    // Lossy on purpose: a stray non-UTF8 byte should not discard the name.
    String::from_utf8_lossy(&out).into_owned()
}

/// Build a [`MagnetLink`] from a raw URI and whatever metadata the indexer
/// supplied alongside it.
///
/// The info hash is parsed from the URI rather than trusted from the caller,
/// so the two can never disagree.
pub fn build_magnet(
    uri: &str,
    title: Option<String>,
    size_bytes: Option<u64>,
    seeders: Option<u32>,
    leechers: Option<u32>,
) -> Result<MagnetLink, MagnetError> {
    let info_hash = parse_info_hash(uri)?;

    // Prefer the indexer's title; fall back to the magnet's own display name.
    let title = title.or_else(|| parse_display_name(uri));

    Ok(MagnetLink {
        uri: uri.to_string(),
        info_hash: Some(info_hash),
        title,
        size_bytes,
        seeders,
        leechers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "cab507494d02ebb1178b38f2e9d7be299c86b862";
    const VALID: &str = "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862";

    #[test]
    fn parses_a_plain_info_hash() {
        assert_eq!(parse_info_hash(VALID).unwrap(), HASH);
    }

    #[test]
    fn lowercases_an_uppercase_hash() {
        let uri = format!("magnet:?xt=urn:btih:{}", HASH.to_uppercase());
        assert_eq!(parse_info_hash(&uri).unwrap(), HASH);
    }

    #[test]
    fn finds_xt_among_other_parameters() {
        let uri = format!(
            "magnet:?dn=Some+Release&xt=urn:btih:{HASH}&tr=udp%3A%2F%2Ftracker.test%3A80"
        );
        assert_eq!(parse_info_hash(&uri).unwrap(), HASH);
    }

    #[test]
    fn rejects_a_non_magnet_string() {
        assert_eq!(
            parse_info_hash("https://example.test/file.torrent"),
            Err(MagnetError::NotAMagnet)
        );
    }

    #[test]
    fn rejects_a_missing_xt() {
        assert_eq!(
            parse_info_hash("magnet:?dn=No+Hash+Here"),
            Err(MagnetError::MissingExactTopic)
        );
    }

    #[test]
    fn rejects_a_v2_hash() {
        // urn:btmh: is BitTorrent v2 and intentionally unsupported.
        let uri = format!("magnet:?xt=urn:btmh:{HASH}");
        assert_eq!(
            parse_info_hash(&uri),
            Err(MagnetError::NotABitTorrentHash)
        );
    }

    #[test]
    fn rejects_a_short_hash() {
        assert_eq!(
            parse_info_hash("magnet:?xt=urn:btih:abc123"),
            Err(MagnetError::MalformedInfoHash)
        );
    }

    #[test]
    fn rejects_a_non_hex_hash() {
        let bad = "z".repeat(40);
        let uri = format!("magnet:?xt=urn:btih:{bad}");
        assert_eq!(parse_info_hash(&uri), Err(MagnetError::MalformedInfoHash));
    }

    #[test]
    fn parses_a_percent_encoded_display_name() {
        let uri = format!("magnet:?xt=urn:btih:{HASH}&dn=%5BSubsPlease%5D+Show+-+01");
        assert_eq!(
            parse_display_name(&uri).as_deref(),
            Some("[SubsPlease] Show - 01")
        );
    }

    #[test]
    fn display_name_is_none_when_absent() {
        assert_eq!(parse_display_name(VALID), None);
    }

    #[test]
    fn build_magnet_prefers_the_supplied_title() {
        let magnet = build_magnet(
            VALID,
            Some("Indexer Title".into()),
            Some(1234),
            Some(10),
            Some(2),
        )
        .unwrap();

        assert_eq!(magnet.title.as_deref(), Some("Indexer Title"));
        assert_eq!(magnet.info_hash.as_deref(), Some(HASH));
        assert_eq!(magnet.size_bytes, Some(1234));
        assert_eq!(magnet.seeders, Some(10));
        assert_eq!(magnet.leechers, Some(2));
    }

    #[test]
    fn build_magnet_falls_back_to_the_display_name() {
        let uri = format!("magnet:?xt=urn:btih:{HASH}&dn=Fallback+Name");
        let magnet = build_magnet(&uri, None, None, None, None).unwrap();
        assert_eq!(magnet.title.as_deref(), Some("Fallback Name"));
    }

    #[test]
    fn build_magnet_propagates_parse_errors() {
        assert_eq!(
            build_magnet("not-a-magnet", None, None, None, None),
            Err(MagnetError::NotAMagnet)
        );
    }
}