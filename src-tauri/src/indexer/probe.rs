//! Lightweight health probes for a release's swarm.
//!
//! A search returns releases ranked by what the indexer *claimed*: seeder
//! counts read off a web page, which may be minutes or hours stale. A probe
//! asks the torrent's own trackers what is true right now, which is the
//! difference between "Nyaa says 50 seeders" and "a tracker just confirmed 12".
//!
//! The split mirrors the rest of the indexer. Everything that can be decided
//! without touching the network -- pulling tracker URLs out of a magnet,
//! turning a hex info hash into the raw 20 bytes a tracker wants, and decoding
//! a bencoded scrape response -- is pure and unit tested here. The network
//! calls themselves are thin wrappers over these.

use crate::torrent::magnet::percent_decode;

/// Trackers we could not ask, or that answered with something unusable.
///
/// Every variant is recoverable: a probe that fails simply leaves the release
/// ranked on its static score, so the orchestrator logs the reason rather than
/// propagating it to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrapeError {
    /// The tracker reported a `failure reason` instead of stats.
    TrackerFailure(String),
    /// The tracker answered, but not about the torrent we asked about.
    UnknownTorrent,
    /// The response could not be decoded at all.
    Decode(String),
    /// The request never completed (DNS, connect, timeout).
    Transport(String),
}

impl std::fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TrackerFailure(reason) => write!(f, "tracker refused the scrape: {reason}"),
            Self::UnknownTorrent => write!(f, "tracker does not know this torrent"),
            Self::Decode(msg) => write!(f, "could not decode tracker response: {msg}"),
            Self::Transport(msg) => write!(f, "tracker request failed: {msg}"),
        }
    }
}

impl std::error::Error for ScrapeError {}

/// What one tracker reports about a torrent right now.
///
/// Named after the tracker protocol's own fields rather than the torrent
/// vocabulary, because the mapping is the interesting part: a tracker's
/// "complete" peers are the seeders, "incomplete" are the leechers, and
/// "downloaded" counts completed downloads (not current peers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrapeStats {
    /// Peers with the whole torrent: `complete`.
    pub seeders: u32,
    /// Peers still downloading: `incomplete`.
    pub leechers: u32,
    /// Times the torrent was finished: `downloaded`.
    pub completed: u32,
}

/// Every `tr` value in a magnet URI, in order, with duplicates removed.
///
/// A magnet that is not a magnet, or that carries no `tr` parameter, yields an
/// empty list rather than an error. That is the honest answer -- "we have no
/// tracker to ask" -- and it lets the caller fall back to the metadata probe
/// without special-casing a malformed link that the indexer layer already
/// validated.
pub fn extract_trackers(magnet_uri: &str) -> Vec<String> {
    let Some(query) = magnet_uri.strip_prefix("magnet:?") else {
        return Vec::new();
    };

    let mut trackers: Vec<String> = Vec::new();

    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key != "tr" {
            continue;
        }

        // Magnets percent-encode the tracker URL (`udp%3A%2F%2F...`), so it
        // has to be decoded before it can be dialled.
        let decoded = percent_decode(value);
        let trimmed = decoded.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Deduplicate: the same tracker is often listed twice, and asking it
        // twice only doubles the latency of the slowest tracker.
        if !trackers.iter().any(|existing| existing == trimmed) {
            trackers.push(trimmed.to_string());
        }
    }

    trackers
}

/// Turn a 40-character hex info hash into the raw 20 bytes trackers expect.
///
/// Trackers do not take the hex form: both the UDP and HTTP scrape protocols
/// carry the info hash as 20 raw bytes (percent-encoded in the HTTP case).
/// Taking the hex form rather than bytes keeps the caller's type honest -- a
/// `Release` stores its hash as the hex string a magnet spells.
pub fn decode_info_hash(hex: &str) -> Result<[u8; 20], ScrapeError> {
    if hex.len() != 40 {
        return Err(ScrapeError::Decode(format!(
            "info hash must be 40 hex characters, got {}",
            hex.len()
        )));
    }

    let mut bytes = [0u8; 20];
    for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk)
            .map_err(|_| ScrapeError::Decode("info hash is not ASCII".into()))?;
        bytes[index] = u8::from_str_radix(text, 16)
            .map_err(|_| ScrapeError::Decode(format!("'{text}' is not a hex byte")))?;
    }

    Ok(bytes)
}

