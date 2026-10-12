//! An internet speed test: latency and download throughput.
//!
//! Measures against Cloudflare's public speed-test endpoints, the same ones
//! behind <https://speed.cloudflare.com>. The measurement is deliberately
//! small -- latency plus download -- because that is what streaming depends
//! on; upload is not what the reader is asking when a download stalls.
//!
//! Runs in Rust, not the webview: the webview's CSP `connect-src` blocks
//! arbitrary external hosts, and the browser timing API the JS speed test uses
//! is not available here. `reqwest` is exempt from the webview CSP, so the
//! measurement is neither sandboxed nor CSP-bound. The protocol is
//! reimplemented with our own `Instant` timing.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;

/// Cloudflare's download endpoint. `?bytes=N` returns N bytes; `bytes=0` is
/// used for latency, since it returns headers (and so a round trip) with no
/// body.
pub const DOWN_URL: &str = "https://speed.cloudflare.com/__down";

/// Event the backend emits with each [`SpeedTestProgress`] update.
pub const PROGRESS_EVENT: &str = "speedtest-progress";

/// Cloudflare's `__down` returns 0 bytes unless a `Referer` is sent. This is
/// not a secret; it is what the browser sends and what their own page sends.
const REFERER: &str = "https://speed.cloudflare.com/";

/// A recognizable User-Agent, in case the endpoint ever filters on it.
const USER_AGENT: &str = concat!("Kitsune/", env!("CARGO_PKG_VERSION"));

/// Connection timeout, so an unreachable endpoint fails fast rather than
/// hanging the test.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// The tunable knobs of a measurement.
///
/// A struct rather than constants so a test can shrink every dimension -- a
/// tiny budget, few samples -- and finish fast without duplicating the
/// measurement code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedTestConfig {
    /// How many latency requests to sample.
    pub latency_samples: usize,
    /// First payload size for the download ramp-up, in bytes.
    pub first_payload_bytes: usize,
    /// Largest payload requested, in bytes.
    pub max_payload_bytes: usize,
    /// Wall-clock budget for the download phase.
    pub download_budget: Duration,
}

impl Default for SpeedTestConfig {
    /// The shipped defaults: enough samples for a stable median, a ramp from
    /// 1 MB to 25 MB, and an 8-second download budget.
    fn default() -> Self {
        Self {
            latency_samples: 15,
            first_payload_bytes: 1_000_000,
            max_payload_bytes: 25_000_000,
            download_budget: Duration::from_secs(8),
        }
    }
}

/// A coarse verdict on the measured connection, for a one-line summary.
///
/// Serialisable as `"good" | "ok" | "poor"` so the frontend can map it to a
/// sentence without duplicating the thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeedVerdict {
    Good,
    Ok,
    Poor,
}

/// The result of a completed speed test.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedTestResult {
    /// Median round-trip time across the latency samples, in milliseconds.
    pub latency_ms: f64,
    /// Half the spread of the latency samples, in milliseconds. A small jitter
    /// means a steady connection.
    pub jitter_ms: f64,
    /// Download throughput in megabits per second (decimal: 1 Mbps = 1e6 bps).
    pub download_mbps: f64,
    /// Total bytes received during the download phase.
    pub bytes_downloaded: u64,
    /// Wall-clock time the download phase took, in milliseconds.
    pub duration_ms: u64,
    /// The one-word verdict for the measured numbers.
    pub verdict: SpeedVerdict,
}

/// One progress update while a test runs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedTestProgress {
    /// `"latency"`, `"download"` or `"done"`.
    pub phase: String,
    /// Overall completion, 0..100.
    pub percent: u8,
    /// The median latency so far, once any sample exists.
    pub latency_ms: Option<f64>,
    /// The running download rate, once the download phase has begun.
    pub download_mbps: Option<f64>,
}

/// Median of a set of samples, ignoring non-finite values.
///
/// The median rather than the mean: one slow sample (a scheduler hiccup, a
/// lost packet) should not drag the reported latency.
pub fn median(values: &[f64]) -> Option<f64> {
    let mut finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return None;
    }
    finite.sort_by(|a, b| a.partial_cmp(b).expect("finite values compare"));
    let mid = finite.len() / 2;
    Some(if finite.len() % 2 == 1 {
        finite[mid]
    } else {
        (finite[mid - 1] + finite[mid]) / 2.0
    })
}

/// Throughput in megabits per second, rounded to one decimal.
///
/// Decimal megabits (1e6 bits), matching how connections are sold and how a
/// speed test is normally read. A non-positive duration yields `0.0` rather
/// than an infinity.
pub fn throughput_mbps(bytes: u64, secs: f64) -> f64 {
    if !secs.is_finite() || secs <= 0.0 {
        return 0.0;
    }
    let bits = bytes as f64 * 8.0;
    let mbps = bits / secs / 1_000_000.0;
    (mbps * 10.0).round() / 10.0
}

