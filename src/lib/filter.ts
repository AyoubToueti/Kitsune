// Parsing and serialising the filter query string.
//
// Pure, so the fiddly part -- turning untrusted URL parameters into a
// `BrowseQuery` without letting a hand-edited value through -- is testable
// without rendering a form.

import type {
  BrowseQuery,
  FormatFilter,
  MediaTag,
  SeasonFilter,
  SortOption,
  StatusFilter,
} from "$lib/types";

/**
 * The values each filter accepts.
 *
 * Declared here and used by BOTH the form options and the parser, so a value
 * the form offers can never be rejected on the way back in. Two separate lists
 * would drift, and the failure would look like a filter that silently does
 * nothing.
 */
export const FORMAT_VALUES = [
  "tv",
  "movie",
  "ova",
  "ona",
  "special",
  "music",
] as const satisfies readonly FormatFilter[];

export const STATUS_VALUES = [
  "releasing",
  "finished",
  "notYetReleased",
] as const satisfies readonly StatusFilter[];

export const SEASON_VALUES = [
  "winter",
  "spring",
  "summer",
  "fall",
] as const satisfies readonly SeasonFilter[];

export const SORT_VALUES = [
  "popularity",
  "score",
  "newest",
  "titleAz",
  "searchMatch",
] as const satisfies readonly SortOption[];

/**
 * Sorts offered on the filter page.
 *
 * `searchMatch` is excluded: it orders by textual relevance, which means
 * nothing without a search term, and the filter page may be used without one.
 */
export const FILTER_SORT_VALUES = [
  "popularity",
  "score",
  "newest",
  "titleAz",
] as const satisfies readonly SortOption[];

/** Human labels for the select options. */
export const FORMAT_LABELS: Record<FormatFilter, string> = {
  tv: "TV",
  movie: "Movie",
  ova: "OVA",
  ona: "ONA",
  special: "Special",
  music: "Music",
};

export const STATUS_LABELS: Record<StatusFilter, string> = {
  releasing: "Currently airing",
  finished: "Finished airing",
  notYetReleased: "Not yet aired",
};

export const SEASON_LABELS: Record<SeasonFilter, string> = {
  winter: "Winter",
  spring: "Spring",
  summer: "Summer",
  fall: "Fall",
};

export const SORT_LABELS: Record<SortOption, string> = {
  popularity: "Most popular",
  score: "Highest rated",
  newest: "Newest",
  titleAz: "Name A–Z",
  searchMatch: "Best match",
};

/** Score floors offered as a minimum. */
export const SCORE_OPTIONS = [50, 60, 70, 80, 90] as const;

/** Oldest year offered. AniList has little before this worth filtering to. */
const EARLIEST_YEAR = 1960;

/** A value from `values`, or undefined when it is absent or not recognised. */
function oneOf<T extends string>(
  raw: string | null,
  values: readonly T[],
): T | undefined {
  if (raw === null) return undefined;
  return values.includes(raw as T) ? (raw as T) : undefined;
}

/** A whole number within range, or undefined. */
function intInRange(raw: string | null, min: number, max: number): number | undefined {
  if (raw === null || raw.trim() === "") return undefined;
  const value = Number(raw);
  if (!Number.isFinite(value)) return undefined;
  const whole = Math.floor(value);
  return whole >= min && whole <= max ? whole : undefined;
}

/**
 * Read a `BrowseQuery` out of URL parameters.
 *
 * Anything unrecognised is dropped rather than forwarded. The parameters come
 * from a URL a user can edit, and the backend rejects an unknown enum outright
 * -- so passing one through would surface as a confusing request failure
 * rather than as the bad link it is.
 *
 * `search` is trimmed, and blank counts as absent.
 */
export function parseBrowseQuery(params: URLSearchParams): BrowseQuery {
  const search = (params.get("search") ?? "").trim();

  const query: BrowseQuery = {
    sort: oneOf(params.get("sort"), SORT_VALUES) ?? "popularity",
  };

  if (search !== "") query.search = search;

  const genres = params.getAll("genre").filter((g) => g.trim() !== "");
  if (genres.length > 0) query.genres = genres;

  // Tags are not validated against a known list here: the catalogue is fetched
  // at runtime, and this function is pure. The backend treats an unrecognised
  // tag as matching nothing rather than erroring, so a hand-edited value
  // degrades to an empty result instead of a failure.
  const tags = params.getAll("tag").filter((t) => t.trim() !== "");
  if (tags.length > 0) query.tags = tags;

  // Exclusion travels as its own parameter rather than as a sign on the tag.
  // A tag name may legitimately be repeated, and overloading `tag=Isekai` to
  // mean both directions would make the URL ambiguous.
  const excludedTags = params
    .getAll("exclude_tag")
    .filter((t) => t.trim() !== "");
  if (excludedTags.length > 0) query.excludedTags = excludedTags;

  const format = oneOf(params.get("format"), FORMAT_VALUES);
  if (format) query.format = format;

  const status = oneOf(params.get("status"), STATUS_VALUES);
  if (status) query.status = status;

  const season = oneOf(params.get("season"), SEASON_VALUES);
  if (season) query.season = season;

  // One year ahead is allowed so upcoming seasons can be browsed.
  const year = intInRange(
    params.get("year"),
    EARLIEST_YEAR,
    new Date().getFullYear() + 1,
  );
  if (year !== undefined) query.seasonYear = year;

  const score = intInRange(params.get("score"), 0, 100);
  if (score !== undefined) query.minScore = score;

  return query;
}

/**
 * A page link that preserves the current filters.
 *
 * Page 1 is written without a `page` parameter, so a filtered view has one
 * canonical URL rather than two spellings of the same results.
 */
export function filterHref(
  params: URLSearchParams,
  page: number,
  base = "/filter",
): string {
  const next = new URLSearchParams(params);

  if (page <= 1) {
    next.delete("page");
  } else {
    next.set("page", String(page));
  }

  const query = next.toString();
  return query === "" ? base : `${base}?${query}`;
}

/**
 * The provider's tag names, grouped under their category for display.
 *
 * Order is preserved from the input rather than sorted: a provider that
 * already returns categories in a sensible order should keep it, and
 * re-sorting here would silently undo that.
 */
export function groupTagsByCategory(
  tags: readonly MediaTag[],
): { category: string; tags: MediaTag[] }[] {
  const groups: { category: string; tags: MediaTag[] }[] = [];
  const index = new Map<string, number>();

  for (const tag of tags) {
    let at = index.get(tag.category);
    if (at === undefined) {
      at = groups.length;
      index.set(tag.category, at);
      groups.push({ category: tag.category, tags: [] });
    }
    // The whole tag travels, not just its name: the chip needs the
    // description for its tooltip, and re-looking it up by name would be a
    // second source of truth.
    groups[at].tags.push(tag);
  }

  return groups;
}

/**
 * Turn a provider's category key into a heading.
 *
 * The provider packs a hierarchy into one string with a hyphen
 * ("Cast-Main Cast", "Theme-Game-Sport"). Only the FIRST hyphen is a
 * separator -- later ones are part of the name, as in "Game-Card & Board" --
 * so this must not replace every occurrence.
 */
export function categoryLabel(category: string): string {
  const at = category.indexOf("-");
  if (at === -1) return category;
  return `${category.slice(0, at)} / ${category.slice(at + 1)}`;
}

/** Years offered in the year select, newest first. */
export function yearOptions(): number[] {
  const newest = new Date().getFullYear() + 1;
  return Array.from(
    { length: newest - EARLIEST_YEAR + 1 },
    (_, i) => newest - i,
  );
}