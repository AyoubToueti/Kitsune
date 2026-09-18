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

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::torrent::magnet::percent_decode;
use crate::types::Release;

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
fn parse_value(input: &[u8], cursor: &mut usize, depth: usize) -> Result<Bencode, BencodeError> {
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

    let digits =
        std::str::from_utf8(&input[start..end]).map_err(|_| BencodeError::InvalidLength)?;
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

    let text = std::str::from_utf8(&input[start..end]).map_err(|_| BencodeError::InvalidInteger)?;
    let value = text
        .parse::<i64>()
        .map_err(|_| BencodeError::InvalidInteger)?;

    *cursor = end + 1;
    Ok(Bencode::Int(value))
}

/// Parse `l<value>...e`.
fn parse_list(input: &[u8], cursor: &mut usize, depth: usize) -> Result<Bencode, BencodeError> {
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
fn parse_dict(input: &[u8], cursor: &mut usize, depth: usize) -> Result<Bencode, BencodeError> {
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

    let stats = files.get(info_hash).ok_or(ScrapeError::UnknownTorrent)?;

    Ok(ScrapeStats {
        seeders: count(stats.get(b"complete")),
        leechers: count(stats.get(b"incomplete")),
        completed: count(stats.get(b"downloaded")),
    })
}

/// How long one tracker gets before it is considered unreachable.
///
/// Three seconds is a compromise: a tracker that is up answers in tens of
/// milliseconds, so this only ever expires on a genuinely dead or filtered
/// host, and a release rarely lists more than a handful of trackers.
pub const TRACKER_TIMEOUT: Duration = Duration::from_secs(3);

/// The `scrape` URL corresponding to an `announce` URL (BEP 0010).
///
/// Trackers are announced to at `.../announce` and scraped at `.../scrape`,
/// so the final path segment is what changes. A URL that does not end in
/// `announce` cannot be rewritten mechanically and is reported as `None`
/// rather than guessed at, since a wrong URL is only a wasted request.
pub fn scrape_url_from_announce(announce_url: &str) -> Option<String> {
    let trimmed = announce_url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return None;
    }

    match trimmed.rsplit_once("announce") {
        // Only the final segment is rewritten: a tracker whose host happens
        // to contain "announce" must not have the host mangled.
        //
        // An empty, path or query suffix is all legitimate. A passkey is
        // usually carried as a query on the announce URL, so rejecting a
        // `?` suffix would silently disable scraping on private trackers.
        Some((prefix, suffix))
            if suffix.is_empty() || suffix.starts_with('/') || suffix.starts_with('?') =>
        {
            Some(format!("{prefix}scrape{suffix}"))
        }
        _ => None,
    }
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

/// Ask one HTTP tracker for a torrent's stats (BEP 0010).
///
/// The info hash is sent as percent-encoded raw bytes, not hex: the two look
/// similar but are different requests, and a tracker will not recognise the
/// hex form.
pub async fn scrape_http(
    client: &reqwest::Client,
    announce_url: &str,
    info_hash: &[u8; 20],
    timeout: Duration,
) -> Result<ScrapeStats, ScrapeError> {
    let base = scrape_url_from_announce(announce_url).ok_or_else(|| {
        ScrapeError::Decode(format!("'{announce_url}' is not an http announce URL"))
    })?;

    let separator = if base.contains('?') { '&' } else { '?' };
    let url = format!(
        "{base}{separator}info_hash={}",
        percent_encode_bytes(info_hash)
    );

    let response = tokio::time::timeout(timeout, client.get(&url).send())
        .await
        .map_err(|_| ScrapeError::Transport(format!("{announce_url} timed out")))?
        .map_err(|e| ScrapeError::Transport(e.to_string()))?;

    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|e| ScrapeError::Transport(e.to_string()))?;

    if !status.is_success() {
        // Report the scrape URL, not the announce URL: they differ, and a
        // message naming the wrong one sends a reader to the wrong request.
        return Err(ScrapeError::Transport(format!(
            "{url} answered HTTP {status}"
        )));
    }

    scrape_stats_from_bencode(&body, info_hash)
}

/// The UDP tracker protocol's magic connection constant (BEP 0015).
///
/// Every connect request must begin with it; a tracker that does not see it
/// ignores the packet. It is a fixed value from the specification, not a
/// per-request secret.
const UDP_PROTOCOL_ID: u64 = 0x0000_0417_2710_1980;

/// UDP action codes: 0 connects, 2 scrapes.
const UDP_ACTION_CONNECT: u32 = 0;
const UDP_ACTION_SCRAPE: u32 = 2;

/// Build a UDP connect request.
///
/// Pure and separated from the socket so the byte layout -- the part that is
/// easy to get wrong and impossible to eyeball -- is asserted in a test rather
/// than inferred from a working connection.
pub fn build_udp_connect_request(transaction_id: u32) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&UDP_PROTOCOL_ID.to_be_bytes());
    out[8..12].copy_from_slice(&UDP_ACTION_CONNECT.to_be_bytes());
    out[12..16].copy_from_slice(&transaction_id.to_be_bytes());
    out
}

/// Read the connection id out of a connect response.
pub fn parse_udp_connect_response(
    response: &[u8],
    transaction_id: u32,
) -> Result<u64, ScrapeError> {
    if response.len() < 16 {
        return Err(ScrapeError::Decode(format!(
            "connect response was {} bytes, expected 16",
            response.len()
        )));
    }

    let action = u32::from_be_bytes(response[0..4].try_into().expect("4 bytes"));
    if action != UDP_ACTION_CONNECT {
        return Err(ScrapeError::Decode(format!(
            "expected a connect action, got {action}"
        )));
    }

    // The transaction id is echoed back specifically so a stale reply to an
    // earlier request cannot be mistaken for this one's answer.
    let echoed = u32::from_be_bytes(response[4..8].try_into().expect("4 bytes"));
    if echoed != transaction_id {
        return Err(ScrapeError::Decode(format!(
            "connect response carried transaction {echoed}, expected {transaction_id}"
        )));
    }

    Ok(u64::from_be_bytes(
        response[8..16].try_into().expect("8 bytes"),
    ))
}

