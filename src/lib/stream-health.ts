// Turning a torrent-progress snapshot into a single, human warning.
//
// The panel already shows numbers (speed, peers, ETA); this decides whether any
// of them mean something is WRONG, and says so in one line. Pure, so the rules
// are unit-testable without a torrent or a render — the same shape as
// `release-display.ts` and `torrent-files.ts`.
//
// The caller supplies `staleSeconds`: how long the byte count has not moved.
// That is the one fact not in the snapshot (it is a change over time), so it is
// tracked in the session and passed in rather than derived here.

import type { TorrentProgress } from "./types";

/**
 * How a notice is styled and how urgent it is.
 *
 * `"ok"` is a positive notice (the episode is fully buffered), not a problem:
 * it is the only level rendered in green, and it is returned INSTEAD of any
 * warning so a finished download never reads as a fault.
 */
export type WarningLevel = "info" | "warn" | "error" | "ok";

/** One thing worth telling the reader, or `null` when all is well. */
export interface StreamWarning {
  level: WarningLevel;
  /** A glyph for the banner, e.g. "⚠". */
  icon: string;
  /** The sentence to show. */
  message: string;
}

/** Below this many live peers, a download is "slow" and may stutter. */
export const FEW_PEERS = 3;

/** Below this speed (MiB/s), a download is "slow" when peers are also few. */
export const SLOW_MBPS = 0.5;

/** No bytes for this many seconds, while live, counts as stalled. */
export const STALL_SECONDS = 10;

/**
 * The one notice worth showing, or `null`.
 *
 * Ordered by severity: a fully buffered episode first (a positive "ok"
 * notice), then an error, then a stall, then slowness, then the informational
 * "still looking". Only one is returned because a single clear sentence beats
 * a stack of them; the panel shows the numbers regardless.
 *
 * `fileFraction` is the chosen file's downloaded fraction, 0..1. It is the
 * only thing that can turn a warning into a green notice.
 */
export function streamWarning(
  progress: TorrentProgress | null,
  staleSeconds: number,
  fileFraction = 0,
): StreamWarning | null {
  if (progress === null) return null;

  // A fully buffered episode is the one case that overrides everything else.
  // It MUST come first: once the file is complete, no more bytes arrive, so
  // `staleSeconds` climbs forever -- without this, a finished download would
  // eventually be reported as "stalled", which is exactly backwards.
  if (fileFraction >= 1) {
    return {
      level: "ok",
      icon: "✓",
      message:
        "Fully buffered — the whole episode is downloaded, so playback will " +
        "not be interrupted by the network.",
    };
  }

  // The backend's own failure message is the most specific thing we have.
  if (progress.state === "error") {
    return {
      level: "error",
      icon: "⛔",
      message: progress.error
        ? `Playback could not start. ${progress.error}`
        : "Playback could not start. Try another release.",
    };
  }

  // A live torrent that has stopped moving. Bytes arriving are the definition
  // of progress; if none have for a while, the swarm has gone quiet on us.
  if (progress.state === "live" && staleSeconds >= STALL_SECONDS) {
    return {
      level: "error",
      icon: "⛔",
      message:
        "Download has stalled — no data for a while. Your connection or the " +
        "swarm may be the cause. Try another release, or keep waiting.",
    };
  }

  // No live peer yet, but the client is still reaching out: not an error, just
  // patience.
  if (
    progress.peersLive === 0 &&
    (progress.peersConnecting > 0 || progress.peersQueued > 0)
  ) {
    return {
      level: "info",
      icon: "🔍",
      message:
        "Looking for peers — this can take a moment on a quiet swarm.",
    };
  }

  // Actually connected, but barely: few peers and little speed means the file
  // may not arrive fast enough to keep playback smooth.
  if (
    progress.peersLive > 0 &&
    progress.peersLive <= FEW_PEERS &&
    progress.downloadMbps < SLOW_MBPS
  ) {
    return {
      level: "warn",
      icon: "⚠",
      message:
        `Slow download — only ${progress.peersLive} peer` +
        `${progress.peersLive === 1 ? "" : "s"} responding. Playback may ` +
        "stutter; a release with more seeders would be safer.",
    };
  }

  // Nothing wrong to report.
  return null;
}

/** The health word shown beside the numbers, derived from the same facts. */
export function healthLabel(progress: TorrentProgress | null): string {
  if (progress === null) return "Waiting";
  if (progress.state === "error") return "Error";
  if (progress.peersLive > 0) {
    return progress.downloadMbps < SLOW_MBPS && progress.peersLive <= FEW_PEERS
      ? "Slow"
      : "Healthy";
  }
  if (progress.peersConnecting > 0 || progress.peersQueued > 0) {
    return "Connecting";
  }
  return "Searching";
}