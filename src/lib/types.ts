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