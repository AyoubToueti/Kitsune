import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

import { harness, settle, type Harness } from "../test/torrent-session-harness.svelte";

const addMagnetMock = vi.hoisted(() => vi.fn());
const addTorrentMock = vi.hoisted(() => vi.fn());
const removeTorrentMock = vi.hoisted(() => vi.fn());
const pauseTorrentMock = vi.hoisted(() => vi.fn());
const resumeTorrentMock = vi.hoisted(() => vi.fn());
const setOnlyFilesMock = vi.hoisted(() => vi.fn());
const getStreamUrlMock = vi.hoisted(() => vi.fn());
const getTorrentStatsMock = vi.hoisted(() => vi.fn());
const openInPlayerMock = vi.hoisted(() => vi.fn());
const chooseAndOpenPlayerMock = vi.hoisted(() => vi.fn());
const openInPlayerChoiceMock = vi.hoisted(() => vi.fn());
const listPlayersMock = vi.hoisted(() => vi.fn());
/**
 * Captures the handler `createTorrentSession` subscribes with, so a test can
 * simulate the backend emitting a player exit. Each subscribe replaces it, as
 * the real listener would be re-established on re-subscription.
 */
const playerExitHandler = vi.hoisted(() => ({
  current: null as ((id: number) => void) | null,
}));
const onPlayerExitMock = vi.hoisted(() =>
  vi.fn(async (handler: (id: number) => void) => {
    playerExitHandler.current = handler;
    return () => {
      if (playerExitHandler.current === handler) {
        playerExitHandler.current = null;
      }
    };
  }),
);
vi.mock("$lib/api/player", () => ({
  addMagnet: addMagnetMock,
  addTorrent: addTorrentMock,
  removeTorrent: removeTorrentMock,
  pauseTorrent: pauseTorrentMock,
  resumeTorrent: resumeTorrentMock,
  setOnlyFiles: setOnlyFilesMock,
  getStreamUrl: getStreamUrlMock,
  getTorrentStats: getTorrentStatsMock,
  openInPlayer: openInPlayerMock,
  chooseAndOpenPlayer: chooseAndOpenPlayerMock,
  openInPlayerChoice: openInPlayerChoiceMock,
  listPlayers: listPlayersMock,
  onPlayerExit: onPlayerExitMock,
}));

const setListEntryMock = vi.hoisted(() => vi.fn());
const recordLastPlayedMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/auth", () => ({
  setListEntry: setListEntryMock,
  recordLastPlayed: recordLastPlayedMock,
}));

import type { Release, TorrentFile, TorrentHandle, TorrentProgress } from "$lib/types";

function file(name: string, lengthBytes = 1000): TorrentFile {
  return { idx: 0, name, lengthBytes };
}

function handle(id: number, files: TorrentFile[]): TorrentHandle {
  return { id, files };
}

function release(): Release {
  return {
    title: "Show - 03 1080p",
    indexer: "nyaa",
    magnetUri: "magnet:?xt=urn:btih:abc",
    resolution: "1080p",
    source: "webdl",
    remux: false,
    trusted: false,
    parsed: {},
    score: 10,
  };
}

/** A stats snapshot with the chosen file fully present, for auto-launch tests. */
function statsReady(): TorrentProgress {
  return {
    state: "live",
    progressBytes: 1000,
    totalBytes: 1000,
    fileProgress: [1000],
    finished: false,
    error: null,
    downloadMbps: 1,
    uploadMbps: 0,
    etaSeconds: 1,
    peersLive: 5,
    peersConnecting: 0,
    peersQueued: 0,
    peersSeen: 5,
  };
}

let current: Harness | null = null;

