// Typed wrappers over the player commands in `src-tauri/src/player/commands.rs`.
//
// Separate from `./anime` because the two answer different questions: that
// module fetches metadata from AniList, this one moves bytes off the local
// machine. Mixing them would put torrent state behind the metadata cache,
// which is the last place it belongs.

import { invoke } from "@tauri-apps/api/core";

import type { TorrentHandle } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const PLAYER_COMMANDS = {
  addTorrent: "add_torrent",
  addMagnet: "add_magnet",
  removeTorrent: "remove_torrent",
  streamUrl: "get_stream_url",
  openInPlayer: "open_in_player",
  getPlayer: "get_player",
  setPlayer: "set_player",
  suggestedPlayers: "suggested_players",
} as const;

/**
 * Add a `.torrent` file and resolve the files inside it.
 *
 * Deliberately NOT cached: adding the same torrent twice is a distinct
 * action, and a stale handle would point at a session that no longer exists.
 */
export async function addTorrent(path: string): Promise<TorrentHandle> {
  return invoke<TorrentHandle>(PLAYER_COMMANDS.addTorrent, { path });
}

/**
 * Add a magnet URI and resolve the files inside it.
 *
 * The counterpart of [`addTorrent`] for a release the app found itself: a
 * search result carries a magnet, not a `.torrent` path. Like `addTorrent` it
 * is not cached -- a handle names a live session, and a stale one would point
 * at a torrent that is no longer running.
 */
export async function addMagnet(magnetUri: string): Promise<TorrentHandle> {
  return invoke<TorrentHandle>(PLAYER_COMMANDS.addMagnet, { magnetUri });
}

/**
 * Drop a torrent from the session, deleting the pieces it cached.
 *
 * The counterpart of [`addMagnet`] and [`addTorrent`]: the watch page calls
 * this when it is left, so a closed episode stops downloading. Not cached, and
 * deliberately fire-and-forget at the call site -- the component that would
 * show an error is already gone by then.
 */
export async function removeTorrent(torrentId: number): Promise<void> {
  return invoke<void>(PLAYER_COMMANDS.removeTorrent, { torrentId });
}

/** The loopback URL that streams one file of one torrent. */
export async function getStreamUrl(
  torrentId: number,
  fileIdx: number,
): Promise<string> {
  return invoke<string>(PLAYER_COMMANDS.streamUrl, { torrentId, fileIdx });
}

/**
 * Open a URL in an external player.
 *
 * Resolves to the name of the player that was launched, which is useful when
 * `player` was omitted and the stored preference is unknown to the caller.
 */
export async function openInPlayer(url: string, player?: string): Promise<string> {
  return invoke<string>(PLAYER_COMMANDS.openInPlayer, { url, player });
}

/** The chosen external player. */
export async function getPlayer(): Promise<string> {
  return invoke<string>(PLAYER_COMMANDS.getPlayer);
}

/** Choose the external player. A blank name restores the default. */
export async function setPlayer(name: string): Promise<void> {
  return invoke<void>(PLAYER_COMMANDS.setPlayer, { name });
}

/** Players the UI offers, in preference order. */
export async function suggestedPlayers(): Promise<string[]> {
  return invoke<string[]>(PLAYER_COMMANDS.suggestedPlayers);
}
