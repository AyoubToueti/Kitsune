// How explicitly a release names the episode it is for.
//
// This grades EXPLICITNESS, not validity. Validity was already decided by the
// backend's matcher (`match_spec.rs`), which drops anything that is not the
// requested episode -- so every release this sees is a genuine match. What
// differs is how much the name actually *says*.
//
// The distinction matters because an untagged release is accepted for a season
// request: `Show - 09` states no season, and an absent season is not a
// disagreement. But that release may equally be a different season's episode 9.
// A name that says `S03E09` has no such ambiguity, so it is worth marking.
//
// Distinct from `release-filter.ts` (substring filtering) and `resolution.ts`
// (resolution chips). Pure, so the rule is testable without rendering.
//
// The work is already done: `parse.rs` sets `season` AND `episode` only for
// `SxxExx`-shaped names, so this reads fields the backend already sent rather
// than re-parsing the title.

import type { Release } from "$lib/types";

/**
 * How much a release's name tells us about which episode it is.
 *
 * Ordered best-first, so a caller could compare two if it ever needed to.
 */
export type MatchClarity = "stated" | "episode" | "unclear";

/**
 * How explicit a release name is about its season and episode.
 *
 * - `stated`: the name gives a season *and* an episode, e.g. `S03E09` or
 *   `3x09`. This is the strongest signal a name can carry: it cannot be
 *   mistaken for another season's episode.
 * - `episode`: the name gives an episode number but no season, e.g. `Show - 09`
 *   or `Show 12`. Genuinely a match -- the backend said so -- but ambiguous
 *   about which season it belongs to.
 * - `unclear`: the name gives no episode number at all. Typically a batch or a
 *   film, which the backend accepted because no episode was requested.
 */
export function matchClarity(release: Release): MatchClarity {
  const { season, episode, absoluteEpisode } = release.parsed;

  // A season *and* an episode: the name is explicit about both. This is the
  // only case that can be trusted without cross-checking anything else.
  if (season != null && episode != null) return "stated";

  // An episode number of some kind, but not a stated season. The backend's
  // absolute-episode patterns fill `episode` and `absoluteEpisode` for the
  // `Show - 09` shape, so either is enough to call it an episode.
  if (episode != null || absoluteEpisode != null) return "episode";

  return "unclear";
}

/**
 * A short label for the clarity, or `undefined` when there is nothing to say.
 *
 * `undefined` for `episode` on purpose: that is the ordinary case and marking
 * every row would defeat the point of marking any.
 */
export function clarityLabel(clarity: MatchClarity): string | undefined {
  switch (clarity) {
    case "stated":
      return "SxxExx";
    case "unclear":
      return "No episode";
    case "episode":
      return undefined;
  }
}

/**
 * A one-line explanation of the clarity, for a `title` attribute.
 *
 * Mirrors how the health dot explains itself: the visual is a glanceable
 * signal, and the prose lives on hover so the row stays compact.
 */
export function clarityTitle(clarity: MatchClarity): string {
  switch (clarity) {
    case "stated":
      return "Names its season and episode";
    case "episode":
      return "Names the episode only, not the season";
    case "unclear":
      return "No episode number in the name";
  }
}