/// Build a UDP scrape request for one torrent.
pub fn build_udp_scrape_request(
    connection_id: u64,
    transaction_id: u32,
    info_hash: &[u8; 20],
) -> [u8; 36] {
    let mut out = [0u8; 36];
    out[..8].copy_from_slice(&connection_id.to_be_bytes());
    out[8..12].copy_from_slice(&UDP_ACTION_SCRAPE.to_be_bytes());
    out[12..16].copy_from_slice(&transaction_id.to_be_bytes());
    out[16..36].copy_from_slice(info_hash);
    out
}

/// Read the stats for one torrent out of a UDP scrape response.
pub fn parse_udp_scrape_response(
    response: &[u8],
    transaction_id: u32,
) -> Result<ScrapeStats, ScrapeError> {
    // 8 bytes of header plus three 4-byte counts for a single torrent.
    if response.len() < 20 {
        return Err(ScrapeError::Decode(format!(
            "scrape response was {} bytes, expected at least 20",
            response.len()
        )));
    }

    let action = u32::from_be_bytes(response[0..4].try_into().expect("4 bytes"));
    if action != UDP_ACTION_SCRAPE {
        return Err(ScrapeError::Decode(format!(
            "expected a scrape action, got {action}"
        )));
    }

    let echoed = u32::from_be_bytes(response[4..8].try_into().expect("4 bytes"));
    if echoed != transaction_id {
        return Err(ScrapeError::Decode(format!(
            "scrape response carried transaction {echoed}, expected {transaction_id}"
        )));
    }

    Ok(ScrapeStats {
        seeders: u32::from_be_bytes(response[8..12].try_into().expect("4 bytes")),
        completed: u32::from_be_bytes(response[12..16].try_into().expect("4 bytes")),
        leechers: u32::from_be_bytes(response[16..20].try_into().expect("4 bytes")),
    })
}

/// Split a `udp://host:port` tracker URL into its host and port.
///
/// Returns the parts rather than a `SocketAddr` because a tracker is named by
/// host, not by IP. Resolving that host is I/O, so it happens in the caller;
/// keeping this pure is what makes it testable without a network.
pub fn udp_authority(tracker_url: &str) -> Result<(String, u16), ScrapeError> {
    let rest = tracker_url
        .strip_prefix("udp://")
        .ok_or_else(|| ScrapeError::Decode(format!("'{tracker_url}' is not a udp tracker")))?;

    // A tracker URL may carry a path (`udp://host:80/announce`); only the
    // authority is needed to dial it.
    let authority = rest.split('/').next().unwrap_or(rest);

    // `rsplit_once` so an IPv6 literal's own colons are not mistaken for the
    // port separator.
    let (host, port) = authority
        .rsplit_once(':')
        .ok_or_else(|| ScrapeError::Decode(format!("'{authority}' has no port")))?;

    let port = port
        .parse::<u16>()
        .map_err(|e| ScrapeError::Decode(format!("'{port}' is not a port: {e}")))?;

    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return Err(ScrapeError::Decode(format!("'{authority}' has no host")));
    }

    Ok((host.to_string(), port))
}

/// A transaction id for one UDP request pair.
///
/// Only needs to be unlikely to collide with a concurrent request, and the
/// sockets here are per-scrape, so a counter is enough. It is not a security
/// value.
fn next_transaction_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Ask one UDP tracker for a torrent's stats (BEP 0015).
///
/// Two round trips: a connect handshake that yields a short-lived connection
/// id, then the scrape itself. UDP is preferred over HTTP because it avoids a
/// TCP handshake and a full HTTP exchange, which matters when several
/// trackers are being asked at once.
pub async fn scrape_udp(
    tracker_url: &str,
    info_hash: &[u8; 20],
    timeout: Duration,
) -> Result<ScrapeStats, ScrapeError> {
    let (host, port) = udp_authority(tracker_url)?;

    // Bind on the wildcard address with an OS-chosen port: the tracker
    // replies to the address the request came from, so any ephemeral port
    // works.
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| ScrapeError::Transport(format!("could not open a UDP socket: {e}")))?;

    // Connecting by host lets the resolver pick an address, and remembers it
    // so a reply is matched to this peer rather than any sender.
    socket
        .connect((host.as_str(), port))
        .await
        .map_err(|e| ScrapeError::Transport(format!("could not reach {host}:{port}: {e}")))?;

    let connect_id = next_transaction_id();
    let connect_request = build_udp_connect_request(connect_id);

    // One buffer is reused for both replies; a tracker response is far
    // smaller than this, and `recv` reports the actual length.
    let mut buffer = [0u8; 2048];

    tokio::time::timeout(timeout, socket.send(&connect_request))
        .await
        .map_err(|_| ScrapeError::Transport(format!("{tracker_url} timed out on connect")))?
        .map_err(|e| ScrapeError::Transport(e.to_string()))?;

    let read = tokio::time::timeout(timeout, socket.recv(&mut buffer))
        .await
        .map_err(|_| ScrapeError::Transport(format!("{tracker_url} timed out on connect")))?
        .map_err(|e| ScrapeError::Transport(e.to_string()))?;

    let connection_id = parse_udp_connect_response(&buffer[..read], connect_id)?;

    let scrape_id = next_transaction_id();
    let scrape_request = build_udp_scrape_request(connection_id, scrape_id, info_hash);

    tokio::time::timeout(timeout, socket.send(&scrape_request))
        .await
        .map_err(|_| ScrapeError::Transport(format!("{tracker_url} timed out on scrape")))?
        .map_err(|e| ScrapeError::Transport(e.to_string()))?;

    let read = tokio::time::timeout(timeout, socket.recv(&mut buffer))
        .await
        .map_err(|_| ScrapeError::Transport(format!("{tracker_url} timed out on scrape")))?
        .map_err(|e| ScrapeError::Transport(e.to_string()))?;

    parse_udp_scrape_response(&buffer[..read], scrape_id)
}

