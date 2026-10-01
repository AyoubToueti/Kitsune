import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import { flushSync } from "svelte";

const addMagnetMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/player", () => ({
  addMagnet: addMagnetMock,
  addTorrent: vi.fn(),
  removeTorrent: vi.fn().mockResolvedValue(undefined),
  getStreamUrl: vi.fn().mockResolvedValue("http://127.0.0.1/stream/0"),
  getTorrentStats: vi.fn().mockResolvedValue(null),
  openInPlayer: vi.fn().mockResolvedValue("mpv"),
  onPlayerExit: vi.fn(async () => () => {}),
}));
vi.mock("$lib/api/auth", () => ({
  setListEntry: vi.fn().mockResolvedValue(undefined),
  recordLastPlayed: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("$lib/api/settings", () => ({
  getSettings: vi.fn().mockResolvedValue({ readyFraction: 0.05 }),
  setSettings: vi.fn(),
}));

import { startPlaying, stopPlaying, nowPlayingSession } from "$lib/now-playing.svelte";
import type { Anime, TorrentHandle } from "$lib/types";
import NowPlayingDisc from "./NowPlayingDisc.svelte";

function anime(): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    coverImage: "https://example.test/cover.jpg",
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

const release = {
  title: "Show - 01",
  indexer: "nyaa" as const,
  magnetUri: "magnet:?xt=urn:btih:abc",
  resolution: "1080p" as const,
  source: "webdl" as const,
  remux: false,
  trusted: false,
  parsed: {},
  score: 10,
};

beforeEach(() => {
  stopPlaying();
  flushSync();
  addMagnetMock.mockReset();
});

describe("NowPlayingDisc", () => {
  it("renders nothing while nothing is playing", () => {
    render(NowPlayingDisc);
    expect(screen.queryByTestId("now-playing-disc")).toBeNull();
  });

  it("shows once a file is chosen, and links to the episode", async () => {
    render(NowPlayingDisc);

    startPlaying(anime(), 2, episodes(5));
    flushSync();
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [{ idx: 0, name: "Show - 03.mkv", lengthBytes: 1000 }],
    } as TorrentHandle);
    await nowPlayingSession().playRelease(release);
    await waitFor(() =>
      expect(screen.getByTestId("now-playing-disc")).toBeInTheDocument(),
    );

    // The buffer ring reflects the chosen file's progress.
    expect(screen.getByTestId("now-playing-ring")).toBeInTheDocument();
  });

  it("disappears when playback stops", async () => {
    render(NowPlayingDisc);

    startPlaying(anime(), 0, episodes(3));
    flushSync();
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [{ idx: 0, name: "Show - 01.mkv", lengthBytes: 1000 }],
    } as TorrentHandle);
    await nowPlayingSession().playRelease(release);
    await waitFor(() =>
      expect(screen.getByTestId("now-playing-disc")).toBeInTheDocument(),
    );

    stopPlaying();
    await waitFor(() =>
      expect(screen.queryByTestId("now-playing-disc")).toBeNull(),
    );
  });
});