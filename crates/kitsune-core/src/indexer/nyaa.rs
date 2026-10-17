//! The Nyaa indexer: search nyaa.si and turn its RSS into [`Release`]s.
//!
//! Nyaa exposes a plain RSS feed, so this is a thin transport plus a parser.
//! The parser is split out and pure, so the feed's shape can be asserted in
//! tests without a network, and only the HTTP call is left untested.
//!
//! Only the "Anime / English-translated" category is queried (`c=1_2`): the
//! app is a player for subtitled releases, and a raw crawl of the whole site
//! would spend the request budget on raw Japanese and live-action uploads.

use async_trait::async_trait;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;

use super::parse::parse_release;
use super::quality::parse_quality;
use super::traits::{Indexer, IndexerError};
use crate::torrent::build_magnet;
use crate::types::{ProviderId, Release};

/// Public Nyaa mirror used by default. Overridable so tests can point at a
/// mock server without reaching the real site.
pub const NYAA_ENDPOINT: &str = "https://nyaa.si";

/// Nyaa's "Anime - English-translated" category id.
const CATEGORY_ANIME_ENGLISH: &str = "1_2";

/// Trackers added to a magnet so peer discovery does not depend on DHT alone.
///
/// None of these is authoritative; they are the long-lived public trackers
/// Nyaa's own torrents list, repeated here so a magnet built from an info
/// hash alone still finds peers.
const TRACKERS: &[&str] = &[
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://open.tracker.cl:1337/announce",
    "udp://9.rarbg.com:2810/announce",
    "udp://tracker.openbittorrent.com:6969/announce",
];

/// One `<item>` from the Nyaa RSS feed, before it is turned into a release.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NyaaItem {
    pub title: String,
    /// The torrent's info hash, hex. Empty when the feed omitted it.
    pub info_hash: String,
    /// Direct `.torrent` download URL, as the feed's `<link>` spelled it.
    ///
    /// Empty when the feed omitted it. Kept apart from the info hash because
    /// the two are not interchangeable: the link is a URL the UI can save,
    /// while the hash is what a magnet is built from.
    pub torrent_url: String,
    /// Display size as the feed spelled it, e.g. "1.4 GiB".
    pub size: String,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    /// When the torrent was posted, as unix seconds.
    ///
    /// Parsed from the feed's `<pubDate>` (RFC 2822). `None` when the element
    /// is missing or unparseable, so a bad date never discards the release --
    /// the list can still sort on every other field.
    pub published_at: Option<i64>,
    /// Nyaa's "trusted uploader" flag.
    pub trusted: bool,
}

/// A Nyaa indexer, holding its own HTTP client so connections are pooled.
pub struct NyaaIndexer {
    client: Client,
    endpoint: String,
}

impl NyaaIndexer {
    /// An indexer pointed at the real site.
    pub fn new() -> Self {
        Self::with_endpoint(NYAA_ENDPOINT)
    }

    /// An indexer pointed elsewhere, for tests.
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
        }
    }
}

impl Default for NyaaIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Indexer for NyaaIndexer {
    fn name(&self) -> &str {
        "nyaa"
    }

    async fn search(&self, query: &str) -> Result<Vec<Release>, IndexerError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(IndexerError::InvalidQuery("empty search query".into()));
        }

        let url = search_url(&self.endpoint, trimmed);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| IndexerError::Transport(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| IndexerError::Transport(e.to_string()))?;

        if !status.is_success() {
            return Err(IndexerError::Status {
                status: status.as_u16(),
                body,
            });
        }

        let items = parse_rss(&body)?;
        Ok(items.iter().filter_map(item_to_release).collect())
    }
}

/// Build the RSS search URL for a query.
///
/// The query string is assembled and percent-encoded by hand rather than with
/// `RequestBuilder::query`, which is not compiled in with the features this
/// crate enables. Hand-rolling also keeps the encoding rule visible and
/// testable: a title with a space or an accented character must survive the
/// round trip, and a bare `format!` would leave it unencoded.
fn search_url(endpoint: &str, query: &str) -> String {
    format!(
        "{}/?page=rss&q={}&c={CATEGORY_ANIME_ENGLISH}&f=0",
        endpoint.trim_end_matches('/'),
        encode_component(query),
    )
}