/// Ask whichever protocol a tracker URL names for a torrent's stats.
///
/// A `udp://` tracker is scraped over UDP and everything else over HTTP, so a
/// caller can hand over whatever the magnet listed without inspecting the
/// scheme itself.
pub async fn scrape_tracker(
    client: &reqwest::Client,
    tracker_url: &str,
    info_hash: &[u8; 20],
    timeout: Duration,
) -> Result<ScrapeStats, ScrapeError> {
    let trimmed = tracker_url.trim();

    if trimmed.starts_with("udp://") {
        return scrape_udp(trimmed, info_hash, timeout).await;
    }

    scrape_http(client, trimmed, info_hash, timeout).await
}

/// How long a torrent gets to publish its metadata.
///
/// Longer than a tracker scrape because it is a different kind of wait: the
/// metadata has to come from a peer over the BitTorrent protocol, not from a
/// tracker that answers in one packet. Thirty seconds is long enough for a
/// slow swarm to hand over the info dictionary and short enough that a dead
/// magnet does not stall the list for minutes.
pub const METADATA_TIMEOUT: Duration = Duration::from_secs(30);

/// What a metadata probe learned about a torrent.
///
/// `resolved` is the headline: a magnet whose metadata a peer actually served
/// is a torrent that exists, which is a much stronger statement than a
/// seeder count on a web page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataProbe {
    /// Whether the metadata arrived before the timeout.
    pub resolved: bool,
    /// Files in the torrent, once known.
    pub file_count: Option<u32>,
    /// Total size in bytes, once known.
    pub total_bytes: Option<u64>,
    /// How long the probe took, from adding the magnet to the verdict.
    pub duration_ms: u64,
}

/// How often the probe looks for metadata that has arrived.
///
/// librqbit hands metadata over asynchronously and has no single "metadata
/// ready" signal to await that this probe can use, so it polls. A tenth of a
/// second is far below the resolution of the timeout, so the poll adds no
/// meaningful latency to the verdict.
const METADATA_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Read a torrent's file count and total size from resolved metadata.
fn metadata_summary(handle: &librqbit::ManagedTorrent) -> Option<(u32, u64)> {
    let mut summary = None;

    // `with_metadata` only invokes the closure once the info dictionary is
    // present, so a successful call *is* the "metadata resolved" answer.
    let _ = handle.with_metadata(|meta| {
        let count = meta.file_infos.len() as u32;
        let total = meta.file_infos.iter().map(|file| file.len).sum();
        summary = Some((count, total));
    });

    summary
}

