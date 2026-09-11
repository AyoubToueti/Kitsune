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