/// Percent-encode one query-string component per RFC 3986.
///
/// Only the unreserved set (letters, digits, `-`, `_`, `.`, `~`) is left
/// alone; everything else, including a space and every multi-byte UTF-8
/// character, is encoded byte by byte. That last part matters: a romaji title
/// with a macron or a CJK title must not be sent raw.
fn encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Turn one feed item into a release, or `None` when it cannot be played.
///
/// An item with no info hash yields no release: without it there is no magnet
/// and nothing to hand the torrent engine, so a half-built release would only
/// fail later with a worse message.
pub fn item_to_release(item: &NyaaItem) -> Option<Release> {
    let magnet_uri = build_magnet_uri(&item.info_hash);
    let magnet = build_magnet(
        &magnet_uri,
        Some(item.title.clone()),
        parse_size(&item.size),
        item.seeders,
        item.leechers,
    )
    .ok()?;

    let quality = parse_quality(&item.title);
    let parsed = parse_release(&item.title);

    Some(Release {
        title: item.title.clone(),
        indexer: ProviderId::Nyaa,
        magnet_uri: magnet.uri,
        torrent_url: if item.torrent_url.is_empty() {
            None
        } else {
            Some(item.torrent_url.clone())
        },
        info_hash: magnet.info_hash,
        size_bytes: magnet.size_bytes,
        seeders: magnet.seeders,
        leechers: magnet.leechers,
        published_at: item.published_at,
        resolution: quality.resolution,
        source: quality.source,
        remux: quality.remux,
        trusted: item.trusted,
        parsed,
        score: 0,
    })
}

/// Build a magnet URI from a bare info hash and the public tracker list.
///
/// The display name is deliberately omitted: [`build_magnet`] is given the
/// feed's title directly, so putting it in the URI too would only risk a
/// second, differently-encoded copy disagreeing with the first.
fn build_magnet_uri(info_hash: &str) -> String {
    let mut uri = format!("magnet:?xt=urn:btih:{info_hash}");
    for tracker in TRACKERS {
        uri.push_str("&tr=");
        uri.push_str(tracker);
    }
    uri
}

