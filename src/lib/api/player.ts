// Typed wrappers over the player commands in `src-tauri/src/player/commands.rs`.
//
// Separate from `./anime` because the two answer different questions: that
// module fetches metadata from AniList, this one moves bytes off the local
// machine. Mixing them would put torrent state behind the metadata cache,
// which is the last place it belongs.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { TorrentHandle, TorrentProgress } from "$lib/types";

/**
 * Event the backend emits when an external player it launched exits.
 *
 * The payload is the torrent id the player was reading, so the frontend can
 * tell whether the exit belongs to the episode it is currently showing.
 * Without this the app has no way to learn that the reader closed the player,
 * and the torrent would keep downloading behind a panel that says "playing".
 */
export const PLAYER_EXIT_EVENT = "player-exit";

/** Command names, centralised so a rename cannot drift. */
export const PLAYER_COMMANDS = {
  addTorrent: "add_torrent",
  addMagnet: "add_magnet",
  removeTorrent: "remove_torrent",
  pauseTorrent: "pause_torrent",
  resumeTorrent: "resume_torrent",
  setOnlyFiles: "set_only_files",
  streamUrl: "get_stream_url",
  torrentStats: "get_torrent_stats",
  openInPlayer: "open_in_player",
  chooseAndOpenPlayer: "choose_and_open_player",
  choosePlayerPreview: "choose_player_preview",
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

/**
 * Pause a torrent's transfer, keeping the pieces it has already fetched.
 *
 * Not cached, and deliberately fire-and-forget at the call site: a pause that
 * races a teardown is harmless, because the torrent is about to be removed
 * anyway.
 */
export async function pauseTorrent(torrentId: number): Promise<void> {
  return invoke<void>(PLAYER_COMMANDS.pauseTorrent, { torrentId });
}

/**
 * Resume a paused torrent's transfer.
 *
 * The counterpart of [`pauseTorrent`]; the backend continues from the partial
 * data rather than restarting the download.
 */
export async function resumeTorrent(torrentId: number): Promise<void> {
  return invoke<void>(PLAYER_COMMANDS.resumeTorrent, { torrentId });
}

/**
 * Restrict a torrent's download to the given file indices.
 *
 * librqbit downloads a torrent's files in order, so a season pack would spend
 * its time on episode 1 while the reader waits on the one they picked. Calling
 * this with the chosen index makes only that file's pieces get fetched, which
 * is what turns the file-progress bar into real movement.
 */
export async function setOnlyFiles(
  torrentId: number,
  onlyFiles: number[],
): Promise<void> {
  return invoke<void>(PLAYER_COMMANDS.setOnlyFiles, { torrentId, onlyFiles });
}

/** The loopback URL that streams one file of one torrent. */
export async function getStreamUrl(
  torrentId: number,
  fileIdx: number,
): Promise<string> {
  return invoke<string>(PLAYER_COMMANDS.streamUrl, { torrentId, fileIdx });
}

/**
 * A download-progress snapshot for a torrent, for the status panel.
 *
 * Resolves to `null` when there is no session or no such torrent yet, which
 * the panel renders as "not started" rather than as an error. Deliberately not
 * cached: this is a live reading, and a stale one would show progress that has
 * already moved on.
 */
export async function getTorrentStats(
  torrentId: number,
): Promise<TorrentProgress | null> {
  return invoke<TorrentProgress | null>(PLAYER_COMMANDS.torrentStats, {
    torrentId,
  });
}

/**
 * Open a URL in an external player.
 *
 * Resolves to the name of the player that was launched, which is useful when
 * `player` was omitted and the stored preference is unknown to the caller.
 *
 * `torrentId` is the torrent the URL streams from. While the player runs, the
 * backend defers a removal of that torrent, so leaving the watch page does not
 * cut off a player that is still reading it. Omit it for a URL with no torrent
 * behind it.
 */
export async function openInPlayer(
  url: string,
  player?: string,
  torrentId?: number,
): Promise<string> {
  return invoke<string>(PLAYER_COMMANDS.openInPlayer, {
    url,
    player,
    torrentId,
  });
}

/**
 * Open the OS "Open With" chooser, then open `url` in the picked application.
 *
 * The replacement for the silent auto-launch: the desktop's own app chooser
 * lists the video players registered for the file type, so the reader picks
 * per launch instead of the app always using the stored preference. Resolves
 * to the chosen program's name, or an empty string when the dialog was
 * cancelled -- a cancel opens nothing and is not an error.
 *
 * `torrentId` is held for the player's lifetime exactly as in [`openInPlayer`].
 */
export async function chooseAndOpenPlayer(
  url: string,
  torrentId?: number,
): Promise<string> {
  return invoke<string>(PLAYER_COMMANDS.chooseAndOpenPlayer, {
    url,
    torrentId,
  });
}

/**
 * Show the app chooser against a stub URL without opening anything.
 *
 * A development affordance for exercising the dialog with no torrent and no
 * buffering wait. Resolves to the chosen program's name, or an empty string on
 * cancel.
 */
export async function choosePlayerPreview(): Promise<string> {
  return invoke<string>(PLAYER_COMMANDS.choosePlayerPreview);
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

/**
 * Subscribe to external-player exits, calling back with the torrent id.
 *
 * `listen` resolves with an unlisten function rather than returning one, so the
 * caller cannot forget to await it and leak the listener. The callback is
 * wrapped so a malformed payload is dropped rather than thrown into Tauri's
 * event loop, and an id that is not a number is ignored -- applying it to
 * torrent 0 by accident would stop the wrong download.
 */
export async function onPlayerExit(
  handler: (torrentId: number) => void,
): Promise<UnlistenFn> {
  return listen<number>(PLAYER_EXIT_EVENT, (event) => {
    const id = event.payload;
    if (typeof id !== "number") return;
    handler(id);
  });
}
