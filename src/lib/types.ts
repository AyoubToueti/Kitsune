// Mirrors of the Rust types in `src-tauri/src/types.rs`.
//
// The Rust side serialises to camelCase, so these match the wire format
// exactly rather than the snake_case field names used internally.

/** Which upstream a given id belongs to. */
export type ProviderId = "anilist" | "tmdb" | "nyaa" | "yts";

/** A work's title in the several forms providers offer. */
export interface Title {
  romaji?: string;
  english?: string;
  native?: string;
  userPreferred?: string;
}

/** A link to an official streaming source. `url` is always present. */
export interface StreamingEpisode {
  title?: string;
  url: string;
  site?: string;
  thumbnail?: string;
}

/**
 * Which curated list to fetch.
 *
 * Mirrors the Rust `ListFilter`. The values are the wire format, so they are
 * passed straight to the backend rather than translated here.
 */
export type ListFilter =
  | "trending"
  | "topAiring"
  | "mostPopular"
  | "topRated"
  | "latestCompleted"
  | "upcoming";

/**
 * A descriptor the provider associates with a work.
 *
 * Distinct from a genre: `category` is what a filter UI groups by, so the two
 * travel together rather than as a bare name.
 */
export interface MediaTag {
  name: string;
  category: string;
  /**
   * The provider's own prose for what the tag means.
   *
   * Optional: the provider may have no description for a tag, and the chip
   * simply renders without a tooltip rather than with an empty one.
   */
  description?: string;
}

/** A work as presented in the UI. */
export interface Anime {
  id: number;
  provider: ProviderId;
  title: Title;
  coverImage?: string;
  bannerImage?: string;
  description?: string;
  episodeCount?: number;
  durationMinutes?: number;
  format?: string;
  genres: string[];
  averageScore?: number;
  popularity?: number;
  status?: string;
  seasonYear?: number;
  streamingEpisodes: StreamingEpisode[];
}

/**
 * Where a page of results sits in the whole set.
 *
 * Mirrors the Rust `PageInfo`. `lastPage` and `hasNextPage` are what let the
 * UI render pagination at all.
 */
export interface PageInfo {
  /**
   * Total matches. AniList caps this at 5000, so for a very broad query it is
   * a floor rather than an exact count.
   */
  total: number;
  currentPage: number;
  lastPage: number;
  hasNextPage: boolean;
}

/** One page of results, with the metadata needed to ask for another. */
export interface AnimePage {
  items: Anime[];
  pageInfo: PageInfo;
}

/** How to order results. Mirrors the Rust `SortOption`. */
export type SortOption =
  | "popularity"
  | "score"
  | "newest"
  | "titleAz"
  /**
   * Closest textual match. Only meaningful alongside a search term; without
   * one the provider falls back to its default order.
   */
  | "searchMatch";

/** Which release status to keep. */
export type StatusFilter = "releasing" | "finished" | "notYetReleased";

/** Which release format to keep. */
export type FormatFilter = "tv" | "movie" | "ova" | "ona" | "special" | "music";

/** Which release season to keep. */
export type SeasonFilter = "winter" | "spring" | "summer" | "fall";

/**
 * A browse request.
 *
 * Every filter is optional, so the UI sends only what the user set. `sort` is
 * required because there is always an order, even when nothing is filtered.
 */
export interface BrowseQuery {
  search?: string;
  genres?: string[];
  /**
   * Tags to require, by name. Combined with AND, like `genres`.
   *
   * These are the provider's tag namespace, not its genres. Only names that
   * came from `getTags()` may be sent: an unknown tag matches nothing rather
   * than erroring, so a typo would look like an empty result set.
   */
  tags?: string[];
  /**
   * Tags to reject, by name. Combined with AND, like `tags`.
   *
   * The provider's rank floor governs this direction too, so excluding a tag
   * only drops works carrying it at or above that rank. Only names that came
   * from `getTags()` may be sent, for the same reason as `tags`.
   */
  excludedTags?: string[];
  format?: FormatFilter;
  status?: StatusFilter;
  season?: SeasonFilter;
  seasonYear?: number;
  /** Minimum average score on the provider's own scale (AniList: 0-100). */
  minScore?: number;
  sort: SortOption;
}

/**
 * One upcoming broadcast, as listed on a schedule.
 *
 * Distinct from an episode of a release: this is the provider announcing that
 * a work airs at a given time, not something this app can play.
 */
export interface ScheduledEpisode {
  /** The work being aired, carried whole so a card needs no second lookup. */
  anime: Anime;
  /**
   * When it airs, as a unix timestamp in seconds.
   *
   * An absolute instant, not a formatted string: the backend has no idea what
   * timezone the viewer is in, so local formatting happens in the UI.
   */
  airingAt: number;
  /** Episode number being aired, when the provider states one. */
  episode?: number;
}

/**
 * Best available title, following the same preference order as the Rust
 * `Title::display`: user's own choice, then English, then romaji, then the
 * native form.
 *
 * Returns `undefined` only when every form is missing or blank, so callers
 * can decide how to render a genuinely untitled entry.
 */
export function displayTitle(title: Title): string | undefined {
  const candidates = [title.userPreferred, title.english, title.romaji, title.native];
  return candidates.find((c) => c != null && c.trim() !== "");
}