/// The next payload size to request: double the current, capped.
///
/// Ramp-up: a small first request warms the connection and gives a number on a
/// slow link, then the size grows so a fast link gets a long enough transfer to
/// measure accurately.
pub fn next_payload_size(current: usize, cap: usize) -> usize {
    current.saturating_mul(2).min(cap)
}

/// A one-word verdict from the measured download rate and latency.
///
/// Download dominates: streaming is throughput-bound, so a fast link with
/// mediocre latency is still fine. Latency only demotes a would-be `Good`
/// result when it is bad enough to make the connection feel sluggish.
pub fn verdict(download_mbps: f64, latency_ms: f64) -> SpeedVerdict {
    if download_mbps >= 25.0 && latency_ms <= 200.0 {
        SpeedVerdict::Good
    } else if download_mbps >= 8.0 {
        SpeedVerdict::Ok
    } else {
        SpeedVerdict::Poor
    }
}

/// A client configured for a speed test: a connect timeout so an unreachable
/// endpoint fails fast, and no overall timeout (a large download must be
/// allowed to finish).
pub fn build_client() -> Result<Client> {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .context("failed to build the speed-test HTTP client")
}

/// Run the whole measurement, emitting progress as it goes.
///
/// `down_url` is a parameter rather than a constant so a test can point it at a
/// mock server. `emit` is called with each progress update; the command passes
/// a closure that forwards it to the webview, and tests pass a collector.
pub async fn run_measurement(
    client: &Client,
    down_url: &str,
    config: &SpeedTestConfig,
    emit: impl Fn(SpeedTestProgress),
) -> Result<SpeedTestResult> {
    let latency = measure_latency(client, down_url, config, &emit).await?;
    let (download_mbps, bytes_downloaded, duration) =
        measure_download(client, down_url, config, &emit).await?;

    let result = SpeedTestResult {
        latency_ms: latency.median_ms,
        jitter_ms: latency.jitter_ms,
        download_mbps,
        bytes_downloaded,
        duration_ms: duration.as_millis() as u64,
        verdict: verdict(download_mbps, latency.median_ms),
    };

    emit(SpeedTestProgress {
        phase: "done".to_string(),
        percent: 100,
        latency_ms: Some(result.latency_ms),
        download_mbps: Some(result.download_mbps),
    });

    Ok(result)
}

/// The latency numbers from the first phase.
struct Latency {
    median_ms: f64,
    jitter_ms: f64,
}

/// Sample round-trip time with `bytes=0` requests.
async fn measure_latency(
    client: &Client,
    down_url: &str,
    config: &SpeedTestConfig,
    emit: &impl Fn(SpeedTestProgress),
) -> Result<Latency> {
    let url = format!("{down_url}?bytes=0");
    let samples_wanted = config.latency_samples.max(1);
    let mut samples: Vec<f64> = Vec::with_capacity(samples_wanted);

    for i in 0..samples_wanted {
        let started = Instant::now();
        let response = client
            .get(&url)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .context("latency request failed")?;
        // The headers are the round trip; the body (empty at bytes=0) does not
        // matter. Consuming it makes the timing honest.
        let _ = response.bytes().await;
        samples.push(started.elapsed().as_secs_f64() * 1000.0);

        // Scale in `u32`: `percent * WEIGHT` (100 * 30) overflows a `u8`.
        let done = ((i + 1) * 100 / samples_wanted).min(100) as u32;
        let percent = (done * LATENCY_PROGRESS_WEIGHT as u32 / 100) as u8;
        emit(SpeedTestProgress {
            phase: "latency".to_string(),
            percent,
            latency_ms: median(&samples),
            download_mbps: None,
        });
    }

    let median_ms = median(&samples).unwrap_or(0.0);
    let jitter_ms = if samples.len() >= 2 {
        let max = samples.iter().copied().fold(f64::MIN, f64::max);
        let min = samples.iter().copied().fold(f64::MAX, f64::min);
        (max - min) / 2.0
    } else {
        0.0
    };

    Ok(Latency { median_ms, jitter_ms })
}

/// Fraction of the overall progress the latency phase occupies.
const LATENCY_PROGRESS_WEIGHT: u8 = 30;