/// Ask a torrent's swarm for its metadata, without downloading any of it.
///
/// This is the strongest liveness signal available: a tracker can report
/// seeders that have since gone away, but a peer that serves the info
/// dictionary is demonstrably present and connected.
///
/// The session is passed in rather than created here so the caller can probe
/// many releases over one session. Starting a session binds sockets and joins
/// the DHT, which is far too heavy to do per release.
///
/// It is an `Arc<Session>` rather than a `&Session` because that is what
/// librqbit's `add_torrent` requires: the engine keeps the torrent alive
/// behind the same allocation.
pub async fn probe_metadata(
    session: &std::sync::Arc<librqbit::Session>,
    magnet_uri: &str,
    timeout: Duration,
) -> MetadataProbe {
    let started = std::time::Instant::now();

    // The timeout wraps the *whole* operation, not just the polling loop.
    // `add_torrent` blocks until metadata arrives when the magnet has no peer
    // willing to hand it over, so bounding only the loop would let a dead
    // magnet hang the probe forever. An ignored live test caught exactly that.
    let resolved = tokio::time::timeout(timeout, resolve_metadata(session, magnet_uri))
        .await
        .unwrap_or(None);

    MetadataProbe {
        resolved: resolved.is_some(),
        file_count: resolved.map(|(count, _)| count),
        total_bytes: resolved.map(|(_, bytes)| bytes),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// Add a magnet and wait for its metadata, or give up with `None`.
///
/// Split from [`probe_metadata`] so the caller can bound the whole thing with
/// one timeout. A `None` is "no metadata", which the caller turns into a
/// verdict rather than an error.
///
/// Cancelling this future mid-add can leave the torrent registered in the
/// session, which is harmless here: the session is dropped when the probe
/// batch finishes, and no pieces were ever requested.
async fn resolve_metadata(
    session: &std::sync::Arc<librqbit::Session>,
    magnet_uri: &str,
) -> Option<(u32, u64)> {
    let handle = session
        .add_torrent(
            librqbit::AddTorrent::from_url(magnet_uri),
            Some(librqbit::AddTorrentOptions {
                overwrite: true,
                // Metadata only: select no files, so nothing is ever fetched.
                only_files: Some(Vec::new()),
                list_only: false,
                ..Default::default()
            }),
        )
        .await
        .ok()?
        .into_handle()?;

    loop {
        if let Some(summary) = metadata_summary(&handle) {
            return Some(summary);
        }
        tokio::time::sleep(METADATA_POLL_INTERVAL).await;
    }
}

/// A probe that ran out of time, stamped with how long it waited.
fn unresolved(started: &std::time::Instant) -> MetadataProbe {
    MetadataProbe {
        resolved: false,
        file_count: None,
        total_bytes: None,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// How many tracker scrapes may be in flight at once.
///
/// A popular episode returns twenty releases, each naming four or five
/// trackers, and the same handful of tracker hosts appears across all of them.
/// Probing everything at once would open a hundred sockets against a few
/// hosts, which is both wasteful and rude. Metadata probes are deliberately
/// *not* capped: they are bounded by the swarm answering, not by our socket
/// use, and each one is a single session-level operation.
pub const MAX_CONCURRENT_TRACKER_SCRAPES: usize = 5;

/// What one tracker said about a release, with the bookkeeping the UI needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackerScrape {
    pub seeders: u32,
    pub leechers: u32,
    pub completed: u32,
    /// The tracker that answered. A count is only as good as its source, and
    /// the UI shows which one it came from.
    pub tracker_url: String,
    pub duration_ms: u64,
}

/// The outcome of probing one release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    /// The release this describes, by info hash. Empty when the release
    /// carried none, in which case there is nothing to probe.
    pub info_hash: String,
    /// The best tracker answer, or `None` when no tracker replied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrape: Option<TrackerScrape>,
    /// What the metadata probe learned. Always present when the probe ran;
    /// `None` only if it was skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MetadataProbe>,
    /// Wall-clock time for the whole probe, both halves together.
    pub total_duration_ms: u64,
}

/// Ask a release's trackers for live stats, and its swarm for metadata.
///
/// The two halves run concurrently and neither can fail the other: a release
/// with no reachable tracker can still prove itself by serving metadata, and a
/// release whose swarm is asleep can still have an accurate seeder count. The
/// result reports whichever halves answered.
pub async fn probe_release(
    session: &Arc<librqbit::Session>,
    client: &reqwest::Client,
    release: &Release,
    semaphore: &Arc<Semaphore>,
) -> ProbeResult {
    let started = std::time::Instant::now();

    // Prefer the release's own hash, but fall back to parsing the magnet so a
    // release built by a provider that left the field empty still probes.
    let info_hash = release
        .info_hash
        .clone()
        .or_else(|| crate::torrent::magnet::parse_info_hash(&release.magnet_uri).ok())
        .unwrap_or_default();

    let scrape = scrape_release(client, release, &info_hash, semaphore);
    let metadata = probe_metadata(session, &release.magnet_uri, METADATA_TIMEOUT);

    let (scrape, metadata) = tokio::join!(scrape, metadata);

    ProbeResult {
        info_hash,
        scrape,
        metadata: Some(metadata),
        total_duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// Try every tracker a release lists until one answers.
///
/// Trackers are tried in order rather than raced: the first one that answers
/// is enough, and racing them would triple the request count for no better
/// answer. A permit is held across the whole release, so the concurrency cap
/// counts releases being scraped, not individual tracker requests.
async fn scrape_release(
    client: &reqwest::Client,
    release: &Release,
    info_hash: &str,
    semaphore: &Arc<Semaphore>,
) -> Option<TrackerScrape> {
    let hash = decode_info_hash(info_hash).ok()?;
    let trackers = extract_trackers(&release.magnet_uri);
    if trackers.is_empty() {
        return None;
    }

    // `.ok()?` rather than unwrap: a closed semaphore means the probe batch is
    // shutting down, and giving up quietly is the right response.
    let _permit = semaphore.acquire().await.ok()?;

    for tracker in &trackers {
        let started = std::time::Instant::now();
        if let Ok(stats) = scrape_tracker(client, tracker, &hash, TRACKER_TIMEOUT).await {
            return Some(TrackerScrape {
                seeders: stats.seeders,
                leechers: stats.leechers,
                completed: stats.completed,
                tracker_url: tracker.clone(),
                duration_ms: started.elapsed().as_millis() as u64,
            });
        }
    }

    None
}

/// Probe every release concurrently, returning `(index, result)` pairs.
///
/// The index is carried through rather than relying on completion order, so
/// the caller can match each result back to the release it describes even
/// though a fast probe for release 7 lands before a slow one for release 1.
/// Results come back sorted by index for convenience.
pub async fn probe_releases(
    session: &Arc<librqbit::Session>,
    client: reqwest::Client,
    releases: &[Release],
) -> Vec<(usize, ProbeResult)> {
    probe_releases_reporting(session, client, releases, |_, _| {}).await
}

/// Probe every release concurrently, reporting each result as it lands.
///
/// The callback runs in completion order, before the caller's list is
/// assembled. That is what lets the UI re-rank progressively: a slow probe for
/// release 1 must not hold back the verdict for release 7. The returned list is
/// still sorted by index, so a caller that ignores the callback gets the same
/// answer as [`probe_releases`].
///
/// The callback takes a reference because it fires for every result and the
/// results are also returned; handing out a clone each time would copy every
/// probe's data for no reason.
pub async fn probe_releases_reporting(
    session: &Arc<librqbit::Session>,
    client: reqwest::Client,
    releases: &[Release],
    mut on_result: impl FnMut(usize, &ProbeResult),
) -> Vec<(usize, ProbeResult)> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TRACKER_SCRAPES));
    let mut set = JoinSet::new();

    for (index, release) in releases.iter().enumerate() {
        // Cloned per task: each spawned future must own what it touches, and
        // the session is shared behind the same `Arc` rather than restarted.
        let session = Arc::clone(session);
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let release = release.clone();

        set.spawn(async move {
            let result = probe_release(&session, &client, &release, &semaphore).await;
            (index, result)
        });
    }

    let mut results = Vec::with_capacity(releases.len());
    while let Some(joined) = set.join_next().await {
        // A panicking task loses only its own result; the rest still arrive.
        if let Ok((index, result)) = joined {
            on_result(index, &result);
            results.push((index, result));
        }
    }

    results.sort_by_key(|(index, _)| *index);
    results
}

/// Run a probe batch on a session of its own, then shut it down.
///
/// A dedicated session is not a nicety: `add_torrent` returns
/// `AlreadyManaged` for an info hash it already holds, before applying the
/// new call's options. If the probe registered a release with an empty file
/// selection on the *player's* session, a later playback request for the same
/// torrent would keep that empty selection and download nothing.
/// `tests/probe_session_conflict.rs` pins that behaviour down.
///
/// The session is per batch rather than long-lived so probed torrents cannot
/// accumulate: it is dropped when the batch ends, which releases every
/// registration with it.
///
/// A session that will not start yields no results rather than an error. The
/// probe is an enhancement to the ranking, so failing to run it should leave
/// the static order in place, not turn a working search into a failure.
pub async fn probe_releases_on_own_session(
    download_dir: std::path::PathBuf,
    client: reqwest::Client,
    releases: &[Release],
    on_result: impl FnMut(usize, &ProbeResult),
) -> Vec<(usize, ProbeResult)> {
    let Ok(session) = librqbit::Session::new(download_dir).await else {
        return Vec::new();
    };

    let results = probe_releases_reporting(&session, client, releases, on_result).await;
    session.stop().await;
    results
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
        assert_eq!(percent_encode_bytes(&[0x00, 0xAB, 0xFF]), "%00%AB%FF");
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
        assert!(
            encoded.contains("IM"),
            "unreserved bytes must not be escaped"
        );
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
            parsed
                .get(b"a")
                .and_then(|inner| inner.get(b"b"))
                .and_then(Bencode::as_int),
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
        assert_eq!(parse_bencode(b"i1ei2e"), Err(BencodeError::TrailingData(3)));
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
        assert_eq!(
            parse_bencode(b"di1ei2ee"),
            Err(BencodeError::UnexpectedByte(b'i'))
        );
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

    // --- scrape URL rewriting ---------------------------------------------

    #[test]
    fn rewrites_an_announce_url_to_scrape() {
        assert_eq!(
            scrape_url_from_announce("http://tracker.test/announce").as_deref(),
            Some("http://tracker.test/scrape")
        );
    }

    #[test]
    fn rewrites_announce_in_a_subdirectory() {
        assert_eq!(
            scrape_url_from_announce("http://tracker.test/tracker/announce").as_deref(),
            Some("http://tracker.test/tracker/scrape")
        );
    }

    #[test]
    fn keeps_a_passkey_query_after_announce() {
        // Private trackers carry a passkey as a query on the announce URL.
        // Dropping it would make every scrape 404, so the query must survive.
        assert_eq!(
            scrape_url_from_announce("https://tracker.test/a/announce?passkey=abc").as_deref(),
            Some("https://tracker.test/a/scrape?passkey=abc")
        );
    }

    #[test]
    fn does_not_rewrite_a_url_without_announce() {
        assert_eq!(scrape_url_from_announce("http://tracker.test/scrape"), None);
    }

    #[test]
    fn does_not_rewrite_a_udp_url() {
        assert_eq!(
            scrape_url_from_announce("udp://tracker.test:80/announce"),
            None
        );
    }

    // --- HTTP scrape ------------------------------------------------------

    #[tokio::test]
    async fn http_scrape_reads_stats_and_sends_the_hash_percent_encoded() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let info_hash = hash_bytes();

        Mock::given(method("GET"))
            .and(path("/scrape"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(scrape_response(4, 1, 20)))
            .mount(&server)
            .await;

        let stats = scrape_http(
            &reqwest::Client::new(),
            &format!("{}/announce", server.uri()),
            &info_hash,
            TRACKER_TIMEOUT,
        )
        .await
        .expect("should scrape");

        assert_eq!(stats.seeders, 4);
        assert_eq!(stats.leechers, 1);

        // Assert on the request that actually arrived. A `query_param` matcher
        // cannot express this: the value is raw bytes, so its escaped form is
        // not valid UTF-8 and the matcher would compare a lossy decode.
        let requests = server
            .received_requests()
            .await
            .expect("wiremock records requests");
        let url = requests[0].url.as_str();

        assert!(
            url.contains(&format!("info_hash={}", percent_encode_bytes(&info_hash))),
            "info hash must be percent-encoded, got {url}"
        );
        assert!(
            !url.contains(&format!("info_hash={HASH}")),
            "the hex form must not be sent; it is a different request"
        );
    }

    #[tokio::test]
    async fn http_scrape_reports_a_non_success_status() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let result = scrape_http(
            &reqwest::Client::new(),
            &format!("{}/announce", server.uri()),
            &hash_bytes(),
            TRACKER_TIMEOUT,
        )
        .await;

        assert!(matches!(result, Err(ScrapeError::Transport(_))));
    }

    #[tokio::test]
    async fn http_scrape_rejects_a_non_http_url() {
        let result = scrape_http(
            &reqwest::Client::new(),
            "udp://tracker.test:80/announce",
            &hash_bytes(),
            TRACKER_TIMEOUT,
        )
        .await;

        assert!(matches!(result, Err(ScrapeError::Decode(_))));
    }

    #[tokio::test]
    async fn http_scrape_times_out_on_a_silent_tracker() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            // Longer than the timeout the client is given below.
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(30)))
            .mount(&server)
            .await;

        let started = std::time::Instant::now();
        let result = scrape_http(
            &reqwest::Client::new(),
            &format!("{}/announce", server.uri()),
            &hash_bytes(),
            Duration::from_millis(200),
        )
        .await;

        assert!(matches!(result, Err(ScrapeError::Transport(_))));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the client timeout should have fired, not the mock delay"
        );
    }

    // --- UDP byte layouts -------------------------------------------------

    #[test]
    fn udp_connect_request_matches_bep_0015() {
        let request = build_udp_connect_request(0x1234_5678);

        assert_eq!(&request[..8], &UDP_PROTOCOL_ID.to_be_bytes());
        assert_eq!(&request[8..12], &0u32.to_be_bytes(), "action is connect");
        assert_eq!(&request[12..16], &0x1234_5678u32.to_be_bytes());
    }

    #[test]
    fn udp_connect_response_yields_the_connection_id() {
        let mut response = Vec::new();
        response.extend_from_slice(&0u32.to_be_bytes());
        response.extend_from_slice(&7u32.to_be_bytes());
        response.extend_from_slice(&0xDEAD_BEEF_CAFE_F00Du64.to_be_bytes());

        assert_eq!(
            parse_udp_connect_response(&response, 7).unwrap(),
            0xDEAD_BEEF_CAFE_F00D
        );
    }

    #[test]
    fn udp_connect_response_rejects_a_stale_transaction() {
        let mut response = Vec::new();
        response.extend_from_slice(&0u32.to_be_bytes());
        response.extend_from_slice(&99u32.to_be_bytes());
        response.extend_from_slice(&0u64.to_be_bytes());

        assert!(matches!(
            parse_udp_connect_response(&response, 7),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn udp_connect_response_rejects_a_short_buffer() {
        assert!(matches!(
            parse_udp_connect_response(&[0u8; 8], 7),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn udp_connect_response_rejects_the_wrong_action() {
        let mut response = Vec::new();
        response.extend_from_slice(&2u32.to_be_bytes());
        response.extend_from_slice(&7u32.to_be_bytes());
        response.extend_from_slice(&0u64.to_be_bytes());

        assert!(matches!(
            parse_udp_connect_response(&response, 7),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn udp_scrape_request_matches_bep_0015() {
        let info_hash = hash_bytes();
        let request = build_udp_scrape_request(0x1122_3344_5566_7788, 42, &info_hash);

        assert_eq!(&request[..8], &0x1122_3344_5566_7788u64.to_be_bytes());
        assert_eq!(&request[8..12], &2u32.to_be_bytes(), "action is scrape");
        assert_eq!(&request[12..16], &42u32.to_be_bytes());
        assert_eq!(&request[16..36], &info_hash[..]);
    }

    #[test]
    fn udp_scrape_response_maps_complete_to_seeders() {
        // The tracker protocol's field order is complete, downloaded,
        // incomplete -- not the order they are usually spoken about.
        let mut response = Vec::new();
        response.extend_from_slice(&2u32.to_be_bytes());
        response.extend_from_slice(&42u32.to_be_bytes());
        response.extend_from_slice(&11u32.to_be_bytes());
        response.extend_from_slice(&22u32.to_be_bytes());
        response.extend_from_slice(&33u32.to_be_bytes());

        let stats = parse_udp_scrape_response(&response, 42).unwrap();
        assert_eq!(
            stats,
            ScrapeStats {
                seeders: 11,
                completed: 22,
                leechers: 33,
            }
        );
    }

    #[test]
    fn udp_scrape_response_rejects_a_short_buffer() {
        assert!(matches!(
            parse_udp_scrape_response(&[0u8; 12], 42),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn udp_scrape_response_rejects_a_stale_transaction() {
        let mut response = Vec::new();
        response.extend_from_slice(&2u32.to_be_bytes());
        response.extend_from_slice(&1u32.to_be_bytes());
        response.extend_from_slice(&[0u8; 12]);

        assert!(matches!(
            parse_udp_scrape_response(&response, 42),
            Err(ScrapeError::Decode(_))
        ));
    }

    // --- UDP address parsing ---------------------------------------------

    #[test]
    fn parses_a_udp_tracker_host_and_port() {
        assert_eq!(
            udp_authority("udp://tracker.test:1337/announce").unwrap(),
            ("tracker.test".to_string(), 1337)
        );
    }

    #[test]
    fn parses_a_udp_address_without_a_path() {
        assert_eq!(
            udp_authority("udp://127.0.0.1:6969").unwrap(),
            ("127.0.0.1".to_string(), 6969)
        );
    }

    #[test]
    fn parses_an_ipv6_literal_without_confusing_its_colons_for_a_port() {
        assert_eq!(
            udp_authority("udp://[::1]:1337/announce").unwrap(),
            ("::1".to_string(), 1337)
        );
    }

    #[test]
    fn rejects_a_non_udp_tracker_address() {
        assert!(matches!(
            udp_authority("http://tracker.test:80/announce"),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn rejects_a_udp_url_without_a_port() {
        assert!(matches!(
            udp_authority("udp://tracker.test/announce"),
            Err(ScrapeError::Decode(_))
        ));
    }

    #[test]
    fn rejects_a_udp_url_with_a_non_numeric_port() {
        assert!(matches!(
            udp_authority("udp://tracker.test:http/announce"),
            Err(ScrapeError::Decode(_))
        ));
    }

    // --- UDP scrape end to end -------------------------------------------

    /// Answer one connect and one scrape on a real loopback socket.
    ///
    /// This exercises the whole UDP path -- request layout, the handshake,
    /// transaction id echo, response parsing -- against a peer that speaks the
    /// protocol, which the pure layout tests above cannot do.
    #[tokio::test]
    async fn udp_scrape_completes_a_connect_and_scrape_handshake() {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind mock tracker");
        let addr = server.local_addr().expect("mock address");

        let info_hash = hash_bytes();

        let responder = tokio::spawn(async move {
            let mut buffer = [0u8; 2048];

            // Connect: echo the transaction id and hand back a connection id.
            let (read, peer) = server.recv_from(&mut buffer).await.expect("connect");
            assert_eq!(read, 16, "connect request is 16 bytes");
            let transaction = u32::from_be_bytes(buffer[12..16].try_into().unwrap());

            let mut reply = Vec::new();
            reply.extend_from_slice(&0u32.to_be_bytes());
            reply.extend_from_slice(&transaction.to_be_bytes());
            reply.extend_from_slice(&0xABCD_1234u64.to_be_bytes());
            server.send_to(&reply, peer).await.expect("connect reply");

            // Scrape: verify the request carried the connection id from the
            // handshake and our info hash, then answer with 9/3/1.
            let (read, peer) = server.recv_from(&mut buffer).await.expect("scrape");
            assert_eq!(read, 36, "scrape request is 36 bytes");
            assert_eq!(
                u64::from_be_bytes(buffer[..8].try_into().unwrap()),
                0xABCD_1234,
                "scrape must echo the connection id from the handshake"
            );
            assert_eq!(&buffer[16..36], &info_hash[..]);

            let transaction = u32::from_be_bytes(buffer[12..16].try_into().unwrap());
            let mut reply = Vec::new();
            reply.extend_from_slice(&2u32.to_be_bytes());
            reply.extend_from_slice(&transaction.to_be_bytes());
            reply.extend_from_slice(&9u32.to_be_bytes());
            reply.extend_from_slice(&3u32.to_be_bytes());
            reply.extend_from_slice(&1u32.to_be_bytes());
            server.send_to(&reply, peer).await.expect("scrape reply");
        });

        let stats = scrape_udp(
            &format!("udp://{addr}/announce"),
            &info_hash,
            Duration::from_secs(5),
        )
        .await
        .expect("should scrape");

        assert_eq!(
            stats,
            ScrapeStats {
                seeders: 9,
                completed: 3,
                leechers: 1,
            }
        );

        responder.await.expect("responder task");
    }

    #[tokio::test]
    async fn udp_scrape_times_out_when_nothing_answers() {
        // Bind a socket but never reply to it, so the request is sent into a
        // void that is reachable but silent -- the timeout, not a connect
        // error, is what must fire.
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind silent tracker");
        let addr = server.local_addr().expect("address");

        let started = std::time::Instant::now();
        let result = scrape_udp(
            &format!("udp://{addr}/announce"),
            &hash_bytes(),
            Duration::from_millis(150),
        )
        .await;

        assert!(matches!(result, Err(ScrapeError::Transport(_))));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    // --- metadata probe ---------------------------------------------------

    #[test]
    fn an_unresolved_probe_claims_nothing_about_the_torrent() {
        // The dangerous failure mode is an unresolved probe that still
        // reports size or file count, which the ranker would read as proof
        // the torrent exists. Every field must be empty.
        let started = std::time::Instant::now();
        let probe = unresolved(&started);

        assert!(!probe.resolved);
        assert_eq!(probe.file_count, None);
        assert_eq!(probe.total_bytes, None);
    }

    #[test]
    fn metadata_probe_defaults_to_unresolved() {
        // Guards the derived `Default`: a defaulted probe must mean "we know
        // nothing", not "healthy".
        let probe = MetadataProbe::default();
        assert!(!probe.resolved);
        assert!(probe.file_count.is_none());
    }

    /// Probes a real torrent over the real DHT.
    ///
    /// Ignored by default because it starts a session, binds sockets and
    /// joins the DHT. Run with `cargo test -- --ignored`.
    ///
    /// `big-buck-bunny.torrent` sits in the repository root and is a
    /// long-lived, well-seeded torrent, so a failure here means the probe is
    /// broken rather than that the swarm died.
    #[tokio::test]
    #[ignore = "joins the DHT; run with --test-threads=1"]
    async fn probes_metadata_for_a_well_seeded_torrent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let session = std::sync::Arc::new(
            librqbit::Session::new(dir.path().to_path_buf())
                .await
                .expect("session should start"),
        );

        let magnet = "magnet:?xt=urn:btih:dd8255ecdc7ca55fb0bbf81323d87062db1f6d1c\
                     &dn=Big+Buck+Bunny";

        let probe = probe_metadata(&session, magnet, Duration::from_secs(60)).await;

        assert!(
            probe.resolved,
            "a well-seeded torrent should publish metadata within 60s"
        );
        assert!(probe.file_count.unwrap_or(0) > 0);
        assert!(probe.total_bytes.unwrap_or(0) > 0);

        // The probe must not have downloaded the torrent. This is the
        // assertion that caught the original bug: with no file selection,
        // librqbit fetched the whole release, and Big Buck Bunny is hundreds
        // of megabytes. The bound is far above any bookkeeping librqbit
        // writes and far below anything real, so a metadata-only probe
        // leaves the directory empty.
        // Wait long enough for real downloading to show up: metadata resolves
        // in seconds, so measuring immediately would make this pass even when
        // the torrent is fetching. With the guard removed, ten seconds is
        // enough for hundreds of megabytes to land.
        tokio::time::sleep(Duration::from_secs(10)).await;
        let downloaded = total_bytes_in(dir.path());
        assert!(
            downloaded < 1_000_000,
            "the metadata probe downloaded {downloaded} bytes; it must fetch none"
        );

        session.stop().await;
    }

    /// Sum the size of every file under `root`, recursively.
    fn total_bytes_in(root: &std::path::Path) -> u64 {
        let mut total = 0;
        let Ok(entries) = std::fs::read_dir(root) else {
            return 0;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => total += total_bytes_in(&path),
                Ok(_) => total += entry.metadata().map(|m| m.len()).unwrap_or(0),
                Err(_) => {}
            }
        }
        total
    }

    /// A magnet nothing is seeding must time out, not hang.
    ///
    /// Ignored for the same reason as the test above. A syntactically valid
    /// but random info hash has no peers, so the only possible outcome is the
    /// timeout.
    #[tokio::test]
    #[ignore = "joins the DHT; run with --test-threads=1"]
    async fn a_dead_magnet_times_out() {
        let dir = tempfile::tempdir().expect("temp dir");
        let session = std::sync::Arc::new(
            librqbit::Session::new(dir.path().to_path_buf())
                .await
                .expect("session should start"),
        );

        let magnet = "magnet:?xt=urn:btih:0000000000000000000000000000000000000001";

        let started = std::time::Instant::now();
        let probe = probe_metadata(&session, magnet, Duration::from_secs(2)).await;

        assert!(!probe.resolved);
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "the probe must return on its own timeout"
        );

        session.stop().await;
    }

    // --- orchestrator -----------------------------------------------------

    fn release_with_magnet(title: &str, magnet: &str) -> Release {
        crate::types::Release {
            title: title.to_string(),
            indexer: crate::types::ProviderId::Nyaa,
            magnet_uri: magnet.to_string(),
            torrent_url: None,
            info_hash: None,
            size_bytes: None,
            seeders: Some(10),
            leechers: None,
            resolution: crate::types::Resolution::R1080p,
            source: crate::types::ReleaseSource::WebDl,
            remux: false,
            trusted: false,
            parsed: Default::default(),
            score: 0,
        }
    }

    fn magnet_for(hash: &str, trackers: &[&str]) -> String {
        let mut uri = format!("magnet:?xt=urn:btih:{hash}");
        for tracker in trackers {
            uri.push_str(&format!("&tr={}", percent_encode_bytes(tracker.as_bytes())));
        }
        uri
    }

    #[test]
    fn scrape_release_is_none_without_trackers() {
        // Nothing to ask. Returning `None` is the honest answer; it must not
        // be an error, since a magnet with no tracker is still playable over
        // DHT.
        let client = reqwest::Client::new();
        let release = release_with_magnet("no trackers", &format!("magnet:?xt=urn:btih:{HASH}"));
        let semaphore = Arc::new(Semaphore::new(1));

        let result = tokio_test::block_on(scrape_release(&client, &release, HASH, &semaphore));
        assert!(result.is_none());
    }

    #[test]
    fn scrape_release_is_none_for_an_unparseable_hash() {
        let client = reqwest::Client::new();
        let release = release_with_magnet(
            "bad hash",
            &magnet_for("not-a-valid-hash", &["udp://tracker.test:80/announce"]),
        );
        let semaphore = Arc::new(Semaphore::new(1));

        let result = tokio_test::block_on(scrape_release(
            &client,
            &release,
            "not-a-valid-hash",
            &semaphore,
        ));
        assert!(result.is_none());
    }

    /// The concurrency cap must actually cap.
    ///
    /// Asserted by holding every permit and checking that a scrape to a
    /// tracker that *would* answer does not complete, then releasing one
    /// permit and checking that it does. A test that only asserted "it
    /// returned" would pass with no semaphore at all.
    #[tokio::test]
    async fn scrape_release_waits_for_a_permit() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(scrape_response(3, 0, 1)))
            .mount(&server)
            .await;

        let semaphore = Arc::new(Semaphore::new(1));
        // Take the only permit, so the scrape below must block.
        let held = Arc::clone(&semaphore)
            .acquire_owned()
            .await
            .expect("permit");

        let release = release_with_magnet(
            "held",
            &magnet_for(HASH, &[&format!("{}/announce", server.uri())]),
        );
        let client = reqwest::Client::new();

        let mut task = tokio::spawn({
            let client = client.clone();
            let release = release.clone();
            let semaphore = Arc::clone(&semaphore);
            async move { scrape_release(&client, &release, HASH, &semaphore).await }
        });

        // With no permit free, the scrape cannot have completed. A short wait
        // is enough: an unblocked scrape against a local mock returns in
        // microseconds.
        assert!(
            tokio::time::timeout(Duration::from_millis(200), &mut task)
                .await
                .is_err(),
            "the scrape should be blocked on the semaphore"
        );

        // Release the permit; the scrape should now finish.
        drop(held);
        let result = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("scrape should finish once a permit is free")
            .expect("task should not panic");

        assert!(result.is_some(), "the mock tracker should have answered");
    }

    /// Probes a real batch of releases over the real network.
    ///
    /// Ignored by default. Run with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "joins the DHT; run with --test-threads=1"]
    async fn probes_a_batch_of_releases_without_losing_any() {
        let dir = tempfile::tempdir().expect("temp dir");
        let session = Arc::new(
            librqbit::Session::new(dir.path().to_path_buf())
                .await
                .expect("session should start"),
        );

        // Big Buck Bunny, a long-lived well-seeded torrent, repeated so the
        // batch has several members that should all probe successfully.
        let magnet = magnet_for(
            "dd8255ecdc7ca55fb0bbf81323d87062db1f6d1c",
            &["udp://tracker.opentrackr.org:1337/announce"],
        );

        let releases: Vec<Release> = (0..3)
            .map(|i| release_with_magnet(&format!("bbb-{i}"), &magnet))
            .collect();

        // Collect the callback's reports separately from the returned list, so
        // the two can be compared: the callback is what drives progressive
        // re-ranking, and it must see every release the return value does.
        let mut reported: Vec<usize> = Vec::new();
        let results = probe_releases_reporting(
            &session,
            reqwest::Client::new(),
            &releases,
            |index, _| reported.push(index),
        )
        .await;

        assert_eq!(results.len(), 3, "every release must yield a result");
        // Indices must come back in input order even though the tasks finish
        // in whatever order the network allows.
        assert_eq!(
            results.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(
            results
                .iter()
                .any(|(_, r)| r.metadata.as_ref().is_some_and(|m| m.resolved)),
            "at least one probe should have resolved metadata"
        );

        // The callback must have fired once per release. Without this, a
        // callback that never runs -- or runs for only some releases -- would
        // leave the UI's list permanently partly ranked, and nothing else in
        // the test would notice.
        reported.sort_unstable();
        assert_eq!(
            reported,
            vec![0, 1, 2],
            "the progress callback must report every release exactly once"
        );

        session.stop().await;
    }
}
