// The app-wide "now playing" session.
//
// The torrent session used to belong to the watch modal, which created it and
// tore it down when it closed. That is fine while the modal is the only thing
// that plays anything, but it means closing the modal kills the torrent -- so a
// "now playing" disc in the layout would have nothing live to read.
//
// This module owns ONE session for the whole app. It lives under an
// `$effect.root`, so the session's own effects (the poll, the player-exit
// listener, the removal-on-replace) keep running across component unmounts and
// navigations. A component that wants to start playback calls `startPlaying`;
// the modal and the disc both read the same reactive surface.
//
// A singleton, because there is one torrent session in the backend. A second
// instance would fight the first for the same commands.

import { createTorrentSession, type TorrentSession } from "./torrent-session.svelte";
import { getSettings, onSettingsChanged } from "./api/settings";
import type { Anime } from "./types";
import type { Episode } from "./episodes";
import { absoluteOffset, episodeNumber } from "./episode";

/** What is currently being watched. */
interface NowPlaying {
  anime: Anime;
  /** The selected episode's index into the caller's episode list. */
  episodeIndex: number;
  /** The episode number to record progress against, when one is known. */
  episode?: number;
  /** The cour offset, so the file matcher can try the absolute spelling. */
  offset: number;
}

let current = $state<NowPlaying | null>(null);

/**
 * The auto-launch threshold, read once.
 *
 * A change to it in settings applies to the next session start, not the running
 * one; re-reading on every poll would be wasteful and the value is stable.
 */
let readyFraction: number | undefined;

/**
 * Whether to prompt for a player, read alongside the threshold.
 *
 * `undefined` until the settings land, which the session reads as "ask" -- the
 * safe default, since a prompt is recoverable and a silent launch is not.
 */
let askEveryTime: boolean | undefined;

// Deliberately NOT a top-level `await`: that makes this module async, and
// SvelteKit's client entry imports the chain reaching it (layout -> disc ->
// here). An async module delays `start()` past the point the app's `component`
// is initialised, which throws "Cannot access 'component' before
// initialization". A fire-and-forget read keeps the import synchronous; the
// session falls back to its own default if this has not landed yet.
void getSettings()
  .then((settings) => {
    readyFraction = settings.readyFraction;
    askEveryTime = settings.askEveryTime;
  })
  .catch(() => {
    // The session falls back to its own default.
  });

/**
 * Re-read the settings when they change elsewhere.
 *
 * The "Always" button and the Settings toggle both write, so without this a
 * running session would keep the value it read at startup and go on prompting
 * (or not) against the reader's latest choice.
 */
void onSettingsChanged((settings) => {
  readyFraction = settings.readyFraction;
  askEveryTime = settings.askEveryTime;
}).catch(() => {
  // A listener that never installs leaves both values at their startup
  // defaults, which is the same fallback the getSettings read above already
  // accepts. Without this the rejection is unhandled.
});

/**
 * The session, created on first use.
 *
 * LAZY on purpose: creating it subscribes to the backend's player-exit event,
 * which reaches Tauri -- so building it at import time would fire a command for
 * any module that merely imports this one (every test that touches `isActive`).
 * Building it on the first `nowPlayingSession()` keeps the import side-effect
 * free and still gives the app exactly one session for its life.
 *
 * `$effect.root` gives the runes inside `createTorrentSession` an owner that
 * never tears down on its own; the session IS the app's playback and lives
 * until the process ends.
 */
let session: TorrentSession | null = null;

function ensureSession(): TorrentSession {
  if (session !== null) return session;

  let created!: TorrentSession;
  $effect.root(() => {
    created = createTorrentSession({
      getId: () => current?.anime.id ?? 0,
      getEpisode: () => current?.episode,
      getEpisodeOffset: () => current?.offset ?? 0,
      getReadyFraction: () => readyFraction ?? 0.05,
      getAskEveryTime: () => askEveryTime ?? true,
    });
  });
  session = created;
  return session;
}

/** The reactive session, for a component that renders its state. */
export function nowPlayingSession(): TorrentSession {
  return ensureSession();
}

/** The work being watched, or `null` when nothing is. */
export function nowPlaying(): NowPlaying | null {
  return current;
}

/**
 * Whether a disc should show.
 *
 * True from the moment a file is chosen (so the disc covers the buffering wait
 * too) until the player exits, at which point the session resets and this
 * follows it back to false.
 */
export function isActive(): boolean {
  // Deliberately does NOT create the session: asking "is anything playing?"
  // must not have the side effect of starting one. Before a session exists,
  // nothing can be chosen, so the answer is false.
  return current !== null && session !== null && session.chosen !== null;
}

/**
 * Begin watching an episode.
 *
 * Resets any previous session first, so a second episode replaces the first
 * rather than layering on it -- the backend has a single session, and a stale
 * torrent would otherwise keep downloading behind the new one. The caller then
 * drives the new session (add a magnet, pick a file) exactly as before.
 */
export function startPlaying(
  anime: Anime,
  episodeIndex: number,
  episodes: Episode[],
): void {
  // Idempotent for the SAME work and episode. The watch modal calls this from
  // an effect that re-runs whenever it reopens -- including when the reader
  // clicks the now-playing disc to bring the modal back for the episode already
  // playing. Resetting then would clear `chosen`, hide the disc, and remove the
  // running torrent. Only a genuine switch (a different work or episode)
  // resets.
  if (
    current !== null &&
    current.anime.id === anime.id &&
    current.episodeIndex === episodeIndex
  ) {
    return;
  }

  const episode = episodes[episodeIndex];
  current = {
    anime,
    episodeIndex,
    episode: episode ? episodeNumber(episode) : undefined,
    offset: absoluteOffset(episodes),
  };

  // A fresh selection starts from nothing: the previous torrent is released and
  // the stage returns to the release list.
  ensureSession().reset();
}

/** Stop watching and release the torrent. */
export function stopPlaying(): void {
  // Only reset an existing session: stopping nothing must not start one.
  session?.reset();
  current = null;
}

// --- opening the watch modal from the disc --------------------------------
//
// The disc cannot simply navigate to `?ep=` to reopen the modal: that parameter
// only fires on a CHANGE, so clicking from the very page you are watching --
// where the URL already says `?ep=` -- would be a no-op. A request signal that
// the detail page consumes is reliable from any page and any URL.
//
// `pending` is a plain flag, not `$state`: the page clears it inside an effect,
// and a reactive write there would re-run that effect. `signal` is the
// reactive half -- it changes so the page's effect wakes up and looks.

let pendingOpen = false;
let openSignal = $state(0);

/** Ask the detail page to open the watch modal for what is playing. */
export function requestOpen(): void {
  pendingOpen = true;
  openSignal += 1;
}

/** The open-request signal, read reactively so an effect re-runs on a bump. */
export function openRequested(): number {
  return openSignal;
}

/**
 * Take the pending open request, if any.
 *
 * True exactly once per [`requestOpen`]. The page calls this only when it can
 * actually open (its work matches the playing one), so a request that arrives
 * before the episode list has loaded is not swallowed.
 */
export function consumeOpenRequest(): boolean {
  if (!pendingOpen) return false;
  pendingOpen = false;
  return true;
}