beforeEach(() => {
  vi.useFakeTimers();
  addMagnetMock.mockReset();
  addTorrentMock.mockReset();
  removeTorrentMock.mockReset();
  pauseTorrentMock.mockReset();
  setOnlyFilesMock.mockReset();
  resumeTorrentMock.mockReset();
  getStreamUrlMock.mockReset();
  getTorrentStatsMock.mockReset();
  openInPlayerChoiceMock.mockReset();
  listPlayersMock.mockReset();
  openInPlayerChoiceMock.mockResolvedValue("mpv");
  listPlayersMock.mockResolvedValue([]);
  openInPlayerMock.mockReset();
  chooseAndOpenPlayerMock.mockReset();
  // `mockClear`, not `mockReset`: the implementation that captures the handler
  // is baked into the hoisted `vi.fn`, and resetting it would drop the seam
  // these tests fire through.
  onPlayerExitMock.mockClear();
  playerExitHandler.current = null;
  setListEntryMock.mockReset();
  recordLastPlayedMock.mockReset();

  removeTorrentMock.mockResolvedValue(undefined);
  pauseTorrentMock.mockResolvedValue(undefined);
  setOnlyFilesMock.mockResolvedValue(undefined);
  resumeTorrentMock.mockResolvedValue(undefined);
  getStreamUrlMock.mockResolvedValue("http://127.0.0.1/stream/0");
  openInPlayerMock.mockResolvedValue("mpv");
  // The auto-launch now prompts the OS chooser instead of opening the stored
  // player directly. A non-empty result means a player was picked, which is
  // what the launch-completed assertions rely on.
  chooseAndOpenPlayerMock.mockResolvedValue("mpv");
  setListEntryMock.mockResolvedValue(undefined);
  recordLastPlayedMock.mockResolvedValue(undefined);
  // Never ready by default, so most tests do not trip the auto-launch.
  getTorrentStatsMock.mockResolvedValue(null);
});

afterEach(() => {
  current?.destroy();
  current = null;
  vi.useRealTimers();
});