/// Parse the Nyaa RSS feed into items.
///
/// Namespaced elements (`nyaa:seeders`) are matched on their full local name,
/// since the feed always uses the `nyaa` prefix. A feed with an unexpected
/// element is ignored rather than failing the whole parse: a missing field is
/// recoverable, a discarded result set is not.
pub fn parse_rss(xml: &str) -> Result<Vec<NyaaItem>, IndexerError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut items = Vec::new();
    let mut current: Option<NyaaItem> = None;
    let mut field: Option<String> = None;

    loop {
        let event = reader
            .read_event()
            .map_err(|e| IndexerError::Decode(e.to_string()))?;

        match event {
            Event::Start(element) => {
                let name = local_name(element.name().as_ref());
                match name.as_str() {
                    "item" => current = Some(NyaaItem::default()),
                    "title" | "link" | "pubDate" | "nyaa:infoHash" | "nyaa:size"
                    | "nyaa:seeders" | "nyaa:leechers" | "nyaa:trusted" => {
                        field = Some(name);
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                let Some(name) = field.as_deref() else {
                    continue;
                };
                let Some(item) = current.as_mut() else {
                    continue;
                };

                let value = text
                    .unescape()
                    .map_err(|e| IndexerError::Decode(e.to_string()))?
                    .into_owned();

                match name {
                    "title" => item.title = value,
                    // The channel also carries a `<link>`, but `current` is
                    // only set inside an `<item>`, so a channel-level link is
                    // ignored rather than mistaken for a torrent URL.
                    "link" => item.torrent_url = value.trim().to_string(),
                    "pubDate" => item.published_at = parse_rfc2822_utc(value.trim()),
                    "nyaa:infoHash" => item.info_hash = value.trim().to_ascii_lowercase(),
                    "nyaa:size" => item.size = value,
                    "nyaa:seeders" => item.seeders = value.trim().parse().ok(),
                    "nyaa:leechers" => item.leechers = value.trim().parse().ok(),
                    "nyaa:trusted" => item.trusted = value.trim().eq_ignore_ascii_case("yes"),
                    _ => {}
                }
            }
            Event::End(element) => {
                let name = local_name(element.name().as_ref());
                match name.as_str() {
                    "item" => {
                        if let Some(item) = current.take() {
                            items.push(item);
                        }
                    }
                    "title" | "link" | "pubDate" | "nyaa:infoHash" | "nyaa:size"
                    | "nyaa:seeders" | "nyaa:leechers" | "nyaa:trusted" => field = None,
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    // `quick-xml` reports a truncated document as a plain end of input rather
    // than an error, so an item or field still open here means the feed was
    // cut off mid-element. Failing loudly beats returning the items that
    // happened to complete, which would silently drop the rest of the feed.
    if current.is_some() || field.is_some() {
        return Err(IndexerError::Decode(
            "unexpected end of input inside an element".into(),
        ));
    }

    Ok(items)
}

/// The element name as a `String`, matching the feed's prefixed spelling.
fn local_name(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).into_owned()
}

/// Parse an RFC 2822 timestamp into unix seconds.
///
/// The feed spells dates like `Tue, 13 Oct 2026 20:13:27 -0000`. Hand-rolled
/// rather than pulling in a date crate: the grammar here is fixed and narrow,
/// and the rest of this module already parses by hand (`parse_size`,
/// `encode_component`).
///
/// Returns `None` for anything unexpected, so a malformed date degrades to "no
/// date" instead of discarding the release.
pub fn parse_rfc2822_utc(input: &str) -> Option<i64> {
    // Drop the optional weekday prefix ("Tue, "). Splitting on the comma and
    // taking the tail keeps this independent of which weekday name is used.
    let rest = match input.split_once(',') {
        Some((_, tail)) => tail.trim(),
        None => input.trim(),
    };

    let mut parts = rest.split_whitespace();
    let day: i64 = parts.next()?.parse().ok()?;
    let month = month_number(parts.next()?)?;
    let year: i64 = parts.next()?.parse().ok()?;
    let clock = parts.next()?;
    // Some feeds omit the zone; treat that as UTC rather than failing.
    let zone = parts.next().unwrap_or("+0000");

    let mut hms = clock.split(':');
    let hour: i64 = hms.next()?.parse().ok()?;
    let minute: i64 = hms.next()?.parse().ok()?;
    // Seconds are optional in RFC 2822; a missing one means :00.
    let second: i64 = match hms.next() {
        Some(s) => s.parse().ok()?,
        None => 0,
    };

    if !(1..=31).contains(&day) || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute) || !(0..=60).contains(&second)
    {
        return None;
    }

    let offset_secs = parse_zone_offset(zone)?;

    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset_secs)
}

/// The month number for a three-letter English month name.
fn month_number(name: &str) -> Option<i64> {
    let lower = name.to_ascii_lowercase();
    let index = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|month| *month == lower)?;
    Some(index as i64 + 1)
}

/// The zone offset in seconds east of UTC, from `+HHMM`, `-HHMM` or `+HH:MM`.
///
/// The named zones RFC 2822 also allows ("GMT", "UT") are treated as UTC;
/// anything else is rejected rather than silently mis-dated.
fn parse_zone_offset(zone: &str) -> Option<i64> {
    if zone.eq_ignore_ascii_case("gmt") || zone.eq_ignore_ascii_case("ut") {
        return Some(0);
    }

    let sign = match zone.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits: String = zone[1..].chars().filter(|c| *c != ':').collect();
    if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let hours: i64 = digits[..2].parse().ok()?;
    let minutes: i64 = digits[2..].parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 3600 + minutes * 60))
}

