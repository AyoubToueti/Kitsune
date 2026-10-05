// The torrent lifecycle behind a chosen episode: add a release's magnet (or a
// `.torrent` by hand), resolve the stream URL, wait for enough of the file to be
// present, and launch the external player — plus the teardown that keeps a
// closed episode from downloading forever.
//
// Extracted from the watch page so the episode modal and the watch page share
// one copy. The tricky parts — the add race, the held torrent, the auto-launch
// guard — are exactly the kind that rot when duplicated, so they live here once.
//
// A `.svelte.ts` module so it can own runes; a caller gets the effects, and a
// test can wrap it in `$effect.root` (see `src/test/torrent-session-harness.svelte.ts`).

import {
  addMagnet,
  addTorrent,
  getStreamUrl,
  getTorrentStats,
  onPlayerExit,
  openInPlayer,
  pauseTorrent,
  removeTorrent,
  resumeTorrent,
} from "./api/player";
import { errorMessage } from "./api/anime";
import { recordLastPlayed, setListEntry } from "./api/auth";
import { createProgressRecorder } from "./progress";
import { fileForEpisode, firstTargetFile } from "./torrent-files";
import type { Release, TorrentFile, TorrentHandle, TorrentProgress } from "./types";

/**
 * How much of the chosen file must be downloaded before launching a player.
 *
 * A HEURISTIC, not a guarantee. mpv can usually start once the container header
 * and first cluster are present — a few MB, which arrive first because librqbit
 * downloads sequentially — but it may still stall if it needs the file tail (an
 * MKV's `Cues`) for duration or seeking. There is no byte count that is
 * knowably "enough"; this fraction is the trade-off between starting sooner and
 * stalling less.
 */
export const READY_FRACTION = 0.05;

/** How often the status panel is refreshed while waiting. */
export const POLL_INTERVAL_MS = 500;

/** The reactive inputs the session reads when it needs them. */
export interface TorrentSessionConfig {
  /** The work being watched, for progress recording. */
  getId: () => number;
  /** The selected episode number, when one is known. */
  getEpisode: () => number | undefined;
  /** How much to add to a list position to get the absolute episode number. */
  getEpisodeOffset: () => number;
  /**
   * The fraction of the file that must be present before launching, 0..1.
   *
   * Optional: defaults to [`READY_FRACTION`] when the caller has no stored
   * preference. Read live, so changing it in settings applies to the next wait
   * without rebuilding the session.
   */
  getReadyFraction?: () => number;
  /**
   * Called once playback has actually begun, after the player is opened.
   *
   * Lets the caller react (close the modal, say) without the session having to
   * know about it. The progress write itself happens here regardless.
   */
  onLaunched?: () => void;
}

/** The reactive surface a caller renders and drives. */
export interface TorrentSession {
  readonly torrentId: number | null;
  readonly files: TorrentFile[];
  readonly chosen: TorrentFile | null;
  readonly streamUrl: string | undefined;
  readonly progress: TorrentProgress | null;
  readonly fileFraction: number;
  readonly torrentFraction: number;
  /** Recent download-speed samples (MiB/s), newest last, for a sparkline. */
  readonly speedHistory: number[];
  /** Seconds since the byte count last changed, for stall detection. */
  readonly staleSeconds: number;
  /** True while an add or a stream resolve is in flight. */
  readonly loading: boolean;
  /** True while a launch is being requested, so it fires only once. */
  readonly launching: boolean;
  /** True once the player has been opened for the current selection. */
  readonly launched: boolean;
  /**
   * True while the reader has paused the buffering download.
   *
   * While paused the poll keeps running so the panel still shows the frozen
   * progress, but the auto-launch is suspended: opening a player against a
   * paused torrent would stall immediately.
   */
  readonly paused: boolean;
  /** A failure from adding a torrent, resolving a stream or launching. */
  readonly error: string | null;
  /** A failure from opening a magnet in the OS, kept apart from `error`. */
  readonly magnetError: string | null;
  /** A failure from saving a release's `.torrent`, kept apart from `error`. */
  readonly downloadError: string | null;
  /** True while a release's magnet is being added, so the row can say so. */
  readonly loadingRelease: boolean;
  /** The release whose magnet is currently being added, for row emphasis. */
  readonly chosenRelease: Release | null;