describe("playRelease", () => {
  it("adds the magnet and starts a lone video", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv")]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(addMagnetMock).toHaveBeenCalledWith("magnet:?xt=urn:btih:abc");
    expect(h.session.torrentId).toBe(7);
    expect(h.session.chosen?.name).toBe("Show - 03.mkv");
  });

  it("waits, paused, on a multi-video release", async () => {
    addMagnetMock.mockResolvedValue(
      handle(7, [file("Show - 03.mkv"), file("Show - 04.mkv")]),
    );
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // A batch never auto-picks; the reader chooses.
    expect(h.session.chosen).toBeNull();
    expect(h.session.paused).toBe(true);
    expect(pauseTorrentMock).toHaveBeenCalledWith(7);
  });

  it("starts the largest video when a torrent holds exactly one", async () => {
    addMagnetMock.mockResolvedValue(
      handle(7, [file("cover.jpg", 9_000_000), file("episode.mkv", 1_000_000)]),
    );
    const h = harness({ episode: 99 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(h.session.chosen?.name).toBe("episode.mkv");
  });

  it("reports an add failure as a message", async () => {
    addMagnetMock.mockRejectedValue("no peers");
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(h.session.error).toBe("no peers");
    expect(h.session.loadingRelease).toBe(false);
  });

  it("ignores a superseded add and removes its torrent", async () => {
    let resolveAdd!: (h: TorrentHandle) => void;
    addMagnetMock.mockReturnValue(
      new Promise<TorrentHandle>((res) => {
        resolveAdd = res;
      }),
    );
    const h = harness({ episode: 3 });
    current = h;

    const pending = h.session.playRelease(release());
    // The session moves on before the add resolves.
    h.session.teardown();
    resolveAdd(handle(7, [file("Show - 03.mkv")]));
    await pending;
    await settle();

    expect(h.session.torrentId).toBeNull();
    expect(removeTorrentMock).toHaveBeenCalledWith(7);
  });
});

describe("loadTorrentFile", () => {
  it("adds a torrent and preselects the matching file", async () => {
    addTorrentMock.mockResolvedValue(handle(9, [file("Show - 03.mkv")]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.loadTorrentFile("/tmp/x.torrent");
    await settle();

    expect(addTorrentMock).toHaveBeenCalledWith("/tmp/x.torrent");
    expect(h.session.chosen?.name).toBe("Show - 03.mkv");
  });

  it("starts a lone video when nothing matches the episode", async () => {
    addTorrentMock.mockResolvedValue(handle(9, [file("something-else.mkv")]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.loadTorrentFile("/tmp/x.torrent");
    await settle();

    expect(h.session.chosen?.name).toBe("something-else.mkv");
  });

  it("waits, paused, when a multi-video torrent has no match", async () => {
    addTorrentMock.mockResolvedValue(
      handle(9, [file("Show - 01.mkv"), file("Show - 02.mkv")]),
    );
    const h = harness({ episode: 3 });
    current = h;

    await h.session.loadTorrentFile("/tmp/x.torrent");
    await settle();

    // Nothing chosen, no stream resolved, and the torrent held paused.
    expect(h.session.chosen).toBeNull();
    expect(h.session.streamUrl).toBeUndefined();
    expect(h.session.files).toHaveLength(2);
    expect(h.session.paused).toBe(true);
    expect(pauseTorrentMock).toHaveBeenCalledWith(9);
  });

  it("waits even when one name looks like the episode", async () => {
    addTorrentMock.mockResolvedValue(
      handle(9, [file("Show.S01E21.1080p.mkv"), file("Show.S01E22.1080p.mkv")]),
    );
    const h = harness({ episode: 1 });
    current = h;

    await h.session.loadTorrentFile("/tmp/x.torrent");
    await settle();

    // "S01E21" contains "01", which must not be taken for episode 1.
    expect(h.session.chosen).toBeNull();
    expect(h.session.paused).toBe(true);
    expect(pauseTorrentMock).toHaveBeenCalledWith(9);
  });
});

describe("play", () => {
  it("resolves the stream URL and clears previous progress", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv")]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(getStreamUrlMock).toHaveBeenCalledWith(7, 0);
    expect(h.session.streamUrl).toBe("http://127.0.0.1/stream/0");
    expect(h.session.fileFraction).toBe(0);
    expect(h.session.launched).toBe(false);
  });

  it("restricts the download to the chosen file", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv")]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // Without this a pack downloads its files in order and the chosen one
    // sits at 0 B, which reads as a stuck progress bar.
    expect(setOnlyFilesMock).toHaveBeenCalledWith(7, [0]);
  });

  it("reports a stream failure and leaves the URL unset", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv")]));
    getStreamUrlMock.mockRejectedValue("file not ready");
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(h.session.error).toBe("file not ready");
    expect(h.session.streamUrl).toBeUndefined();
  });
});

describe("auto-launch", () => {
  it("offers the picker once enough of the file is present", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // The threshold OPENS the picker; it does not launch a player itself.
    expect(h.session.pickerOpen).toBe(true);
    expect(h.session.launched).toBe(false);
  });

  it("launches the stored player directly when always is set", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3, askEveryTime: false });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // No picker: the stored player is opened straight away.
    expect(h.session.pickerOpen).toBe(false);
    expect(openInPlayerMock).toHaveBeenCalledWith(
      "http://127.0.0.1/stream/0",
      undefined,
      7,
    );
    expect(h.session.launched).toBe(true);
  });

  it("choosePlayer launches the stored player when always is set", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3, askEveryTime: false });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // The button means "play now": it opens the default, not the picker.
    h.session.choosePlayer();
    await settle();

    expect(h.session.pickerOpen).toBe(false);
    expect(openInPlayerMock).toHaveBeenCalled();
  });

  it("choosePlayer opens the picker when no default is set", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3, askEveryTime: true });
    current = h;

    await h.session.playRelease(release());
    await settle();
    h.session.closePicker();

    h.session.choosePlayer();
    expect(h.session.pickerOpen).toBe(true);
  });

  it("does not offer the picker again while it is already open", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // Several more polls past the threshold must not change the state.
    await vi.advanceTimersByTimeAsync(2000);
    await settle();
    expect(h.session.pickerOpen).toBe(true);
    expect(h.session.launched).toBe(false);
  });

  it("records progress only when noteLaunched is called", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // Offering the picker records nothing...
    expect(setListEntryMock).not.toHaveBeenCalled();

    // ...only the picker's confirmation does.
    h.session.noteLaunched();
    await settle();
    expect(h.session.launched).toBe(true);
    expect(h.session.pickerOpen).toBe(false);
    expect(setListEntryMock).toHaveBeenCalledWith(42, "current", 3);
    expect(recordLastPlayedMock).toHaveBeenCalledWith(42, 3);
  });

  it("opens nothing when the picker is closed without a pick", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const launched = vi.fn();
    const h = harness({ episode: 3, launched });
    current = h;

    await h.session.playRelease(release());
    await settle();
    h.session.closePicker();
    await settle();

    // Nothing opened, so playback did NOT start and nothing is recorded.
    expect(h.session.launched).toBe(false);
    expect(launched).not.toHaveBeenCalled();
    expect(setListEntryMock).not.toHaveBeenCalled();
  });

  it("can reopen the picker on demand after a cancel", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    h.session.closePicker();

    h.session.choosePlayer();
    expect(h.session.pickerOpen).toBe(true);
  });
});