/// Percent-encode raw bytes for a query-string value, per RFC 3986.
///
/// The HTTP scrape protocol wants the info hash as raw bytes escaped this way,
/// which is why it cannot reuse the hex form: `%00` and `00` are different
/// requests. Only the unreserved set survives unescaped.
pub fn percent_encode_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Why a bencoded buffer could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BencodeError {
    /// The buffer ended in the middle of a value.
    UnexpectedEnd,
    /// A byte appeared where a value or a delimiter was expected.
    UnexpectedByte(u8),
    /// An integer had no digits, or had a stray sign.
    InvalidInteger,
    /// A byte-string length was missing or unparseable.
    InvalidLength,
    /// Nesting went deeper than [`MAX_DEPTH`], so parsing was abandoned
    /// rather than risking a stack overflow on a hostile response.
    DepthExceeded,
    /// Parsing finished with bytes left over, so the value is not the whole
    /// buffer and the caller would be reading a truncated view.
    TrailingData(usize),
}

impl std::fmt::Display for BencodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEnd => write!(f, "bencode ended mid-value"),
            Self::UnexpectedByte(byte) => write!(f, "unexpected byte 0x{byte:02X}"),
            Self::InvalidInteger => write!(f, "malformed bencode integer"),
            Self::InvalidLength => write!(f, "malformed bencode string length"),
            Self::DepthExceeded => write!(f, "bencode nested too deeply"),
            Self::TrailingData(at) => write!(f, "{at} trailing bytes after the value"),
        }
    }
}

impl std::error::Error for BencodeError {}

/// How deep a bencoded value may nest before it is rejected.
///
/// A scrape response is three levels deep (`dict -> files -> stats`), so this
/// is far above anything legitimate. The limit exists because the parser
/// recurses, and a tracker -- or something impersonating one -- should not be
/// able to exhaust the stack with a buffer of `d` bytes.
pub const MAX_DEPTH: usize = 32;

/// A decoded bencode value.
///
/// Dict keys are byte strings rather than `String`: a scrape response is keyed
/// by the raw 20-byte info hash, which is not valid UTF-8 in general.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bencode {
    Int(i64),
    Bytes(Vec<u8>),
    List(Vec<Bencode>),
    /// Insertion-ordered pairs. A linear scan is fine at this size and avoids
    /// requiring `Ord` on the byte-string keys.
    Dict(Vec<(Vec<u8>, Bencode)>),
}

