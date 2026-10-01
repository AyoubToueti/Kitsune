import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

import { harness, settle, type Harness } from "../test/torrent-session-harness.svelte";

const addMagnetMock = vi.hoisted(() => vi.fn());
const addTorrentMock = vi.hoisted(() => vi.fn());
const removeTorrentMock = vi.hoisted(() => vi.fn());
const getStreamUrlMock = vi.hoisted(() => vi.fn());
const getTorrentStatsMock = vi.hoisted(() => vi.fn());
const openInPlayerMock = vi.hoisted(() => vi.fn());
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
  getStreamUrl: getStreamUrlMock,
  getTorrentStats: getTorrentStatsMock,
  openInPlayer: openInPlayerMock,
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
  getStreamUrlMock.mockReset();
  getTorrentStatsMock.mockReset();
  openInPlayerMock.mockReset();
  // `mockClear`, not `mockReset`: the implementation that captures the handler
  // is baked into the hoisted `vi.fn`, and resetting it would drop the seam
  // these tests fire through.
  onPlayerExitMock.mockClear();
  playerExitHandler.current = null;
  setListEntryMock.mockReset();
  recordLastPlayedMock.mockReset();

  removeTorrentMock.mockResolvedValue(undefined);
  getStreamUrlMock.mockResolvedValue("http://127.0.0.1/stream/0");
  openInPlayerMock.mockResolvedValue("mpv");
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
  it("adds the magnet and plays the file matching the episode", async () => {
    addMagnetMock.mockResolvedValue(
      handle(7, [file("Show - 03.mkv"), file("Show - 04.mkv")]),
    );
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(addMagnetMock).toHaveBeenCalledWith("magnet:?xt=urn:btih:abc");
    expect(h.session.torrentId).toBe(7);
    expect(h.session.chosen?.name).toBe("Show - 03.mkv");
  });

  it("falls back to the largest playable file when nothing matches", async () => {
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

  it("does not fall back to best-effort when nothing matches", async () => {
    addTorrentMock.mockResolvedValue(handle(9, [file("something-else.mkv")]));
    const h = harness({ episode: 3 });
    current = h;

    await h.session.loadTorrentFile("/tmp/x.torrent");
    await settle();

    expect(h.session.chosen).toBeNull();
    expect(h.session.files).toHaveLength(1);
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
  it("opens the player once enough of the file is present", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const launched = vi.fn();
    const h = harness({ episode: 3, launched });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(openInPlayerMock).toHaveBeenCalledWith(
      "http://127.0.0.1/stream/0",
      undefined,
      7,
    );
    expect(h.session.launched).toBe(true);
    expect(launched).toHaveBeenCalled();
  });

  it("records progress only after launching", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    expect(setListEntryMock).toHaveBeenCalledWith(42, "current", 3);
    expect(recordLastPlayedMock).toHaveBeenCalledWith(42, 3);
  });

  it("launches only once even though the poll keeps running", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    // Let several poll intervals elapse.
    await vi.advanceTimersByTimeAsync(2000);
    await settle();

    expect(openInPlayerMock).toHaveBeenCalledTimes(1);
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
  it("resets the session when the current torrent's player exits", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();
    expect(h.session.launched).toBe(true);

    // The backend reports the player for torrent 7 closed.
    playerExitHandler.current!(7);
    await settle();

    expect(h.session.launched).toBe(false);
    expect(h.session.torrentId).toBeNull();
    expect(removeTorrentMock).toHaveBeenCalledWith(7);
  });

  it("ignores an exit for a different torrent", async () => {
    addMagnetMock.mockResolvedValue(handle(7, [file("Show - 03.mkv", 1000)]));
    getTorrentStatsMock.mockResolvedValue(statsReady());
    const h = harness({ episode: 3 });
    current = h;

    await h.session.playRelease(release());
    await settle();

    // An exit for a torrent this session does not track must not tear it down.
    playerExitHandler.current!(999);
    await settle();

    expect(h.session.torrentId).toBe(7);
    expect(h.session.launched).toBe(true);
  });
});