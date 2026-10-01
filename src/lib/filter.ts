// Parsing and serialising the filter query string.
//
// Pure, so the fiddly part -- turning untrusted URL parameters into a
// `BrowseQuery` without letting a hand-edited value through -- is testable
// without rendering a form.

import type {
  BrowseQuery,
  FormatFilter,
  ListFilter,
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

/**
 * Every sort the parser accepts.
 *
 * Includes `searchMatch` so a hand-written `?sort=searchMatch` round-trips,
 * even though no control offers it.
 */
export const SORT_VALUES = [
  "titleAz",
  "popularity",
  "score",
  "trending",
  "favorites",
  "dateAdded",
  "newest",
  "searchMatch",
] as const satisfies readonly SortOption[];

/**
 * Sorts offered in the sort menu, in the order the reference lists them.
 *
 * `searchMatch` is excluded: it orders by textual relevance, which means
 * nothing without a search term, so it would be a control that does nothing
 * on a page that has no query.
 */
export const SORT_MENU_VALUES = [
  "titleAz",
  "popularity",
  "score",
  "trending",
  "favorites",
  "dateAdded",
  "newest",
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

/**
 * The browse query each curated list stands for.
 *
 * A home section is a sort plus, sometimes, a status: "Top airing" is the
 * popularity order restricted to what is still releasing. The filter page takes
 * both as URL parameters, so the mapping is what lets a section's "View more"
 * land on the same ordering rather than a differently-ordered list.
 *
 * Every `ListFilter` must appear, so adding a variant is a compile error here
 * rather than a section that silently links to the wrong order. `status` is
 * absent for the lists that do not restrict one.
 */
export const LIST_QUERIES: Record<
  ListFilter,
  { sort: SortOption; status?: StatusFilter }
> = {
  trending: { sort: "trending" },
  topAiring: { sort: "popularity", status: "releasing" },
  mostPopular: { sort: "popularity" },
  topRated: { sort: "score" },
  latestCompleted: { sort: "newest", status: "finished" },
  upcoming: { sort: "popularity", status: "notYetReleased" },
};

/**
 * A link to the filter page showing one curated list.
 *
 * The home sections carry no other filters, so the URL is built fresh rather
 * than preserving parameters: there is nothing to preserve. `status` is only
 * written when the list restricts one, so "Most popular" does not gain an
 * empty-looking `status=` it never meant.
 */
export function listHref(filter: ListFilter): string {
  const { sort, status } = LIST_QUERIES[filter];

  const params = new URLSearchParams({ sort });
  if (status !== undefined) params.set("status", status);

  return `/filter?${params}`;
}

/**
 * A link to the filter page showing one genre.
 *
 * The genre is a filter, not a route of its own: /filter carries the same list
 * plus the filter bar and sort menu, so browsing a genre no longer needs a page
 * that duplicates it without those controls.
 *
 * The sort is written explicitly rather than left to the parser's default, so
 * the ordering cannot change under the link if that default ever moves. Genre
 * names can contain spaces ("Slice of Life"), which `URLSearchParams` encodes
 * as `+`; `parseBrowseQuery` reads that back as a space.
 */
export function genreHref(genre: string): string {
  const params = new URLSearchParams({ sort: "popularity" });
  params.set("genre", genre);

  return `/filter?${params}`;
}

/**
 * Labels follow the provider's own wording, so the menu reads the same as the
 * site the ordering comes from.
 */
export const SORT_LABELS: Record<SortOption, string> = {
  titleAz: "Title",
  popularity: "Popularity",
  score: "Average Score",
  trending: "Trending",
  favorites: "Favorites",
  dateAdded: "Date Added",
  // AniList calls this START_DATE_DESC, which its own UI labels "Release Date"
  // rather than "newest".
  newest: "Release Date",
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

  // Presence of `order=reverse` flips the sort. The value is fixed rather than
  // free text so the parameter reads as a flag, and a hand-edited `order=asc`
  // is simply ignored (natural direction) rather than half-understood.
  if (params.get("order") === "reverse") query.reversed = true;

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
 * A link to the same view with a different sort.
 *
 * The page number is dropped: page 5 of a re-ordered list is not a meaningful
 * place to land, and keeping it could show a page past the new end.
 */
export function sortHref(
  params: URLSearchParams,
  sort: SortOption,
  base = "/filter",
): string {
  const next = new URLSearchParams(params);
  next.set("sort", sort);
  next.delete("page");
  // A new sort starts at its natural direction. Carrying `order=reverse` over
  // from a different field would reverse something the reader never reversed.
  next.delete("order");

  const query = next.toString();
  return query === "" ? base : `${base}?${query}`;
}

/**
 * A link to the same view with the sort direction flipped.
 *
 * Toggling, not setting: reversing twice returns to the natural direction, so
 * the parameter is dropped rather than written as an explicit "forward". The
 * page number is dropped for the same reason as in {@link sortHref} -- page 5
 * of a re-ordered list is not a meaningful place to land.
 */
export function reverseHref(params: URLSearchParams, base = "/filter"): string {
  const next = new URLSearchParams(params);

  if (next.get("order") === "reverse") {
    next.delete("order");
  } else {
    next.set("order", "reverse");
  }
  next.delete("page");

  const query = next.toString();
  return query === "" ? base : `${base}?${query}`;
}

/** One applied filter, as the active-filters bar presents it. */
export interface ActiveFilter {
  /** What the pill reads. */
  label: string;
  /** The URL parameter carrying it, for building a removal link. */
  param: string;
  /** The parameter's value, needed because some repeat (genre, tag). */
  value: string;
  /** Rejections render differently from requirements. */
  excluded: boolean;
}

/**
 * Every filter currently applied, in the order the panel presents them.
 *
 * Derived from the parsed query rather than from the raw parameters, so an
 * unrecognised value that `parseBrowseQuery` dropped does not get a pill
 * claiming it is active.
 */
export function activeFilters(query: BrowseQuery): ActiveFilter[] {
  const filters: ActiveFilter[] = [];

  if (query.search) {
    filters.push({
      label: `“${query.search}”`,
      param: "search",
      value: query.search,
      excluded: false,
    });
  }

  for (const genre of query.genres ?? []) {
    filters.push({ label: genre, param: "genre", value: genre, excluded: false });
  }

  for (const tag of query.tags ?? []) {
    filters.push({ label: tag, param: "tag", value: tag, excluded: false });
  }

  for (const tag of query.excludedTags ?? []) {
    // Prefixed because inclusion and exclusion are different claims about the
    // same tag, and a bare name would not say which one was meant.
    filters.push({
      label: `Not ${tag}`,
      param: "exclude_tag",
      value: tag,
      excluded: true,
    });
  }

  if (query.format) {
    filters.push({
      label: FORMAT_LABELS[query.format],
      param: "format",
      value: query.format,
      excluded: false,
    });
  }

  if (query.status) {
    filters.push({
      label: STATUS_LABELS[query.status],
      param: "status",
      value: query.status,
      excluded: false,
    });
  }

  if (query.season) {
    filters.push({
      label: SEASON_LABELS[query.season],
      param: "season",
      value: query.season,
      excluded: false,
    });
  }

  if (query.seasonYear !== undefined) {
    filters.push({
      label: String(query.seasonYear),
      param: "year",
      value: String(query.seasonYear),
      excluded: false,
    });
  }

  if (query.minScore !== undefined) {
    filters.push({
      label: `Score ${query.minScore}+`,
      param: "score",
      value: String(query.minScore),
      excluded: false,
    });
  }

  return filters;
}

/**
 * A link to the same view with one filter removed.
 *
 * Rebuilds the query string rather than calling `delete`, because the repeated
 * parameters (genre, tag) must keep their other values -- `delete` would drop
 * every genre when the user only meant to remove one.
 *
 * `page` is dropped as well: page 5 of a different filter set is not a
 * meaningful place to land.
 */
export function removeFilterHref(
  params: URLSearchParams,
  param: string,
  value: string,
  base = "/filter",
): string {
  const next = new URLSearchParams();

  for (const [key, existing] of params) {
    if (key === param && existing === value) continue;
    next.append(key, existing);
  }

  next.delete("page");

  const query = next.toString();
  return query === "" ? base : `${base}?${query}`;
}

/**
 * A link to the same view with every filter removed.
 *
 * Built from the applied filters rather than from a fixed list, so a parameter
 * this module does not know about -- `/search`'s `q`, for instance -- survives.
 * Clearing the filters should not also clear what the user searched for.
 */
export function clearFiltersHref(
  params: URLSearchParams,
  query: BrowseQuery,
  base = "/filter",
): string {
  const drop = new Set(activeFilters(query).map((filter) => filter.param));
  drop.add("page");

  const next = new URLSearchParams();

  for (const [key, value] of params) {
    if (drop.has(key)) continue;
    next.append(key, value);
  }

  const queryString = next.toString();
  return queryString === "" ? base : `${base}?${queryString}`;
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