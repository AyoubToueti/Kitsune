// Typed wrappers over the Tauri commands in `src-tauri/src/commands.rs`.
//
// Everything the UI needs from the backend goes through here, so there is
// one place that knows the command names and argument shapes. Components
// import these rather than calling `invoke` directly.

import { invoke } from "@tauri-apps/api/core";

import type { Anime, ListFilter, ScheduledEpisode } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const COMMANDS = {
  trending: "get_trending",
  list: "get_list",
  search: "search_anime",
  byId: "get_anime",
  byGenre: "get_by_genre",
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

/** Search by free text. A blank query resolves to an empty list. */
export async function searchAnime(query: string, limit?: number): Promise<Anime[]> {
  return invoke<Anime[]>(COMMANDS.search, { query, limit });
}

/** Look up a single title. Resolves to `null` when the id does not exist. */
export async function getAnime(id: number): Promise<Anime | null> {
  return invoke<Anime | null>(COMMANDS.byId, { id });
}

/** A curated list, chosen by intent. Backs the home-screen shelves. */
export async function getList(filter: ListFilter, limit?: number): Promise<Anime[]> {
  return invoke<Anime[]>(COMMANDS.list, { filter, limit });
}

/** Titles in a genre, most popular first. */
export async function getByGenre(genre: string, limit?: number): Promise<Anime[]> {
  return invoke<Anime[]>(COMMANDS.byGenre, { genre, limit });
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