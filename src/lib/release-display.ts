// Presentation helpers for a release row: the health dot's colour, the row's
// emphasis for how explicitly its name states the episode, the hover text
// behind each, and a human byte size.
//
// Pure functions of a `Release` / `ProbeOutcome`, so they are testable without
// rendering and can be shared by the watch page and the episode modal. The
// Tailwind classes returned here are the app's, not this module's — the point
// is that both call sites draw a badge the same way.
//
// Distinct from `release-match.ts`, which decides HOW explicit a name is;
// this only decides how to DRAW that verdict.

import type { HealthBadge, ProbeOutcome } from "./types";
import type { MatchClarity } from "./release-match";

/** The colour a badge dot is drawn in, or a muted pulse while unprobed. */
export function badgeClass(badge: HealthBadge | undefined): string {
  switch (badge) {
    case "green":
      return "bg-health-green";
    case "yellow":
      return "bg-health-yellow";
    case "red":
      return "bg-health-red";
    default:
      // Unprobed is not the same as dead, so it gets a neutral pulse rather
      // than a verdict colour.
      return "bg-ink-faint animate-pulse";
  }
}

/**
 * The row's emphasis for how explicitly its name states the episode.
 *
 * `stated` rows get the accent border, which already means "this one" elsewhere
 * in the app. `unclear` rows dim, so the eye skips them. The ordinary `episode`
 * case is left as it was — marking every row would defeat the point of marking
 * any.
 */
export function clarityClass(clarity: MatchClarity): string {
  switch (clarity) {
    case "stated":
      return "border-accent/70 ring-1 ring-accent/40";
    case "unclear":
      return "opacity-60";
    case "episode":
      return "";
  }
}

/**
 * A one-line explanation of a release's probe, for the `title` attribute.
 *
 * Mirrors how the clarity label explains itself: the visual is a glanceable
 * signal, and the prose lives on hover so the row stays compact.
 */
export function badgeTitle(outcome: ProbeOutcome | undefined): string {
  if (!outcome) return "Checking swarm health…";

  const parts: string[] = [];
  const seeders = outcome.probe.scrape?.seeders;
  if (seeders !== undefined) parts.push(`${seeders} seeders on tracker`);
  parts.push(
    outcome.probe.metadata?.resolved
      ? "metadata confirmed"
      : "metadata not found",
  );
  return parts.join(" · ");
}

/** A human size for a release row, e.g. "1.4 GB". */
export function formatSize(bytes?: number): string | undefined {
  if (bytes === undefined || bytes <= 0) return undefined;
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value < 10 && unit > 0 ? 1 : 0)} ${units[unit]}`;
}