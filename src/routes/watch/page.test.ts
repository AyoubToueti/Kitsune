import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/svelte";

import type { Anime, TorrentHandle } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js.
import { page as appState } from "$app/state";

const getAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getAnime: getAnimeMock };
});

const openDialogMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialogMock }));

const addTorrentMock = vi.hoisted(() => vi.fn());
const getStreamUrlMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/player", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/player")>("$lib/api/player");
  return {
    ...actual,
    addTorrent: addTorrentMock,
    getStreamUrl: getStreamUrlMock,
    // The sidebar button fetches these; stub them so the page renders.
    getPlayer: vi.fn().mockResolvedValue("mpv"),
    suggestedPlayers: vi.fn().mockResolvedValue(["mpv"]),
  };
});

import Page from "./[id]/+page.svelte";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 16498,
    provider: "anilist",
    title: { romaji: "Attack on Titan" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

function handle(): TorrentHandle {
  return {
    id: 5,
    files: [
      { idx: 0, name: "[Group] Show - 01 [1080p].mkv", lengthBytes: 1_400_000_000 },
      { idx: 1, name: "[Group] Show - 02 [1080p].mkv", lengthBytes: 1_400_000_000 },
    ],
  };
}

function setId(id: string | number) {
  const state = appState as { url: URL; params: Record<string, string> };
  state.url = new URL(`http://localhost/watch/${id}`);
  state.params = { id: String(id) };
}

beforeEach(() => {
  getAnimeMock.mockReset().mockResolvedValue(anime());
  openDialogMock.mockReset().mockResolvedValue("/tmp/show.torrent");
  addTorrentMock.mockReset().mockResolvedValue(handle());
  getStreamUrlMock
    .mockReset()
    .mockResolvedValue("http://127.0.0.1:3030/torrents/5/stream/0");
  setId(16498);
});

describe("watch page", () => {
  it("shows a loading state while the metadata request is in flight", () => {
    getAnimeMock.mockReturnValue(new Promise(() => {}));

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("loads the anime named in the route", async () => {
    setId(42);

    render(Page);

    expect(await screen.findByRole("heading", { name: /attack on titan/i })).toBeInTheDocument();
    expect(getAnimeMock).toHaveBeenCalledWith(42);
  });

  it("shows the empty player state before a torrent is loaded", async () => {
    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    expect(screen.getByText(/load a torrent/i)).toBeInTheDocument();
  });

  it("surfaces a metadata failure", async () => {
    getAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("says so when the anime is missing", async () => {
    getAnimeMock.mockResolvedValue(null);

    render(Page);

    expect(await screen.findByText(/not found/i)).toBeInTheDocument();
  });

  it("loads a torrent and lists its files", async () => {
    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    await fireEvent.click(screen.getByRole("button", { name: /load torrent/i }));

    expect(openDialogMock).toHaveBeenCalled();
    expect(addTorrentMock).toHaveBeenCalledWith("/tmp/show.torrent");

    const files = await screen.findByTestId("torrent-files");
    expect(within(files).getAllByRole("button")).toHaveLength(2);
  });

  it("does nothing when the file picker is cancelled", async () => {
    openDialogMock.mockResolvedValue(null);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await fireEvent.click(screen.getByRole("button", { name: /load torrent/i }));

    // A cancelled picker is not an error and must not add a torrent.
    expect(addTorrentMock).not.toHaveBeenCalled();
  });

  it("plays the file that matches the selected episode", async () => {
    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await fireEvent.click(screen.getByRole("button", { name: /load torrent/i }));

    const files = await screen.findByTestId("torrent-files");
    await fireEvent.click(within(files).getAllByRole("button")[1]);

    expect(getStreamUrlMock).toHaveBeenCalledWith(5, 1);
    // The stream URL is handed to the video element.
    await screen.findByRole("button", { name: /open in external player/i });
  });

  it("surfaces a torrent failure", async () => {
    addTorrentMock.mockRejectedValue("torrent produced no files");

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await fireEvent.click(screen.getByRole("button", { name: /load torrent/i }));

    expect(await screen.findByText(/produced no files/i)).toBeInTheDocument();
  });

  it("preselects the file matching the current episode", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        streamingEpisodes: [
          { url: "https://x.test/1", title: "Episode 1" },
          { url: "https://x.test/2", title: "Episode 2" },
        ],
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    // Select episode 2, then load the torrent: file 02 should play without a
    // second click.
    const list = await screen.findByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[1]);

    await fireEvent.click(screen.getByRole("button", { name: /load torrent/i }));

    expect(getStreamUrlMock).toHaveBeenCalledWith(5, 1);
  });

  it("offers the relations sidebar", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        relations: [
          {
            id: 20958,
            title: { romaji: "Attack on Titan Season 2" },
            relationType: "SEQUEL",
          },
        ],
      }),
    );

    render(Page);

    expect(await screen.findByText("Attack on Titan Season 2")).toBeInTheDocument();
  });
});