impl Bencode {
    /// This value as an integer, if it is one.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// This value as raw bytes, if it is a byte string.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(value) => Some(value),
            _ => None,
        }
    }

    /// Look up a key in a dictionary.
    ///
    /// `None` for a non-dictionary, or for a key that is absent; a caller that
    /// must distinguish the two should match on the variant first.
    pub fn get(&self, key: &[u8]) -> Option<&Bencode> {
        match self {
            Self::Dict(pairs) => pairs
                .iter()
                .find(|(candidate, _)| candidate.as_slice() == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

/// Parse a complete bencoded buffer.
///
/// Rejects trailing bytes: a scrape response is exactly one value, so anything
/// after it means the buffer is not what it claims to be.
pub fn parse_bencode(input: &[u8]) -> Result<Bencode, BencodeError> {
    let mut cursor = 0usize;
    let value = parse_value(input, &mut cursor, 0)?;

    if cursor != input.len() {
        return Err(BencodeError::TrailingData(input.len() - cursor));
    }

    Ok(value)
}

/// Parse one value starting at `cursor`, advancing it past the value.
fn parse_value(
    input: &[u8],
    cursor: &mut usize,
    depth: usize,
) -> Result<Bencode, BencodeError> {
    if depth > MAX_DEPTH {
        return Err(BencodeError::DepthExceeded);
    }

    let byte = *input.get(*cursor).ok_or(BencodeError::UnexpectedEnd)?;

    match byte {
        b'i' => parse_int(input, cursor),
        b'l' => parse_list(input, cursor, depth),
        b'd' => parse_dict(input, cursor, depth),
        b'0'..=b'9' => parse_bytes(input, cursor),
        other => Err(BencodeError::UnexpectedByte(other)),
    }
}

/// Parse `<digits>:<bytes>`.
fn parse_bytes(input: &[u8], cursor: &mut usize) -> Result<Bencode, BencodeError> {
    Ok(Bencode::Bytes(parse_byte_string(input, cursor)?))
}

/// Parse `<digits>:<bytes>` into the raw bytes, for callers that know a byte
/// string is what they expect.
fn parse_byte_string(input: &[u8], cursor: &mut usize) -> Result<Vec<u8>, BencodeError> {
    let length = parse_length(input, cursor)?;

    // `length` is bounded by what the buffer can hold, so the addition cannot
    // overflow on a 64-bit target; the check is still written with checked
    // arithmetic because the bound is what guarantees it.
    let start = *cursor;
    let end = start
        .checked_add(length)
        .ok_or(BencodeError::InvalidLength)?;

    let slice = input.get(start..end).ok_or(BencodeError::UnexpectedEnd)?;
    *cursor = end;

    Ok(slice.to_vec())
}

/// Read a decimal length followed by the `:` separator.
fn parse_length(input: &[u8], cursor: &mut usize) -> Result<usize, BencodeError> {
    let start = *cursor;
    let mut end = start;

    while let Some(byte) = input.get(end) {
        if byte.is_ascii_digit() {
            end += 1;
            continue;
        }
        break;
    }

    if end == start {
        return Err(BencodeError::InvalidLength);
    }

    // Bencode forbids leading zeros ("01:") since the length is canonical.
    if end - start > 1 && input[start] == b'0' {
        return Err(BencodeError::InvalidLength);
    }

    if input.get(end) != Some(&b':') {
        return Err(BencodeError::InvalidLength);
    }

    let digits = std::str::from_utf8(&input[start..end])
        .map_err(|_| BencodeError::InvalidLength)?;
    let length = digits
        .parse::<usize>()
        .map_err(|_| BencodeError::InvalidLength)?;

    *cursor = end + 1;
    Ok(length)
}

/// Parse `i<digits>e`.
fn parse_int(input: &[u8], cursor: &mut usize) -> Result<Bencode, BencodeError> {
    *cursor += 1; // consume 'i'

    let start = *cursor;
    let mut end = start;

    if input.get(end) == Some(&b'-') {
        end += 1;
    }

    let digits_start = end;
    while let Some(byte) = input.get(end) {
        if byte.is_ascii_digit() {
            end += 1;
            continue;
        }
        break;
    }

    if end == digits_start {
        return Err(BencodeError::InvalidInteger);
    }

    if input.get(end) != Some(&b'e') {
        return Err(BencodeError::InvalidInteger);
    }

    let text = std::str::from_utf8(&input[start..end])
        .map_err(|_| BencodeError::InvalidInteger)?;
    let value = text
        .parse::<i64>()
        .map_err(|_| BencodeError::InvalidInteger)?;

    *cursor = end + 1;
    Ok(Bencode::Int(value))
}

/// Parse `l<value>...e`.
fn parse_list(
    input: &[u8],
    cursor: &mut usize,
    depth: usize,
) -> Result<Bencode, BencodeError> {
    *cursor += 1; // consume 'l'

    let mut items = Vec::new();
    loop {
        match input.get(*cursor) {
            None => return Err(BencodeError::UnexpectedEnd),
            Some(b'e') => {
                *cursor += 1;
                return Ok(Bencode::List(items));
            }
            Some(_) => items.push(parse_value(input, cursor, depth + 1)?),
        }
    }
}

/// Parse `d<key><value>...e`, where every key is a byte string.
fn parse_dict(
    input: &[u8],
    cursor: &mut usize,
    depth: usize,
) -> Result<Bencode, BencodeError> {
    *cursor += 1; // consume 'd'

    let mut pairs = Vec::new();
    loop {
        match input.get(*cursor) {
            None => return Err(BencodeError::UnexpectedEnd),
            Some(b'e') => {
                *cursor += 1;
                return Ok(Bencode::Dict(pairs));
            }
            Some(b'0'..=b'9') => {
                // A dictionary key is always a byte string, so parsing it
                // directly as one avoids re-inspecting the result.
                let key = parse_byte_string(input, cursor)?;
                let value = parse_value(input, cursor, depth + 1)?;
                pairs.push((key, value));
            }
            Some(other) => return Err(BencodeError::UnexpectedByte(*other)),
        }
    }
}

/// Read the tracker's stats for one torrent out of a scrape response.
///
/// A response that names a `failure reason` is an error, not an empty result:
/// "the tracker says no" and "the tracker says zero seeders" are different
/// facts, and conflating them would mark a healthy release dead.
///
/// Likewise, a torrent the tracker does not list is [`ScrapeError::UnknownTorrent`]
/// rather than a zero-seeder result, for the same reason.
pub fn scrape_stats_from_bencode(
    response: &[u8],
    info_hash: &[u8; 20],
) -> Result<ScrapeStats, ScrapeError> {
    let root = parse_bencode(response).map_err(|e| ScrapeError::Decode(e.to_string()))?;

    if let Some(reason) = root.get(b"failure reason").and_then(Bencode::as_bytes) {
        let reason = String::from_utf8_lossy(reason).into_owned();
        return Err(ScrapeError::TrackerFailure(reason));
    }

    // The canonical shape nests the per-torrent dicts under `files`. Some
    // trackers answer a single-hash scrape with that dict at the top level, so
    // fall back to the root when there is no `files` key.
    let files = match root.get(b"files") {
        Some(files) => files,
        None => &root,
    };

    let stats = files
        .get(info_hash)
        .ok_or(ScrapeError::UnknownTorrent)?;

    Ok(ScrapeStats {
        seeders: count(stats.get(b"complete")),
        leechers: count(stats.get(b"incomplete")),
        completed: count(stats.get(b"downloaded")),
    })
}

/// Read a bencode integer as an unsigned count.
///
/// A negative or oversized value is treated as zero rather than failing the
/// scrape: the count is advisory, and dropping a working torrent because a
/// tracker reported `-1` would be the worse outcome.
fn count(value: Option<&Bencode>) -> u32 {
    value
        .and_then(Bencode::as_int)
        .and_then(|raw| u32::try_from(raw).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "cab507494d02ebb1178b38f2e9d7be299c86b862";

    fn hash_bytes() -> [u8; 20] {
        decode_info_hash(HASH).expect("valid test hash")
    }

    // --- extract_trackers -------------------------------------------------

    #[test]
    fn extracts_a_single_tracker() {
        let uri = format!(
            "magnet:?xt=urn:btih:{HASH}&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce"
        );

        assert_eq!(
            extract_trackers(&uri),
            vec!["udp://tracker.opentrackr.org:1337/announce"]
        );
    }

    #[test]
    fn extracts_trackers_in_order() {
        let uri = format!(
            "magnet:?xt=urn:btih:{HASH}&tr=udp%3A%2F%2Fa.test%3A80&tr=udp%3A%2F%2Fb.test%3A80"
        );

        assert_eq!(
            extract_trackers(&uri),
            vec!["udp://a.test:80", "udp://b.test:80"]
        );
    }

    #[test]
    fn removes_duplicate_trackers() {
        let uri = format!(
            "magnet:?xt=urn:btih:{HASH}&tr=udp%3A%2F%2Fa.test%3A80&tr=udp%3A%2F%2Fa.test%3A80"
        );

        assert_eq!(extract_trackers(&uri), vec!["udp://a.test:80"]);
    }

    #[test]
    fn a_magnet_without_trackers_yields_nothing() {
        assert!(extract_trackers(&format!("magnet:?xt=urn:btih:{HASH}")).is_empty());
    }

    #[test]
    fn a_non_magnet_yields_nothing() {
        assert!(extract_trackers("https://nyaa.si/view/1").is_empty());
    }

    #[test]
    fn a_blank_tracker_is_skipped() {
        let uri = format!("magnet:?xt=urn:btih:{HASH}&tr=&tr=udp%3A%2F%2Fa.test%3A80");
        assert_eq!(extract_trackers(&uri), vec!["udp://a.test:80"]);
    }

    #[test]
    fn a_parameter_that_is_not_tr_is_ignored() {
        let uri = format!("magnet:?xt=urn:btih:{HASH}&dn=Not+A+Tracker");
        assert!(extract_trackers(&uri).is_empty());
    }

    // --- decode_info_hash / percent_encode_bytes --------------------------

    #[test]
    fn decodes_a_lowercase_hex_hash() {
        let bytes = hash_bytes();
        assert_eq!(bytes.len(), 20);
        assert_eq!(bytes[0], 0xCA);
        assert_eq!(bytes[19], 0x62);
    }

    #[test]
    fn decodes_an_uppercase_hex_hash() {
        assert_eq!(
            decode_info_hash(&HASH.to_uppercase()).unwrap(),
            hash_bytes()
        );
    }

    #[test]
    fn rejects_a_hash_of_the_wrong_length() {
        assert!(matches!(
            decode_info_hash("abc123"),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn rejects_a_non_hex_hash() {
        let bad = "z".repeat(40);
        assert!(matches!(
            decode_info_hash(&bad),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn percent_encoding_escapes_every_byte_outside_the_unreserved_set() {
        // A raw info hash is not printable text, so almost every byte must be
        // escaped; this is exactly why the hex form cannot be sent.
        assert_eq!(
            percent_encode_bytes(&[0x00, 0xAB, 0xFF]),
            "%00%AB%FF"
        );
    }

    #[test]
    fn percent_encoding_leaves_unreserved_characters_alone() {
        assert_eq!(percent_encode_bytes(b"azAZ09-_.~"), "azAZ09-_.~");
    }

    #[test]
    fn percent_encoding_escapes_only_what_it_must() {
        let encoded = percent_encode_bytes(&hash_bytes());
        // Three of this hash's twenty bytes fall inside the unreserved set
        // and survive unescaped: 0x49 'I', 0x4D 'M', 0x38 '8' and 0x62 'b'.
        // The rest must be escaped, which is exactly why the hex form cannot
        // be sent in place of the raw bytes.
        assert_eq!(
            encoded,
            "%CA%B5%07IM%02%EB%B1%17%8B8%F2%E9%D7%BE%29%9C%86%B8b"
        );
        assert!(encoded.contains("IM"), "unreserved bytes must not be escaped");
    }

    // --- bencode ----------------------------------------------------------

    #[test]
    fn parses_a_bare_integer() {
        assert_eq!(parse_bencode(b"i42e").unwrap(), Bencode::Int(42));
    }

    #[test]
    fn parses_a_negative_integer() {
        assert_eq!(parse_bencode(b"i-7e").unwrap(), Bencode::Int(-7));
    }

    #[test]
    fn parses_a_byte_string() {
        assert_eq!(
            parse_bencode(b"4:abcd").unwrap(),
            Bencode::Bytes(b"abcd".to_vec())
        );
    }

    #[test]
    fn parses_an_empty_byte_string() {
        assert_eq!(parse_bencode(b"0:").unwrap(), Bencode::Bytes(Vec::new()));
    }

    #[test]
    fn parses_a_list() {
        assert_eq!(
            parse_bencode(b"li1ei2ee").unwrap(),
            Bencode::List(vec![Bencode::Int(1), Bencode::Int(2)])
        );
    }

    #[test]
    fn parses_an_empty_list() {
        assert_eq!(parse_bencode(b"le").unwrap(), Bencode::List(Vec::new()));
    }

    #[test]
    fn parses_a_dictionary() {
        let parsed = parse_bencode(b"d1:ai1ee").unwrap();
        assert_eq!(parsed.get(b"a").and_then(Bencode::as_int), Some(1));
    }

    #[test]
    fn parses_a_nested_dictionary() {
        let parsed = parse_bencode(b"d1:ad1:bi2eee").unwrap();
        assert_eq!(
            parsed.get(b"a").and_then(|inner| inner.get(b"b")).and_then(Bencode::as_int),
            Some(2)
        );
    }

    #[test]
    fn a_missing_key_is_none() {
        let parsed = parse_bencode(b"d1:ai1ee").unwrap();
        assert!(parsed.get(b"missing").is_none());
    }

    #[test]
    fn rejects_trailing_data() {
        assert_eq!(
            parse_bencode(b"i1ei2e"),
            Err(BencodeError::TrailingData(3))
        );
    }

    #[test]
    fn rejects_a_truncated_string() {
        assert_eq!(parse_bencode(b"4:ab"), Err(BencodeError::UnexpectedEnd));
    }

    #[test]
    fn rejects_an_unterminated_integer() {
        assert_eq!(parse_bencode(b"i42"), Err(BencodeError::InvalidInteger));
    }

    #[test]
    fn rejects_an_integer_with_no_digits() {
        assert_eq!(parse_bencode(b"ie"), Err(BencodeError::InvalidInteger));
    }

    #[test]
    fn rejects_a_length_with_a_leading_zero() {
        assert_eq!(parse_bencode(b"04:abcd"), Err(BencodeError::InvalidLength));
    }

    #[test]
    fn rejects_a_length_without_a_colon() {
        assert_eq!(parse_bencode(b"4abcd"), Err(BencodeError::InvalidLength));
    }

    #[test]
    fn rejects_an_unterminated_list() {
        assert_eq!(parse_bencode(b"li1e"), Err(BencodeError::UnexpectedEnd));
    }

    #[test]
    fn rejects_an_unterminated_dictionary() {
        assert_eq!(parse_bencode(b"d1:ai1e"), Err(BencodeError::UnexpectedEnd));
    }

    #[test]
    fn rejects_a_dictionary_key_that_is_not_a_string() {
        assert_eq!(parse_bencode(b"di1ei2ee"), Err(BencodeError::UnexpectedByte(b'i')));
    }

    #[test]
    fn rejects_an_empty_buffer() {
        assert_eq!(parse_bencode(b""), Err(BencodeError::UnexpectedEnd));
    }

    #[test]
    fn rejects_nesting_past_the_depth_limit() {
        // Balanced input two levels past the limit: every list is closed, so
        // `DepthExceeded` can only come from the guard and not from the
        // parser running out of input.
        let depth = MAX_DEPTH + 2;
        let mut input = vec![b'l'; depth];
        input.extend(std::iter::repeat(b'e').take(depth));

        assert_eq!(parse_bencode(&input), Err(BencodeError::DepthExceeded));
    }

    #[test]
    fn accepts_nesting_at_the_depth_limit() {
        // Balanced at exactly MAX_DEPTH, which is the deepest input the guard
        // must still accept. Off-by-one errors here would silently reject
        // legitimate responses, so the boundary is asserted from both sides.
        let mut input = vec![b'l'; MAX_DEPTH];
        input.extend(std::iter::repeat(b'e').take(MAX_DEPTH));

        // `expect` rather than `assert!(is_ok())` so a regression prints the
        // error that was actually returned instead of just "assertion failed".
        let parsed = parse_bencode(&input).expect("nesting at the limit should parse");
        assert!(matches!(parsed, Bencode::List(_)));
    }

    // --- scrape_stats_from_bencode ---------------------------------------

    /// Build a canonical scrape response for one torrent.
    fn scrape_response(seeders: i64, leechers: i64, completed: i64) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"d5:filesd20:");
        out.extend_from_slice(&hash_bytes());
        out.extend_from_slice(b"d8:completei");
        out.extend_from_slice(seeders.to_string().as_bytes());
        out.extend_from_slice(b"e10:downloadedi");
        out.extend_from_slice(completed.to_string().as_bytes());
        out.extend_from_slice(b"e10:incompletei");
        out.extend_from_slice(leechers.to_string().as_bytes());
        // Close the incomplete int, then the stats, files and root dicts.
        out.extend_from_slice(b"eeee");
        out
    }

    #[test]
    fn reads_stats_from_a_canonical_response() {
        let stats = scrape_stats_from_bencode(&scrape_response(12, 3, 90), &hash_bytes())
            .expect("should decode");

        assert_eq!(
            stats,
            ScrapeStats {
                seeders: 12,
                leechers: 3,
                completed: 90,
            }
        );
    }

    #[test]
    fn reads_stats_from_a_flat_response() {
        // Some trackers skip the `files` wrapper for a single-hash scrape.
        let mut response = Vec::new();
        response.extend_from_slice(b"d20:");
        response.extend_from_slice(&hash_bytes());
        response.extend_from_slice(b"d8:completei5e10:downloadedi1e10:incompletei2eee");

        let stats = scrape_stats_from_bencode(&response, &hash_bytes()).expect("should decode");
        assert_eq!(stats.seeders, 5);
        assert_eq!(stats.leechers, 2);
    }

    #[test]
    fn a_failure_reason_is_an_error() {
        let response = b"d14:failure reason12:unregisterede";

        assert_eq!(
            scrape_stats_from_bencode(response, &hash_bytes()),
            Err(ScrapeError::TrackerFailure("unregistered".into()))
        );
    }

    #[test]
    fn a_torrent_the_tracker_does_not_know_is_an_error_not_zero_seeders() {
        // Reporting `ScrapeStats::default()` here would look like a healthy
        // response saying "no peers", which would wrongly mark a release dead.
        let mut response = Vec::new();
        response.extend_from_slice(b"d5:filesd20:");
        response.extend_from_slice(&[0x11; 20]);
        response.extend_from_slice(b"d8:completei5eeee");

        assert_eq!(
            scrape_stats_from_bencode(&response, &hash_bytes()),
            Err(ScrapeError::UnknownTorrent)
        );
    }

    #[test]
    fn missing_count_fields_read_as_zero() {
        let mut response = Vec::new();
        response.extend_from_slice(b"d5:filesd20:");
        response.extend_from_slice(&hash_bytes());
        response.extend_from_slice(b"d8:completei7eeee");

        let stats = scrape_stats_from_bencode(&response, &hash_bytes()).expect("should decode");
        assert_eq!(stats.seeders, 7);
        assert_eq!(stats.leechers, 0);
        assert_eq!(stats.completed, 0);
    }

    #[test]
    fn a_negative_count_reads_as_zero() {
        let stats = scrape_stats_from_bencode(&scrape_response(-1, 0, 0), &hash_bytes())
            .expect("should decode");
        assert_eq!(stats.seeders, 0);
    }

    #[test]
    fn an_undecodable_response_is_a_decode_error() {
        assert!(matches!(
            scrape_stats_from_bencode(b"not bencode", &hash_bytes()),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn scrape_error_is_a_std_error() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<ScrapeError>();
    }
}