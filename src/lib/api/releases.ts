// Typed wrapper over the indexer command in `src-tauri/src/indexer/commands.rs`.
//
// Kept apart from `./player`: this module asks the wider internet a question
// ("what releases exist for this work?") while `./player` moves bytes on the
// local machine. The two share no cache and no session.

import { invoke } from "@tauri-apps/api/core";

import type { Release } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const RELEASE_COMMANDS = {
  search: "search_releases",
  download: "download_torrent",
} as const;

/**
 * Search the configured indexers for a work, optionally narrowed to an episode.
 *
 * `title` and `episode` are the exact key names the Rust command expects:
 * Tauri maps a command's snake_case parameters to camelCase keys, and the
 * command's first argument is literally `title`, so sending `work` would fail
 * as a missing argument. The backend ranks and de-duplicates the hits before
 * returning them, so the caller renders the list as-is.
 *
 * Deliberately NOT cached: swarm health moves minute to minute, and a cached
 * seeder count would be a lie.
 */
export async function searchReleases(
  title: string,
  episode?: number,
): Promise<Release[]> {
  return invoke<Release[]>(RELEASE_COMMANDS.search, { title, episode });
}

export async function downloadTorrent(url: string, path: string): Promise<string> {
  return invoke<string>(RELEASE_COMMANDS.download, { url, path });
}