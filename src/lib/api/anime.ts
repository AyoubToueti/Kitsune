// Typed wrappers over the Tauri commands in `src-tauri/src/commands.rs`.
//
// Everything the UI needs from the backend goes through here, so there is
// one place that knows the command names and argument shapes. Components
// import these rather than calling `invoke` directly.

import { invoke } from "@tauri-apps/api/core";

import type {
  Anime,
  AnimePage,
  BrowseQuery,
  ListFilter,
  ScheduledEpisode,
} from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const COMMANDS = {
  trending: "get_trending",
  list: "get_list",
  browse: "get_browse",
  byId: "get_anime",
  genres: "get_genres",
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
  return invoke<Anime[]>(COMMANDS.trending, { limit });
}

/** Look up a single title. Resolves to `null` when the id does not exist. */
export async function getAnime(id: number): Promise<Anime | null> {
  return invoke<Anime | null>(COMMANDS.byId, { id });
}

/** A curated list, chosen by intent. Backs the home-screen shelves. */
export async function getList(filter: ListFilter, limit?: number): Promise<Anime[]> {
  return invoke<Anime[]>(COMMANDS.list, { filter, limit });
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
  return invoke<AnimePage>(COMMANDS.browse, { query, page, perPage });
}

/** The genres available for browsing. */
export async function getGenres(): Promise<string[]> {
  return invoke<string[]>(COMMANDS.genres);
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
  return invoke<ScheduledEpisode[]>(COMMANDS.schedule, { from, to, limit });
}