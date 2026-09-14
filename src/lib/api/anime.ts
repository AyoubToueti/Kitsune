// Typed wrappers over the Tauri commands in `src-tauri/src/commands.rs`.
//
// Everything the UI needs from the backend goes through here, so there is
// one place that knows the command names and argument shapes. Components
// import these rather than calling `invoke` directly.

import { invoke } from "@tauri-apps/api/core";

import { API_TTL, cached } from "./cache";
import type {
  Anime,
  AnimePage,
  BrowseQuery,
  ListFilter,
  MediaTag,
  ScheduledEpisode,
} from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const COMMANDS = {
  trending: "get_trending",
  list: "get_list",
  browse: "get_browse",
  byId: "get_anime",
  genres: "get_genres",
  tags: "get_tags",
  schedule: "get_schedule",
} as const;

/**
 * Convert whatever the backend rejected with into a readable message.
 *
 * Tauri surfaces a command's `Err(String)` as the rejection value, but a
 * transport-level problem can reject with an `Error`, and `invoke` is
 * untyped enough that anything is possible. Rendering `[object Object]` at
 * the user is worse than a generic fallback, so unknown shapes get one.
 */
export function errorMessage(error: unknown): string {
  if (typeof error === "string" && error.trim() !== "") {
    return error;
  }
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return "Something went wrong while contacting AniList.";
}

/** Trending titles, for the home screen. */
export async function getTrending(limit?: number): Promise<Anime[]> {
  // Keyed by the limit: a rail asking for 20 and a hero asking for 5 are
  // different results and must not share an entry.
  return cached(`trending:${limit ?? "default"}`, API_TTL.list, () =>
    invoke<Anime[]>(COMMANDS.trending, { limit }),
  );
}

/** Look up a single title. Resolves to `null` when the id does not exist. */
export async function getAnime(id: number): Promise<Anime | null> {
  return cached(`anime:${id}`, API_TTL.detail, () =>
    invoke<Anime | null>(COMMANDS.byId, { id }),
  );
}

/** A curated list, chosen by intent. Backs the home-screen shelves. */
export async function getList(filter: ListFilter, limit?: number): Promise<Anime[]> {
  return cached(`list:${filter}:${limit ?? "default"}`, API_TTL.list, () =>
    invoke<Anime[]>(COMMANDS.list, { filter, limit }),
  );
}

/**
 * Browse with filters and paging.
 *
 * One parameterised call rather than one per filter combination, so adding a
 * filter never changes this surface.
 *
 * `page` is 1-based. `perPage` is a request, not a guarantee -- the backend
 * clamps it, and the provider caps it at 50.
 */
export async function browseAnime(
  query: BrowseQuery,
  page?: number,
  perPage?: number,
): Promise<AnimePage> {
  // The whole query is the key: two filter combinations are different result
  // sets and must not collide. The LRU cap in the cache bounds how many of
  // these keys can accumulate.
  return cached(
    `browse:${JSON.stringify({ query, page, perPage })}`,
    API_TTL.list,
    () => invoke<AnimePage>(COMMANDS.browse, { query, page, perPage }),
  );
}

/** The genres available for browsing. */
export async function getGenres(): Promise<string[]> {
  // Effectively static, so the longest TTL: every route that mounts a filter
  // panel shares one result.
  return cached("genres", API_TTL.catalogue, () =>
    invoke<string[]>(COMMANDS.genres),
  );
}

/** The tags available for filtering, each with its grouping category. */
export async function getTags(): Promise<MediaTag[]> {
  // The tag catalogue is the largest payload in the app and barely changes, so
  // it is the biggest win from caching.
  return cached("tags", API_TTL.catalogue, () =>
    invoke<MediaTag[]>(COMMANDS.tags),
  );
}

/**
 * Broadcasts falling within a window, soonest first.
 *
 * `from` and `to` are unix timestamps in seconds, so the caller owns the span
 * and there is no hidden "next N hours" assumption.
 */
export async function getSchedule(
  from: number,
  to: number,
  limit?: number,
): Promise<ScheduledEpisode[]> {
  // The window is part of the key: each day is a distinct slice. The shortest
  // TTL, because airing times move and a stale schedule is worse than a fetch.
  return cached(
    `schedule:${from}:${to}:${limit ?? "default"}`,
    API_TTL.schedule,
    () => invoke<ScheduledEpisode[]>(COMMANDS.schedule, { from, to, limit }),
  );
}