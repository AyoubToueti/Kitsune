// Resolution ordering and availability, for the release filter.
//
// Pure, so the fiddly part -- deciding which chips to offer and in what order --
// is testable without rendering the page.

import type { Release, Resolution } from "$lib/types";

/**
 * Every resolution the backend can report, highest first.
 *
 * This is the display and sort order for the filter chips, not a list of what
 * to offer: the chips come from what a search actually returned. Declared as a
 * total order over the `Resolution` union so a new variant is a compile error
 * here rather than a value that silently sorts last.
 *
 * `"unknown"` is last because it is the absence of a claim, not a low quality.
 * Ordering it below 360p would read as "worse than 360p", which it is not.
 */
export const RESOLUTION_ORDER = [
  "2160p",
  "1080p",
  "720p",
  "576p",
  "540p",
  "480p",
  "360p",
  "unknown",
] as const satisfies readonly Resolution[];

/** One filter chip: a resolution present in the results, and how many. */
export interface ResolutionCount {
  resolution: Resolution;
  /** How many releases report this resolution. */
  count: number;
}

/**
 * The resolutions actually present in a set of releases, highest first.
 *
 * Only resolutions that appear are returned, so a search that found nothing but
 * 1080p and 720p offers exactly two chips rather than the whole range from 4K
 * down to 144p. A resolution the search never returned would be a control that
 * can only ever empty the list.
 *
 * `"unknown"` is included when present rather than dropped: those releases are
 * real and have to remain reachable when a filter is active, and a chip is the
 * only way to select them.
 */
export function availableResolutions(releases: Release[]): ResolutionCount[] {
  const counts = new Map<Resolution, number>();

  for (const release of releases) {
    counts.set(release.resolution, (counts.get(release.resolution) ?? 0) + 1);
  }

  // Driven by RESOLUTION_ORDER rather than the map's insertion order, so the
  // chips do not reshuffle as the release list is re-ranked by the probe.
  return RESOLUTION_ORDER.filter((resolution) => counts.has(resolution)).map(
    (resolution) => ({ resolution, count: counts.get(resolution) ?? 0 }),
  );
}

/**
 * Whether a release passes the active resolution filter.
 *
 * An empty filter keeps everything, which is what makes "no filter selected"
 * and "every resolution selected" behave the same way -- and means clearing the
 * filter cannot leave the list empty by accident.
 */
export function matchesResolution(
  release: Release,
  selected: readonly Resolution[],
): boolean {
  if (selected.length === 0) return true;
  return selected.includes(release.resolution);
}