  /** Add a release's magnet and start the best-matching file. */
  playRelease(release: Release): Promise<void>;
  /** Add a `.torrent` from a path and start the matching file, if any. */
  loadTorrentFile(path: string): Promise<void>;
  /** Select a file and resolve its stream URL. */
  play(file: TorrentFile): Promise<void>;
  /** Pause the buffering download, suspending auto-launch. */
  pause(): Promise<void>;
  /** Resume a paused download, allowing auto-launch again. */
  resume(): Promise<void>;
  /** Switch the playing file to whatever matches `number`. */
  selectEpisodeNumber(number: number | undefined): void;
  /** Return to the initial state, releasing the torrent. Safe to call twice. */
  reset(): void;
  /** Drop the current torrent. Safe to call more than once. */
  teardown(): void;
}

/**
 * Build a torrent session whose inputs are read reactively.
 *
 * The returned object is the session's whole state and behaviour. Its teardown
 * is tied to the owning component's lifetime: an `$effect` cleanup bumps the
 * add generation and removes the current torrent, so leaving the component
 * releases everything without the caller having to remember.
 */
export function createTorrentSession(
  config: TorrentSessionConfig,
): TorrentSession {
  let torrentId = $state<number | null>(null);
  let files = $state<TorrentFile[]>([]);
  let chosen = $state<TorrentFile | null>(null);
  let streamUrl = $state<string | undefined>(undefined);
  let progress = $state<TorrentProgress | null>(null);
  let paused = $state(false);
  let fileFraction = $state(0);
  let torrentFraction = $state(0);
  let launching = $state(false);
  let launched = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let magnetError = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  let loadingRelease = $state(false);
  let chosenRelease = $state<Release | null>(null);

  /**
   * The last few download-speed samples, newest last.
   *
   * A short ring the status panel draws as a sparkline, so a stall is visible
   * at a glance rather than only as a number that happens to read 0.0. Capped
   * so it never grows without bound while a page is open.
   */
  const SPEED_SAMPLES = 30;
  let speedHistory = $state<number[]>([]);

  /**
   * How long the byte count has been unchanged, in seconds.
   *
   * Not in the snapshot because it is a fact about time, not a reading. Updated
   * on each poll by comparing against the previous byte count; the warning
   * logic reads it to tell "slow" from "stalled".
   */
  let staleSeconds = $state(0);
  /** The last byte count seen, to detect movement between polls. */
  let lastBytes = 0;

  /**
   * Marks the current episode as watched once playback actually starts.
   *
   * Built once for the session's life: the set it keeps must survive across
   * re-selections, or switching files back and forth would re-write the same
   * episode every time.
   */
  const recorder = createProgressRecorder(
    (animeId, episode) => setListEntry(animeId, "current", episode),
    (animeId, episode, err) => {
      console.warn(
        `Could not record progress for ${animeId} episode ${episode ?? "?"}:`,
        err,
      );
    },
  );

  /**
   * Which add is the current one, bumped by every new add AND by teardown.
   *
   * An add can still be resolving when the caller goes away, and the handle it
   * returns names a live torrent that nothing will ever reference. Bumping on
   * teardown as well as on each add makes this counter the whole answer to "is
   * this result still wanted": equal means yes, otherwise the result belongs to
   * a session that is gone or has moved on.
   *
   * Two adds can also overlap — a release clicked and then a `.torrent` picked
   * by hand — and the loser would otherwise never be removed. Sharing one
   * counter between both entry points is what makes that race detectable.
   */
  let addGeneration = 0;

  /** Take ownership of an add's result, or release it when it is unwanted. */
  function adopt(handle: TorrentHandle, generation: number): boolean {
    if (generation === addGeneration) return true;

    void removeTorrent(handle.id);
    return false;
  }

  /**
   * Release the current torrent when it is replaced or the session ends.
   *
   * Reading `torrentId` inside the effect is what registers the dependency, so
   * the cleanup fires both when a new torrent is adopted and on teardown. That
   * covers every add that *completed*; the in-flight case is `adopt`'s job.
   */
  $effect(() => {
    const id = torrentId;
    if (id === null) return;

    return () => {
      void removeTorrent(id);
    };
  });

  // Teardown only: bump the generation so any in-flight add is released, then
  // nothing else may outlive the component.
  $effect(() => () => {
    addGeneration += 1;
  });

  /** Pick the file and resolve its stream URL. */
  async function play(file: TorrentFile): Promise<void> {
    if (torrentId === null) return;

    // Re-picking the file that is already playing must not open a second
    // player; the external player owns playback and there is nothing to do.
    if (launched && chosen?.idx === file.idx) return;

    chosen = file;
    error = null;

    // Picking a file while paused resumes the download: the reader has chosen,
    // so the "wait for a pick" hold no longer applies. `launching` is cleared
    // too, so a launch interrupted by the pause may run again.
    if (paused) {
      try {
        await resumeTorrent(torrentId);
      } catch (err) {
        error = errorMessage(err);
        return;
      }
      paused = false;
      launching = false;
    }

    // A new file is a new wait: clear the previous file's numbers so the panel
    // does not briefly show the last episode's progress as if it were this
    // one's.
    progress = null;
    fileFraction = 0;
    torrentFraction = 0;
    launched = false;
    launching = false;
    try {
      streamUrl = await getStreamUrl(torrentId, file.idx);
    } catch (err) {
      streamUrl = undefined;
      error = errorMessage(err);
      return;
    }

    // The progress is NOT recorded here. Nothing has played yet — the reader is
    // still waiting for bytes — so marking the episode watched would be a lie.
    // It is written by `launch` once a player is actually opened.
  }

  /**
   * Pause the buffering download.
   *
   * The flag is set BEFORE the backend call so the auto-launch guard reads it
   * on the very next poll: a torrent that crosses the ready threshold during
   * the round-trip must not open a player the reader just asked to hold. A
   * failure is surfaced but leaves the flag set -- the download may or may not
   * have paused, and showing "paused" is the safer of the two wrong states
   * because the reader can always press resume.
   */
  async function pause(): Promise<void> {
    if (torrentId === null || paused) return;

    paused = true;
    error = null;
    try {
      await pauseTorrent(torrentId);
    } catch (err) {
      error = errorMessage(err);
    }
  }

  /**
   * Resume a paused download.
   *
   * Clears `launching` as well as `paused`: a launch that was interrupted by
   * the pause must be allowed to run again, or the reader would be stuck
   * waiting for a player that can no longer open.
   */
  async function resume(): Promise<void> {
    if (torrentId === null || !paused) return;

    error = null;
    try {
      await resumeTorrent(torrentId);
    } catch (err) {
      error = errorMessage(err);
      return;
    }

    paused = false;
    launching = false;
  }

  /**
   * Refresh the status panel from the backend.
   *
   * A failure is swallowed rather than surfaced: a dropped poll is transient
   * and the next tick will answer, while flashing an error for every blip would
   * make a working download look broken.
   */
  async function pollProgress(): Promise<void> {
    if (torrentId === null) return;

    let snapshot: TorrentProgress | null;
    try {
      snapshot = await getTorrentStats(torrentId);
    } catch {
      return;
    }
    if (snapshot === null) return;

    progress = snapshot;
    torrentFraction =
      snapshot.totalBytes > 0
        ? snapshot.progressBytes / snapshot.totalBytes
        : 0;
    fileFraction =
      chosen && chosen.lengthBytes > 0
        ? (snapshot.fileProgress[chosen.idx] ?? 0) / chosen.lengthBytes
        : 0;

    // Stall detection: reset the counter whenever more bytes have arrived,
    // otherwise advance it by one poll interval. Only meaningful while live.
    if (snapshot.progressBytes > lastBytes) {
      staleSeconds = 0;
      lastBytes = snapshot.progressBytes;
    } else if (snapshot.state === "live") {
      staleSeconds += POLL_INTERVAL_MS / 1000;
    }

    // Sparkline: one sample per poll, newest last, capped.
    speedHistory = [...speedHistory, snapshot.downloadMbps].slice(
      -SPEED_SAMPLES,
    );
  }

  /**
   * Open the external player, once enough of the file is present.
   *
   * Guarded so it fires exactly once per selection: the poll runs several times
   * a second, and every tick past the threshold would otherwise launch another
   * player. The progress writes happen here rather than in `play` because this
   * is the moment playback actually begins.
   */
  async function launch(): Promise<void> {
    if (launched || launching) return;
    if (streamUrl === undefined || torrentId === null) return;

    launching = true;
    try {
      // No player argument: the backend uses the reader's stored preference,
      // falling back to its default when none was ever chosen.
      await openInPlayer(streamUrl, undefined, torrentId);
    } catch (err) {
      error = errorMessage(err);
      launching = false;
      return;
    }

    launched = true;
    launching = false;

    // Only now that playback has started. The episode NUMBER is recorded, not
    // a list index: the caller derives it through `episodeNumber`, and sending
    // an index would be off by one on every entry. Recorded even without a
    // number: watching a release with no episode selected still means the
    // reader is watching this work.
    const animeId = config.getId();
    const episode = config.getEpisode();
    recorder.record(animeId, episode);

    // Separately, remember this as the work the reader last OPENED. The resume
    // disc reads this rather than the list, because the list only reorders on a
    // CHANGE — re-watching the current episode would not move it. Fire and
    // forget: a failed record must never affect playback.
    void recordLastPlayed(animeId, episode).catch(() => {
      // Swallowed deliberately: this is a convenience hint, and the disc falls
      // back to the list when it is missing.
    });

    config.onLaunched?.();
  }

  /**
   * Poll the torrent while a file is selected and the player has not opened.
   *
   * Keyed on `chosen` and `streamUrl`, so a new selection restarts the wait and
   * the cleanup clears the old timer. It stops as soon as `launched` is set,
   * because there is nothing left to wait for and polling a playing torrent
   * every half second would be pure noise.
   */
  $effect(() => {
    const file = chosen;
    const url = streamUrl;
    if (file === null || url === undefined) return;

    // Poll, then check readiness. The first poll runs immediately rather than
    // waiting a full interval, so an already-downloaded file launches at once
    // instead of after an arbitrary delay.
    //
    // Polling CONTINUES after the player opens: the status panel keeps showing
    // live speed, peers and health while the episode plays, which is what makes
    // the buffer-ahead bar and the slow/stall warnings meaningful mid-playback.
    // `launch` guards itself, so calling it each tick is harmless once launched.
    const tick = () =>
      pollProgress().then(() => {
        if (launched) return;
        // A paused download must not open a player: the reader asked to hold,
        // and an external player against a paused torrent would stall at once.
        if (paused) return;
        const threshold = config.getReadyFraction?.() ?? READY_FRACTION;
        if (fileFraction >= threshold) void launch();
      });

    void tick();
    const timer = setInterval(() => void tick(), POLL_INTERVAL_MS);

    return () => clearInterval(timer);
  });

  /**
   * Start the file inside a freshly-added handle, or wait for a pick.
   *
   * `firstTargetFile` decides: an exact episode match starts, a single video
   * starts, but a multi-video torrent with no match waits -- paused -- for the
   * reader to choose rather than downloading an arbitrary guess.
   */
  async function startBestFile(handle: TorrentHandle): Promise<void> {
    torrentId = handle.id;
    files = handle.files;

    const target = firstTargetFile(
      handle.files,
      config.getEpisode(),
      config.getEpisodeOffset(),
    );

    if (target) {
      await play(target);
    } else {
      await pause();
    }
  }

  /**
   * Move a found release: add its magnet, then play the right file inside it.
   *
   * The episode match is tried first and the best-effort file second, so a
   * release whose files do not spell the episode still starts on something
   * watchable. The whole handle is kept, so the file list stays usable for
   * switching by hand.
   */
  async function playRelease(release: Release): Promise<void> {
    if (loadingRelease) return;

    chosenRelease = release;
    error = null;
    loadingRelease = true;
    try {
      const generation = ++addGeneration;
      const handle = await addMagnet(release.magnetUri);
      if (!adopt(handle, generation)) return;

      await startBestFile(handle);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loadingRelease = false;
    }
  }

  /** Choose a `.torrent` from a path and preselect the matching file, if any. */
  async function loadTorrentFile(path: string): Promise<void> {
    loading = true;
    error = null;
    try {
      const generation = ++addGeneration;
      const handle = await addTorrent(path);
      if (!adopt(handle, generation)) return;

      torrentId = handle.id;
      files = handle.files;

      // Same rule as a release: an exact match or a lone video starts, a
      // multi-video torrent with no match waits for the reader to pick.
      const target = firstTargetFile(
        handle.files,
        config.getEpisode(),
        config.getEpisodeOffset(),
      );

      if (target) {
        await play(target);
      } else {
        await pause();
      }
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  /** Switch the playing file to whatever matches `number`. */
  function selectEpisodeNumber(number: number | undefined): void {
    if (files.length === 0) return;

    const match =
      number !== undefined
        ? fileForEpisode(files, number, config.getEpisodeOffset())
        : null;
    if (match) void play(match);
  }

  /**
   * Return the session to its initial state, releasing the torrent.
   *
   * Clearing `torrentId` is what releases the torrent: the `$effect` above
   * fires its cleanup for the previous id. Bumping the generation releases any
   * add still in flight. Every per-selection field is cleared too, so a caller
   * that reuses the session for another episode does not see the previous
   * one's `launched`/`chosen`/`files` and get stuck on its stage.
   */
  function reset(): void {
    torrentId = null;
    files = [];
    chosen = null;
    streamUrl = undefined;
    progress = null;
    fileFraction = 0;
    torrentFraction = 0;
    launching = false;
    launched = false;
    paused = false;
    loading = false;
    error = null;
    magnetError = null;
    downloadError = null;
    loadingRelease = false;
    chosenRelease = null;
    speedHistory = [];
    staleSeconds = 0;
    lastBytes = 0;
    addGeneration += 1;
  }

  /** Drop the current torrent and clear the selection. Safe to call twice. */
  function teardown(): void {
    reset();
  }

  /**
   * Pause when the external player the reader was watching exits.
   *
   * The backend emits the torrent id that was playing. Matching it against the
   * current id means an exit for a torrent this session no longer tracks is
   * ignored. Unlike a teardown, an exit does NOT remove the torrent or return
   * to the release list: the files and selection are kept, the download is
   * paused, and the reader can pick another file to resume.
   */
  $effect(() => {
    let cancelled = false;
    const unlisten = onPlayerExit((id) => {
      if (cancelled) return;
      if (id !== torrentId) return;
      launched = false;
      launching = false;
      void pause();
    });

    return () => {
      cancelled = true;
      void unlisten.then((fn) => fn());
    };
  });

  return {
    get torrentId() {
      return torrentId;
    },
    get files() {
      return files;
    },
    get chosen() {
      return chosen;
    },
    get streamUrl() {
      return streamUrl;
    },
    get progress() {
      return progress;
    },
    get fileFraction() {
      return fileFraction;
    },
    get torrentFraction() {
      return torrentFraction;
    },
    get speedHistory() {
      return speedHistory;
    },
    get staleSeconds() {
      return staleSeconds;
    },
    get loading() {
      return loading;
    },
    get launching() {
      return launching;
    },
    get paused() {
      return paused;
    },
    get launched() {
      return launched;
    },
    get error() {
      return error;
    },
    get magnetError() {
      return magnetError;
    },
    get downloadError() {
      return downloadError;
    },
    get loadingRelease() {
      return loadingRelease;
    },
    get chosenRelease() {
      return chosenRelease;
    },
    playRelease,
    loadTorrentFile,
    reset,
    play,
    pause,
    resume,
    selectEpisodeNumber,
    teardown,
  };
}