describe("teardown", () => {
  it("removes the current torrent", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv")]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    h.session.teardown();
    await settle();

    expect(removeTorrentMock).toHaveBeenCalledWith(7);
    expect(h.session.torrentId).toBeNull();
  });
});

describe("reset", () => {
  it("clears every per-selection field and releases the torrent", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    // Simulate the picker's confirmation, so playback is marked started.
    h.session.noteLaunched();
    await settle();
    // Sanity: the play actually populated the session before we reset it, or
    // the assertions below would pass on an empty session too.
    expect(h.session.launched).toBe(true);
    expect(h.session.files.length).toBeGreaterThan(0);

    h.session.reset();
    await settle();

    expect(h.session.torrentId).toBeNull();
    expect(h.session.files).toEqual([]);
    expect(h.session.chosen).toBeNull();
    expect(h.session.streamUrl).toBeUndefined();
    expect(h.session.launched).toBe(false);
    expect(h.session.chosenRelease).toBeNull();
    expect(removeTorrentMock).toHaveBeenCalledWith(7);
  });
});

describe("player exit", () => {
  it("pauses and keeps the session when the current player exits", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    h.session.noteLaunched();
    await settle();
    expect(h.session.launched).toBe(true);

    // The backend reports the player for torrent 7 closed.
    playerExitHandler.current!(7);
    await settle();

    // Playback stopped, but the torrent, files and selection are kept and the
    // download is paused rather than torn down.
    expect(h.session.launched).toBe(false);
    expect(h.session.paused).toBe(true);
    expect(h.session.torrentId).toBe(7);
    expect(h.session.files).toHaveLength(1);
    expect(h.session.chosen?.name).toBe("Show - 03.mkv");
    expect(pauseTorrentMock).toHaveBeenCalledWith(7);
    expect(removeTorrentMock).not.toHaveBeenCalled();
  });

  it("ignores an exit for a different torrent", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    h.session.noteLaunched();
    await settle();

    // An exit for a torrent this session does not track must not tear it down.
    playerExitHandler.current!(999);
    await settle();

    expect(h.session.torrentId).toBe(7);
    expect(h.session.launched).toBe(true);
  });
});

describe("pause and resume", () => {
  it("pauses the torrent and exposes the paused flag", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    await h.session.pause();
    await settle();

    expect(pauseTorrentMock).toHaveBeenCalledWith(7);
    expect(h.session.paused).toBe(true);
  });

  it("suppresses auto-launch while paused", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    // Ready from the very first poll, so only the paused guard can stop it.
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // Launch already fired; reset the spy and pause, then let more polls run.
    chooseAndOpenPlayerMock.mockClear();
    await h.session.pause();
    await vi.advanceTimersByTimeAsync(2000);
    await settle();

    expect(chooseAndOpenPlayerMock).not.toHaveBeenCalled();
  });

  it("resumes the torrent and clears the paused flag", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    await h.session.pause();
    await settle();

    await h.session.resume();
    await settle();

    expect(resumeTorrentMock).toHaveBeenCalledWith(7);
    expect(h.session.paused).toBe(false);
  });

  it("clears the paused flag on reset", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    await h.session.pause();
    await settle();

    h.session.reset();
    await settle();

    expect(h.session.paused).toBe(false);
  });

  it("resumes the download when a file is picked while paused", async () => {
    addTorrentMock.mockResolvedValue(
      handle(9, [file("Show - 01.mkv"), file("Show - 02.mkv")]),
    );
    const h = harness({ episode: 3 });
    current = h;

    // The multi-video torrent with no match starts paused, waiting for a pick.
    await h.session.loadTorrentFile("/tmp/x.torrent");
    await settle();
    expect(h.session.paused).toBe(true);

    // Picking a file resumes the download and resolves its stream.
    await h.session.play(file("Show - 01.mkv"));
    await settle();

    expect(resumeTorrentMock).toHaveBeenCalledWith(9);
    expect(h.session.paused).toBe(false);
    expect(h.session.streamUrl).toBe("http://127.0.0.1/stream/0");
  });
});