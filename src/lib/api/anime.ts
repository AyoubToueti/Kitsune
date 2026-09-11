// Typed wrappers over the Tauri commands in `src-tauri/src/commands.rs`.
//
// Everything the UI needs from the backend goes through here, so there is
// one place that knows the command names and argument shapes. Components
// import these rather than calling `invoke` directly.

import { invoke } from "@tauri-apps/api/core";

import type { Anime } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const COMMANDS = {
  trending: "get_trending",
  search: "search_anime",
  byId: "get_anime",
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