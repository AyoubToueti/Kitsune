// Season arithmetic and display.
//
// Split out from the components because "which season is next" is date maths
// with off-by-one hazards at year boundaries, and that is worth testing without
// rendering anything.

import { SEASON_LABELS, SEASON_VALUES } from "./filter";
import type { BrowseQuery, SeasonFilter } from "./types";

/** A season and the calendar year it belongs to. */
export interface Season {
  season: SeasonFilter;
  year: number;
}

/**
 * Which quarter a month falls in.
 *
 * Winter is Jan-Mar, so the index is `(month - 1) / 3` rather than a lookup on
 * month names -- the latter is easy to get subtly wrong at the boundaries.
 */
function quarterOf(month: number): number {
  return Math.floor((month - 1) / 3);
}

/**
 * The season after the one `now` falls in.
 *
 * Rolls the year when the current season is the last one, which is the case
 * that is easy to miss: December's next season is Winter of the FOLLOWING year,
 * not Winter of this one.
 */
export function nextSeason(now: Date = new Date()): Season {
  const current = quarterOf(now.getMonth() + 1);
  const rollsOver = current === SEASON_VALUES.length - 1;

  return {
    season: SEASON_VALUES[(current + 1) % SEASON_VALUES.length],
    year: rollsOver ? now.getFullYear() + 1 : now.getFullYear(),
  };
}

/**
 * A provider season plus year as one label, e.g. "Fall 2023".
 *
 * The provider spells the season in its own casing ("FALL"), so it is matched
 * case-insensitively against our own values rather than compared literally.
 * Either half may be missing, and a partial label beats none.
 */
export function formatSeason(
  season: string | undefined,
  year: number | undefined,
): string | undefined {
  const label =
    season === undefined
      ? undefined
      : SEASON_LABELS[
          (SEASON_VALUES.find(
            (value) => value.toLowerCase() === season.toLowerCase(),
          ) ?? "") as SeasonFilter
        ];

  if (label !== undefined && year !== undefined) return `${label} ${year}`;
  if (label !== undefined) return label;
  if (year !== undefined) return String(year);
  return undefined;
}

/**
 * The `BrowseQuery` that selects one season's unreleased titles.
 *
 * Not-yet-released rather than everything dated to the season: a season
 * includes titles that already aired, and "upcoming" should mean upcoming.
 */
export function seasonQuery(season: Season): BrowseQuery {
  return {
    season: season.season,
    seasonYear: season.year,
    status: "notYetReleased",
    sort: "popularity",
  };
}

/**
 * A link to the same season's unreleased titles, for a "View all".
 *
 * Built from the same `Season` as the query, so the link and the request
 * cannot disagree about which season is being shown.
 */
export function seasonHref(season: Season, base = "/filter"): string {
  const params = new URLSearchParams({
    season: season.season,
    year: String(season.year),
    status: "notYetReleased",
  });

  return `${base}?${params.toString()}`;
}