/// Download progressively larger payloads within the time budget.
///
/// Returns the throughput, the total bytes, and how long the phase took. The
/// body is streamed with `Response::chunk()` rather than read whole, so bytes
/// can be counted (and progress emitted) as they arrive instead of after the
/// transfer completes.
async fn measure_download(
    client: &Client,
    down_url: &str,
    config: &SpeedTestConfig,
    emit: &impl Fn(SpeedTestProgress),
) -> Result<(f64, u64, Duration)> {
    let started = Instant::now();
    let mut bytes_total: u64 = 0;
    let mut size = config.first_payload_bytes.max(1);
    let budget = config.download_budget;

    'outer: while started.elapsed() < budget {
        let url = format!("{down_url}?bytes={size}");
        let mut response = client
            .get(&url)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .context("download request failed")?;

        while let Some(chunk) = response
            .chunk()
            .await
            .context("reading a download chunk failed")?
        {
            bytes_total += chunk.len() as u64;

            let elapsed = started.elapsed();
            let rate = throughput_mbps(bytes_total, elapsed.as_secs_f64());
            let fraction = if budget.is_zero() {
                1.0
            } else {
                (elapsed.as_secs_f64() / budget.as_secs_f64()).min(1.0)
            };
            let span = (100.0 - LATENCY_PROGRESS_WEIGHT as f64) as u8;
            let percent = LATENCY_PROGRESS_WEIGHT + (fraction * span as f64) as u8;
            emit(SpeedTestProgress {
                phase: "download".to_string(),
                percent: percent.min(100),
                latency_ms: None,
                download_mbps: Some(rate),
            });

            // Stop as soon as the budget is spent, even mid-transfer: the bytes
            // and the time cover the same interval, so the rate stays valid.
            if elapsed >= budget {
                break 'outer;
            }
        }

        size = next_payload_size(size, config.max_payload_bytes);
    }

    let elapsed = started.elapsed();
    Ok((
        throughput_mbps(bytes_total, elapsed.as_secs_f64()),
        bytes_total,
        elapsed,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_odd_count_is_the_middle() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
    }

    #[test]
    fn median_of_even_count_averages_the_middle_two() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
    }

    #[test]
    fn median_ignores_non_finite_values() {
        assert_eq!(median(&[1.0, f64::NAN, 3.0, f64::INFINITY]), Some(2.0));
    }

    #[test]
    fn median_of_nothing_is_none() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[f64::NAN]), None);
    }

    #[test]
    fn throughput_is_bits_over_seconds_in_megabits() {
        // 1 MB in 1 s is 8 Mbps.
        assert_eq!(throughput_mbps(1_000_000, 1.0), 8.0);
        // 10 MB in 2 s is 40 Mbps.
        assert_eq!(throughput_mbps(10_000_000, 2.0), 40.0);
    }

    #[test]
    fn throughput_guards_a_non_positive_duration() {
        assert_eq!(throughput_mbps(1_000_000, 0.0), 0.0);
        assert_eq!(throughput_mbps(1_000_000, -1.0), 0.0);
        assert_eq!(throughput_mbps(1_000_000, f64::NAN), 0.0);
    }

    #[test]
    fn next_payload_size_doubles_then_caps() {
        assert_eq!(next_payload_size(1_000_000, 25_000_000), 2_000_000);
        assert_eq!(next_payload_size(20_000_000, 25_000_000), 25_000_000);
        assert_eq!(next_payload_size(25_000_000, 25_000_000), 25_000_000);
    }

    #[test]
    fn verdict_is_good_for_a_fast_low_latency_link() {
        assert_eq!(verdict(50.0, 20.0), SpeedVerdict::Good);
    }

    #[test]
    fn verdict_demotes_a_fast_link_with_bad_latency() {
        // Fast enough for Good, but latency over the threshold drops it to Ok.
        assert_eq!(verdict(50.0, 300.0), SpeedVerdict::Ok);
    }

    #[test]
    fn verdict_is_poor_for_a_slow_link() {
        assert_eq!(verdict(3.0, 40.0), SpeedVerdict::Poor);
    }

    #[test]
    fn verdict_serialises_lowercase() {
        let json = serde_json::to_value(SpeedVerdict::Good).unwrap();
        assert_eq!(json, serde_json::json!("good"));
        let json = serde_json::to_value(SpeedVerdict::Poor).unwrap();
        assert_eq!(json, serde_json::json!("poor"));
    }

    #[test]
    fn result_serialises_in_camel_case() {
        let result = SpeedTestResult {
            latency_ms: 12.5,
            jitter_ms: 1.5,
            download_mbps: 42.0,
            bytes_downloaded: 10_000_000,
            duration_ms: 8000,
            verdict: SpeedVerdict::Good,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("latencyMs").is_some(), "json: {json}");
        assert!(json.get("downloadMbps").is_some(), "json: {json}");
        assert!(json.get("bytesDownloaded").is_some(), "json: {json}");
        assert_eq!(json.get("verdict").unwrap(), "good");
    }

    #[test]
    fn default_config_is_the_shipped_one() {
        let c = SpeedTestConfig::default();
        assert_eq!(c.latency_samples, 15);
        assert_eq!(c.first_payload_bytes, 1_000_000);
        assert_eq!(c.max_payload_bytes, 25_000_000);
        assert_eq!(c.download_budget, Duration::from_secs(8));
    }
}