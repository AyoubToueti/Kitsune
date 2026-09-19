// Free-text filtering for the release list.
//
// Distinct from `filter.ts`, which parses the browse page's URL parameters: this
// is a substring match over a release name, applied on top of the resolution
// filter rather than instead of it.
//
// Pure, so the matching rule is testable without rendering the page.

import type { Release } from "$lib/types";

/**
 * Whether a release matches a typed query.
 *
 * A plain case-insensitive substring match, so a partial word is enough: typing
 * `1080` finds every 1080p release without needing the full `1080p`. The query is
 * trimmed first, so a stray space does not empty the list.
 *
 * An empty query keeps everything, which is what makes "nothing typed" and
 * "everything matched" behave the same way -- and means clearing the box cannot
 * leave the list empty by accident.
 */
export function matchesQuery(release: Release, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (needle === "") return true;

  return release.title.toLowerCase().includes(needle);
}

/**
 * Keep only the releases matching a query, preserving their order.
 *
 * Order is preserved rather than re-sorted: the caller has already ranked the
 * list by quality and swarm health, and a text filter is a narrowing step, not a
 * re-ranking one.
 */
export function filterByQuery(
  releases: Release[],
  query: string,
): Release[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return releases;

  return releases.filter((release) => matchesQuery(release, query));
}