/// Days since the Unix epoch for a civil date (Howard Hinnant's algorithm).
///
/// Avoids a leap-year table by shifting the year to start in March, which puts
/// the leap day at the END of the shifted year and makes the month arithmetic
/// uniform.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Parse a human size such as `1.4 GiB` or `750 MiB` into bytes.
///
/// Returns `None` for anything that is not a number followed by a known unit,
/// because a wrong size would mislead the viewer more than an absent one.
pub fn parse_size(size: &str) -> Option<u64> {
    let mut parts = size.split_whitespace();
    let number: f64 = parts.next()?.parse().ok()?;
    let unit = parts.next()?.to_ascii_lowercase();

    let multiplier: f64 = match unit.as_str() {
        "b" => 1.0,
        "kib" | "kb" => 1024.0,
        "mib" | "mb" => 1024.0 * 1024.0,
        "gib" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "tib" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((number * multiplier).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:nyaa="https://nyaa.si/xmlns/nyaa">
  <channel>
    <item>
      <title>[SubsPlease] Show - 05 (1080p) [ABCD1234].mkv</title>
      <link>https://nyaa.si/download/123.torrent</link>
      <pubDate>Tue, 13 Oct 2026 20:13:27 -0000</pubDate>
      <nyaa:seeders>42</nyaa:seeders>
      <nyaa:leechers>3</nyaa:leechers>
      <nyaa:size>1.4 GiB</nyaa:size>
      <nyaa:trusted>Yes</nyaa:trusted>
      <nyaa:infoHash>cab507494d02ebb1178b38f2e9d7be299c86b862</nyaa:infoHash>
    </item>
    <item>
      <title>[Group] Other Show - 01 [720p]</title>
      <nyaa:seeders>1</nyaa:seeders>
      <nyaa:size>350 MiB</nyaa:size>
      <nyaa:trusted>No</nyaa:trusted>
      <nyaa:infoHash>1111111111111111111111111111111111111111</nyaa:infoHash>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn parses_every_item() {
        let items = parse_rss(SAMPLE).expect("should parse");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn reads_all_the_fields() {
        let items = parse_rss(SAMPLE).expect("should parse");
        let first = &items[0];

        assert_eq!(
            first.title,
            "[SubsPlease] Show - 05 (1080p) [ABCD1234].mkv"
        );
        assert_eq!(
            first.info_hash,
            "cab507494d02ebb1178b38f2e9d7be299c86b862"
        );
        assert_eq!(first.seeders, Some(42));
        assert_eq!(first.leechers, Some(3));
        assert_eq!(first.size, "1.4 GiB");
        assert!(first.trusted);
    }

    #[test]
    fn reads_the_torrent_download_link() {
        let items = parse_rss(SAMPLE).expect("should parse");
        assert_eq!(items[0].torrent_url, "https://nyaa.si/download/123.torrent");
        // The second item has no `<link>`, so the URL stays empty.
        assert_eq!(items[1].torrent_url, "");
    }

    #[test]
    fn a_channel_level_link_is_not_a_torrent_url() {
        // Nyaa's channel carries its own `<link>`; only a link inside an
        // `<item>` names a torrent.
        let xml = r#"<?xml version="1.0"?>
<rss><channel>
  <link>https://nyaa.si/</link>
  <item>
    <title>Show - 01</title>
    <nyaa:infoHash>cab507494d02ebb1178b38f2e9d7be299c86b862</nyaa:infoHash>
  </item>
</channel></rss>"#;
        let items = parse_rss(xml).expect("should parse");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].torrent_url, "");
    }

    #[test]
    fn trusted_no_is_false() {
        let items = parse_rss(SAMPLE).expect("should parse");
        assert!(!items[1].trusted);
    }

    /// 2026-10-13T20:13:27Z, as the feed spells it.
    ///
    /// Cross-checked against the civil-date arithmetic: 2026-01-01 is
    /// 1767225600, plus 285 days to 2026-10-13, plus 20:13:27.
    const EXPECTED_PUBLISHED_AT: i64 = 1_791_922_407;

    #[test]
    fn reads_the_publish_date() {
        let items = parse_rss(SAMPLE).expect("should parse");
        assert_eq!(items[0].published_at, Some(EXPECTED_PUBLISHED_AT));
        // The second item carries no `<pubDate>`.
        assert_eq!(items[1].published_at, None);
    }

    #[test]
    fn the_publish_date_reaches_the_release() {
        let items = parse_rss(SAMPLE).expect("should parse");
        let release = item_to_release(&items[0]).expect("should build");
        assert_eq!(release.published_at, Some(EXPECTED_PUBLISHED_AT));
    }

    #[test]
    fn parses_rfc2822_without_a_weekday() {
        // The weekday prefix is optional, and seconds may be absent.
        assert_eq!(
            parse_rfc2822_utc("13 Oct 2026 20:13 -0000"),
            Some(EXPECTED_PUBLISHED_AT - 27),
        );
    }

    #[test]
    fn parses_a_non_utc_offset() {
        // +0200 is two hours ahead of UTC, so the same wall clock is 7200 s
        // EARLIER in unix time.
        assert_eq!(
            parse_rfc2822_utc("Tue, 13 Oct 2026 22:13:27 +0200"),
            Some(EXPECTED_PUBLISHED_AT),
        );
    }

    #[test]
    fn parses_gmt_as_utc() {
        assert_eq!(
            parse_rfc2822_utc("Tue, 13 Oct 2026 20:13:27 GMT"),
            Some(EXPECTED_PUBLISHED_AT),
        );
    }

    #[test]
    fn a_malformed_date_is_none() {
        for bad in [
            "",
            "not a date",
            "Tue, 13 Foo 2026 20:13:27 -0000",
            "Tue, 13 Oct 2026 99:13:27 -0000",
            "Tue, 13 Oct 2026 20:13:27 +9999",
            "Tue, 13 Oct 2026 20:13:27 XYZ",
        ] {
            assert_eq!(parse_rfc2822_utc(bad), None, "should reject: {bad}");
        }
    }

    #[test]
    fn a_bad_publish_date_does_not_discard_the_item() {
        let xml = SAMPLE.replace(
            "Tue, 13 Oct 2026 20:13:27 -0000",
            "sometime last tuesday",
        );
        let items = parse_rss(&xml).expect("should still parse");
        assert_eq!(items[0].published_at, None);
        assert_eq!(items[0].seeders, Some(42));
    }

    #[test]
    fn info_hash_is_lowercased() {
        let xml = SAMPLE.replace(
            "cab507494d02ebb1178b38f2e9d7be299c86b862",
            "CAB507494D02EBB1178B38F2E9D7BE299C86B862",
        );
        let items = parse_rss(&xml).expect("should parse");
        assert_eq!(
            items[0].info_hash,
            "cab507494d02ebb1178b38f2e9d7be299c86b862"
        );
    }

    #[test]
    fn an_empty_feed_yields_no_items() {
        let xml = r#"<?xml version="1.0"?><rss><channel></channel></rss>"#;
        assert_eq!(parse_rss(xml).expect("should parse"), Vec::new());
    }

    #[test]
    fn a_malformed_document_is_a_decode_error() {
        // An unclosed start tag is not well-formed XML.
        let xml = "<rss><channel><item><title>x";
        let result = parse_rss(xml);
        assert!(matches!(result, Err(IndexerError::Decode(_))));
    }

    #[test]
    fn item_becomes_a_release() {
        let items = parse_rss(SAMPLE).expect("should parse");
        let release = item_to_release(&items[0]).expect("should build");

        assert_eq!(release.indexer, ProviderId::Nyaa);
        assert_eq!(release.resolution, crate::types::Resolution::R1080p);
        assert_eq!(release.trusted, true);
        assert_eq!(release.seeders, Some(42));
        assert_eq!(release.parsed.absolute_episode, Some(5));
        assert_eq!(release.parsed.subgroup.as_deref(), Some("SubsPlease"));
    }

    #[test]
    fn the_magnet_carries_the_info_hash_and_trackers() {
        let items = parse_rss(SAMPLE).expect("should parse");
        let release = item_to_release(&items[0]).expect("should build");

        assert!(release.magnet_uri.starts_with(
            "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862"
        ));
        assert!(release.magnet_uri.contains("&tr="));
    }

    #[test]
    fn the_release_carries_the_torrent_url() {
        let items = parse_rss(SAMPLE).expect("should parse");
        let release = item_to_release(&items[0]).expect("should build");
        assert_eq!(
            release.torrent_url.as_deref(),
            Some("https://nyaa.si/download/123.torrent")
        );

        // An item with no link yields `None`, not an empty string.
        let without = item_to_release(&items[1]).expect("should build");
        assert_eq!(without.torrent_url, None);
    }

    #[test]
    fn an_item_without_a_hash_yields_no_release() {
        let item = NyaaItem {
            title: "Show - 01".into(),
            info_hash: String::new(),
            ..Default::default()
        };
        assert!(item_to_release(&item).is_none());
    }

    #[test]
    fn parses_human_sizes() {
        assert_eq!(parse_size("1 GiB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("750 MiB"), Some(750 * 1024 * 1024));
        assert_eq!(parse_size("100 MiB"), Some(100 * 1024 * 1024));
    }

    #[test]
    fn parses_a_fractional_size() {
        // 1.5 GiB, to the nearest byte.
        let expected = (1.5_f64 * 1024.0 * 1024.0 * 1024.0).round() as u64;
        assert_eq!(parse_size("1.5 GiB"), Some(expected));
    }

    #[test]
    fn an_unrecognised_size_is_none() {
        assert_eq!(parse_size("unknown"), None);
        assert_eq!(parse_size("1 flibbertigibbet"), None);
    }

    #[tokio::test]
    async fn search_rejects_an_empty_query() {
        let indexer = NyaaIndexer::new();
        let result = indexer.search("   ").await;
        assert!(matches!(result, Err(IndexerError::InvalidQuery(_))));
    }

    #[tokio::test]
    async fn search_reads_the_feed_from_the_endpoint() {
        use wiremock::matchers::{method, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("page", "rss"))
            .and(query_param("q", "Show"))
            .respond_with(ResponseTemplate::new(200).set_body_string(SAMPLE))
            .mount(&server)
            .await;

        let indexer = NyaaIndexer::with_endpoint(server.uri());
        let releases = indexer.search("Show").await.expect("should succeed");

        assert_eq!(releases.len(), 2);
    }

    #[tokio::test]
    async fn search_surfaces_a_non_success_status() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503).set_body_string("down"))
            .mount(&server)
            .await;

        let indexer = NyaaIndexer::with_endpoint(server.uri());
        let result = indexer.search("Show").await;

        assert!(matches!(result, Err(IndexerError::Status { status: 503, .. })));
    }
}