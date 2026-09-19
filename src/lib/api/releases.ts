// Typed wrapper over the indexer command in `src-tauri/src/indexer/commands.rs`.
//
// Kept apart from `./player`: this module asks the wider internet a question
// ("what releases exist for this work?") while `./player` moves bytes on the
// local machine. The two share no cache and no session.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { ProbeOutcome, Release } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const RELEASE_COMMANDS = {
  search: "search_releases",
  download: "download_torrent",
  probe: "probe_releases",
} as const;

/**
 * Event the backend emits once per probed release.
 *
 * Centralised for the same reason as the command names: the emitter and the
 * listener are in different languages, so nothing but this constant ties them
 * together, and a typo in either would silently deliver no events.
 */
export const PROBE_RESULT_EVENT = "probe-result";

/**
 * Search the configured indexers for a work, optionally narrowed to an episode.
 *
   * `titles` and `episode` are the exact key names the Rust command expects:
   * Tauri maps a command's snake_case parameters to camelCase keys, and the
   * command's first argument is literally `titles`. Send every title form the
   * work has (see `titleForms`), best first: an uploader may have used either
   * the English or the romaji title, and only one of them will match.
   *
       * `episode` counts from the start of the cour; `absoluteEpisode` counts from
       * the start of the whole work. Both are sent when they differ, because anime
       * numbering is inconsistent: a later cour's releases may be named either
       * `Show - 01` or `Show - 13`, and the matcher accepts whichever the release
       * used. Pass the same value twice, or omit the second, when they agree.
       *
       * The backend expands the titles into several query spellings, de-duplicates
       * the merged hits and ranks them, so the caller renders the list as-is.
       *
       * Deliberately NOT cached: swarm health moves minute to minute, and a cached
       * seeder count would be a lie.
       */
      export async function searchReleases(
        titles: string[],
        episode?: number,
        absoluteEpisode?: number,
      ): Promise<Release[]> {
        return invoke<Release[]>(RELEASE_COMMANDS.search, {
          titles,
          episode,
          absoluteEpisode,
        });
}

/**
 * Download a `.torrent` file from an indexer to a path the user chose.
 *
 * The fetch happens in the backend rather than the webview: the frontend cannot
 * reach nyaa.si directly (CORS), and a native request can write the bytes
 * straight to disk instead of holding them in a JS string.
 */
export async function downloadTorrent(url: string, path: string): Promise<string> {
  return invoke<string>(RELEASE_COMMANDS.download, { url, path });
}

/**
 * Ask the backend to probe a set of releases for swarm health.
 *
 * Returns once every probe has finished, but the useful signal arrives earlier,
 * as [`PROBE_RESULT_EVENT`] events: one per release, in completion order. The
 * returned list is sorted by index, so a caller that only wants the final
 * verdict can ignore the events entirely.
 *
 * `releases` must be the exact list that was searched for, in the same order:
 * each event identifies its release by position. Nothing is cached, because
 * swarm health changes minute to minute.
 */
export async function probeReleases(
  releases: Release[],
): Promise<ProbeOutcome[]> {
  return invoke<ProbeOutcome[]>(RELEASE_COMMANDS.probe, { releases });
}

/**
 * Subscribe to probe progress, calling back as each release is judged.
 *
 * `listen` resolves with an unlisten function rather than returning one, so
 * the caller cannot forget to await it and leak the listener. The callback is
 * wrapped so a malformed payload is dropped rather than thrown into Tauri's
 * event loop.
 */
export async function onProbeResult(
  handler: (outcome: ProbeOutcome) => void,
): Promise<UnlistenFn> {
  return listen<ProbeOutcome>(PROBE_RESULT_EVENT, (event) => {
    const outcome = event.payload;
    // A payload without an index cannot be matched to a release, so it is
    // ignored rather than applied to release 0 by accident.
    if (outcome == null || typeof outcome.index !== "number") return;
    handler(outcome);
  });
}