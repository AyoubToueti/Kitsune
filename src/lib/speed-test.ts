// Presentation helpers for the internet speed test.
//
// The measurement itself lives in Rust; this turns its numbers into the short
// sentences and labels the UI shows. Pure, so the wording and the thresholds
// are unit-tested without a network -- the same shape as `stream-health.ts`
// and `release-display.ts`.

import type { SpeedTestResult, SpeedVerdict } from "./types";

/** A one-line summary of a finished test, for a banner. */
export interface SpeedSummary {
  /** How the banner is styled. */
  level: "good" | "ok" | "poor";
  /** A glyph for the banner. */
  icon: string;
  /** The sentence to show. */
  message: string;
}

/**
 * A plain-language summary of a finished test.
 *
 * The verdict is decided in Rust so the threshold lives in one place; this only
 * chooses the words and the styling. `latencyMs` is mentioned when it is the
 * reason a fast link is not rated "good", so the reader is not left wondering.
 */
export function speedSummary(result: SpeedTestResult): SpeedSummary {
  const mbps = result.downloadMbps.toFixed(0);
  switch (result.verdict) {
    case "good":
      return {
        level: "good",
        icon: "✓",
        message: `${mbps} Mbps — comfortable for streaming, even in 1080p.`,
      };
    case "ok":
      return {
        level: "ok",
        icon: "•",
        message: `${mbps} Mbps — fine for streaming, though a 1080p stream may need to buffer first.`,
      };
    case "poor":
      return {
        level: "poor",
        icon: "⚠",
        message: `${mbps} Mbps — this may stutter. Streaming will be more reliable on a faster connection.`,
      };
  }
}

/**
 * A short verdict label for the inline result next to a button.
 *
 * Compact by design: the modal footer has no room for a sentence, so it shows
 * `↓ 42 Mbps · 18 ms` regardless of the verdict -- the numbers speak for
 * themselves there.
 */
export function compactResult(result: SpeedTestResult): string {
  return `↓ ${result.downloadMbps.toFixed(0)} Mbps · ${result.latencyMs.toFixed(0)} ms`;
}

/** A verdict as a CSS colour class, for the banner text. */
export function verdictClass(verdict: SpeedVerdict): string {
  switch (verdict) {
    case "good":
      return "text-health-green";
    case "ok":
      return "text-health-yellow";
    case "poor":
      return "text-danger";
  }
}