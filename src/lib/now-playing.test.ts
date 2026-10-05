import { describe, it, expect, vi, beforeEach } from "vitest";

import { flushSync } from "svelte";

const addMagnetMock = vi.hoisted(() => vi.fn());
const removeTorrentMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const getStreamUrlMock = vi.hoisted(() => vi.fn());
const getTorrentStatsMock = vi.hoisted(() => vi.fn().mockResolvedValue(null));
const openInPlayerMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/player", () => ({
  addMagnet: addMagnetMock,
  addTorrent: vi.fn(),
  removeTorrent: removeTorrentMock,
  getStreamUrl: getStreamUrlMock,
  getTorrentStats: getTorrentStatsMock,
  openInPlayer: openInPlayerMock,
  onPlayerExit: vi.fn(async () => () => {}),
}));

vi.mock("$lib/api/auth", () => ({
  setListEntry: vi.fn().mockResolvedValue(undefined),
  recordLastPlayed: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("$lib/api/settings", () => ({
  getSettings: vi.fn().mockResolvedValue({ readyFraction: 0.05, askEveryTime: true }),
  onSettingsChanged: vi.fn(async () => () => {}),
  setSettings: vi.fn(),
}));

import {
  consumeOpenRequest,
  isActive,
  nowPlaying,
  nowPlayingSession,
  openRequested,
  requestOpen,
  startPlaying,
  stopPlaying,
} from "./now-playing.svelte";
import type { Anime, TorrentHandle } from "./types";

function anime(): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
  };
}

function episodes(count: number) {
  return Array.from({ length: count }, (_, i) => ({
    number: i + 1,
    title: `Episode ${i + 1}`,
  }));
}

beforeEach(() => {
  addMagnetMock.mockReset();
  removeTorrentMock.mockClear();
  stopPlaying();
  flushSync();
});

describe("now-playing store", () => {
  it("starts inactive", () => {
    expect(isActive()).toBe(false);
    expect(nowPlaying()).toBeNull();
  });

  it("records the work and episode on startPlaying", () => {
    startPlaying(anime(), 2, episodes(5));
    flushSync();

    const playing = nowPlaying();
    expect(playing?.anime.id).toBe(21);
    expect(playing?.episodeIndex).toBe(2);
    expect(playing?.episode).toBe(3);
  });

  it("is active only once a file is chosen", async () => {
    startPlaying(anime(), 0, episodes(3));
    flushSync();
    // Nothing chosen yet.
    expect(isActive()).toBe(false);

    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [{ idx: 0, name: "Show - 01.mkv", lengthBytes: 1000 }],
    } as TorrentHandle);

    await nowPlayingSession().playRelease({
      title: "Show - 01",
      indexer: "nyaa",
      magnetUri: "magnet:?xt=urn:btih:abc",
      resolution: "1080p",
      source: "webdl",
      remux: false,
      trusted: false,
      parsed: {},
      score: 10,
    });
    flushSync();

    expect(isActive()).toBe(true);
  });

  it("clears everything on stopPlaying", () => {
    startPlaying(anime(), 1, episodes(4));
    flushSync();
    expect(nowPlaying()).not.toBeNull();

    stopPlaying();
    flushSync();

    expect(nowPlaying()).toBeNull();
    expect(isActive()).toBe(false);
    expect(nowPlayingSession().chosen).toBeNull();
  });

  it("replaces a previous episode on a new startPlaying", () => {
    startPlaying(anime(), 0, episodes(3));
    flushSync();
    startPlaying(anime(), 1, episodes(3));
    flushSync();

    expect(nowPlaying()?.episodeIndex).toBe(1);
  });

  it("does not reset the session when reopened for the same episode", async () => {
    startPlaying(anime(), 1, episodes(4));
    flushSync();
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [{ idx: 0, name: "Show - 02.mkv", lengthBytes: 1000 }],
    } as TorrentHandle);
    await nowPlayingSession().playRelease({
      title: "Show - 02",
      indexer: "nyaa",
      magnetUri: "magnet:?xt=urn:btih:abc",
      resolution: "1080p",
      source: "webdl",
      remux: false,
      trusted: false,
      parsed: {},
      score: 10,
    });
    flushSync();
    expect(isActive()).toBe(true);

    // The modal reopening for the same episode must NOT tear the session down,
    // or the disc would vanish mid-playback.
    startPlaying(anime(), 1, episodes(4));
    flushSync();

    expect(isActive()).toBe(true);
    expect(nowPlayingSession().chosen?.name).toBe("Show - 02.mkv");
    expect(removeTorrentMock).not.toHaveBeenCalled();
  });

  it("resets when switching to a different episode", async () => {
    startPlaying(anime(), 1, episodes(4));
    flushSync();
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [{ idx: 0, name: "Show - 02.mkv", lengthBytes: 1000 }],
    } as TorrentHandle);
    await nowPlayingSession().playRelease({
      title: "Show - 02",
      indexer: "nyaa",
      magnetUri: "magnet:?xt=urn:btih:abc",
      resolution: "1080p",
      source: "webdl",
      remux: false,
      trusted: false,
      parsed: {},
      score: 10,
    });
    flushSync();

    startPlaying(anime(), 2, episodes(4));
    flushSync();

    // A real switch clears the old selection.
    expect(nowPlayingSession().chosen).toBeNull();
    expect(nowPlaying()?.episodeIndex).toBe(2);
  });

  it("hands an open request to the page exactly once", () => {
    // No request yet.
    expect(consumeOpenRequest()).toBe(false);

    requestOpen();
    // The signal moves so a page effect wakes up.
    const before = openRequested();
    requestOpen();
    expect(openRequested()).toBeGreaterThan(before);

    // Consumed once, then gone.
    expect(consumeOpenRequest()).toBe(true);
    expect(consumeOpenRequest()).toBe(false);
  });
});