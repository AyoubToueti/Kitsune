import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@testing-library/svelte";

import type { Anime, EpisodeInfo, Release, TorrentHandle } from "$lib/types";
import type { Episode } from "$lib/episodes";

const openDialogMock = vi.hoisted(() => vi.fn());
const saveDialogMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: openDialogMock,
  save: saveDialogMock,
}));

const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

const searchReleasesMock = vi.hoisted(() => vi.fn());
const probeReleasesMock = vi.hoisted(() => vi.fn());
const onProbeResultMock = vi.hoisted(() => vi.fn());
const downloadTorrentMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/releases", () => ({
  searchReleases: searchReleasesMock,
  probeReleases: probeReleasesMock,
  onProbeResult: onProbeResultMock,
  downloadTorrent: downloadTorrentMock,
}));

const addMagnetMock = vi.hoisted(() => vi.fn());
const addTorrentMock = vi.hoisted(() => vi.fn());
const removeTorrentMock = vi.hoisted(() => vi.fn());
const getStreamUrlMock = vi.hoisted(() => vi.fn());
const getTorrentStatsMock = vi.hoisted(() => vi.fn());
const openInPlayerMock = vi.hoisted(() => vi.fn());
// The session subscribes to player exits; a never-firing stub is enough here,
// and returning an unlisten function matches the real shape so cleanup works.
const onPlayerExitMock = vi.hoisted(() =>
  vi.fn(async () => () => {}),
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

// The footer's speed test subscribes to progress on open; a never-firing stub
// is enough, and returning an unlisten matches the real shape.
const runSpeedTestMock = vi.hoisted(() => vi.fn());
const onSpeedTestProgressMock = vi.hoisted(() => vi.fn(async () => () => {}));
vi.mock("$lib/api/diagnostics", () => ({
  runSpeedTest: runSpeedTestMock,
  onSpeedTestProgress: onSpeedTestProgressMock,
}));

import EpisodeWatchModal from "./EpisodeWatchModal.svelte";
import { stopPlaying } from "$lib/now-playing.svelte";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 16498,
    provider: "anilist",
    title: { romaji: "Attack on Titan", english: "Attack on Titan" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

function episodes(count = 3): Episode[] {
  return Array.from({ length: count }, (_, i) => ({
    number: i + 1,
    title: `Episode ${i + 1}`,
  }));
}

function release(title: string): Release {
  return {
    title,
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

beforeEach(() => {
  // The now-playing store is a module singleton, so its state outlives a test.
  // Clear it so each test starts with nothing playing -- otherwise an
  // idempotent `startPlaying` for the same work and episode would keep the
  // previous test's session.
  stopPlaying();
  searchReleasesMock.mockReset().mockResolvedValue([]);
  probeReleasesMock.mockReset().mockResolvedValue([]);
  onProbeResultMock.mockReset().mockResolvedValue(() => {});
  downloadTorrentMock.mockReset().mockResolvedValue("/tmp/x.torrent");
  addMagnetMock.mockReset();
  addTorrentMock.mockReset();
  removeTorrentMock.mockReset().mockResolvedValue(undefined);
  getStreamUrlMock.mockReset().mockResolvedValue("http://127.0.0.1/stream/0");
  getTorrentStatsMock.mockReset().mockResolvedValue(null);
  openInPlayerMock.mockReset().mockResolvedValue("mpv");
  setListEntryMock.mockReset().mockResolvedValue(undefined);
  recordLastPlayedMock.mockReset().mockResolvedValue(undefined);
  openDialogMock.mockReset();
  saveDialogMock.mockReset();
  openUrlMock.mockReset();
  runSpeedTestMock.mockReset();
  onSpeedTestProgressMock.mockReset().mockResolvedValue(() => {});
});

describe("EpisodeWatchModal", () => {
  it("does not search while closed", () => {
    render(EpisodeWatchModal, {
      props: {
        open: false,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    expect(searchReleasesMock).not.toHaveBeenCalled();
  });

  it("shows the episode header", async () => {
    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 2,
        onClose: () => {},
      },
    });

    // The eyebrow shows the number; the title shows the entry's own text.
    expect(screen.getByRole("heading", { level: 2 })).toBeInTheDocument();
    expect(screen.getAllByText(/Episode 3/).length).toBeGreaterThan(0);
  });

  it("searches for the selected episode's number", async () => {
    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 2,
        onClose: () => {},
      },
    });

    await waitFor(() =>
      expect(searchReleasesMock).toHaveBeenCalledWith(
        expect.arrayContaining(["Attack on Titan"]),
        3,
        undefined,
      ),
    );
  });

  it("lists the releases a search returned", async () => {
    searchReleasesMock.mockResolvedValue([release("Show - 03 1080p")]);
    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 2,
        onClose: () => {},
      },
    });

    expect(
      await screen.findByText("Show - 03 1080p"),
    ).toBeInTheDocument();
  });

  it("shows an empty message when nothing is found", async () => {
    searchReleasesMock.mockResolvedValue([]);
    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    expect(
      await screen.findByTestId("releases-empty"),
    ).toBeInTheDocument();
  });

  it("plays a release: adds its magnet and shows the files", async () => {
    searchReleasesMock.mockResolvedValue([release("Show - 01 1080p")]);
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [
        { idx: 0, name: "Show - 01.mkv", lengthBytes: 1000 },
        { idx: 1, name: "Show - 01.ass", lengthBytes: 10 },
      ],
    } as TorrentHandle);

    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    await fireEvent.click(await screen.findByTestId("release-play"));

    expect(addMagnetMock).toHaveBeenCalledWith("magnet:?xt=urn:btih:abc");
    expect(
      await screen.findByTestId("torrent-files"),
    ).toBeInTheDocument();
  });

  it("labels each file row with its kind and size", async () => {
    searchReleasesMock.mockResolvedValue([release("Show - 01 1080p")]);
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [
        { idx: 0, name: "Show - 01.mkv", lengthBytes: 1_400_000_000 },
        { idx: 1, name: "Show - 01.ass", lengthBytes: 42_000 },
      ],
    } as TorrentHandle);

    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    await fireEvent.click(await screen.findByTestId("release-play"));
    const list = await screen.findByTestId("torrent-files");

    // Each row names what the file is, so the list is scannable rather than a
    // wall of filenames.
    expect(within(list).getByText(/^Video · /)).toBeInTheDocument();
    expect(within(list).getByText(/^Subtitle · /)).toBeInTheDocument();
  });

  it("marks the auto-matched file as Matched", async () => {
    searchReleasesMock.mockResolvedValue([release("Show - 01 1080p")]);
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [
        { idx: 0, name: "Show - 01.mkv", lengthBytes: 1_400_000_000 },
        { idx: 1, name: "Show - 02.mkv", lengthBytes: 1_400_000_000 },
      ],
    } as TorrentHandle);

    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    await fireEvent.click(await screen.findByTestId("release-play"));
    const list = await screen.findByTestId("torrent-files");

    expect(within(list).getByText("Matched")).toBeInTheDocument();
  });

  it("keeps the live status panel after the player opens", async () => {
    searchReleasesMock.mockResolvedValue([release("Show - 01 1080p")]);
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [{ idx: 0, name: "Show - 01.mkv", lengthBytes: 1000 }],
    } as TorrentHandle);
    // The player opens; the panel must NOT collapse to a static screen.
    getTorrentStatsMock.mockResolvedValue({
      state: "live",
      progressBytes: 1000,
      totalBytes: 1000,
      fileProgress: [1000],
      finished: false,
      error: null,
      downloadMbps: 4,
      uploadMbps: 0,
      etaSeconds: 5,
      peersLive: 8,
      peersConnecting: 0,
      peersQueued: 0,
      peersSeen: 10,
    });

    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    await fireEvent.click(await screen.findByTestId("release-play"));

    // The playing banner appears, but the download panel and file list stay.
    await waitFor(() =>
      expect(screen.getByTestId("playing-banner")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("stream-status")).toBeInTheDocument();
    expect(screen.getByTestId("torrent-files")).toBeInTheDocument();
    expect(screen.queryByTestId("stage-playing")).toBeNull();
  });

  it("calls onClose from the header button", async () => {
    const onClose = vi.fn();
    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose,
      },
    });

    await fireEvent.click(screen.getByLabelText("Close"));
    expect(onClose).toHaveBeenCalled();
  });

  it("returns to the releases when 'Change torrent' is clicked with a torrent loaded", async () => {
    searchReleasesMock.mockResolvedValue([release("Show - 01 1080p")]);
    addMagnetMock.mockResolvedValue({
      id: 7,
      files: [
        { idx: 0, name: "Show - 01.mkv", lengthBytes: 1000 },
        { idx: 1, name: "Show - 01.ass", lengthBytes: 10 },
      ],
    } as TorrentHandle);

    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    await fireEvent.click(await screen.findByTestId("release-play"));
    // A torrent is now loaded, so the footer offers "Change torrent".
    await screen.findByTestId("torrent-files");

    await fireEvent.click(screen.getByText("Change torrent"));

    // It goes back to the release list rather than opening the file picker.
    expect(await screen.findByTestId("stage-releases")).toBeInTheDocument();
    expect(openDialogMock).not.toHaveBeenCalled();
  });

  it("runs a speed test from the footer and shows the inline result", async () => {
    runSpeedTestMock.mockResolvedValue({
      latencyMs: 18,
      jitterMs: 2,
      downloadMbps: 42,
      bytesDownloaded: 10_000_000,
      durationMs: 8000,
      verdict: "good",
    });

    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    await fireEvent.click(await screen.findByTestId("modal-speed-test"));

    expect(await screen.findByTestId("modal-speed-result")).toHaveTextContent(
      "42 Mbps · 18 ms",
    );
  });

  it("shows a footer speed-test error without breaking the modal", async () => {
    runSpeedTestMock.mockRejectedValue(new Error("a speed test is already running"));

    render(EpisodeWatchModal, {
      props: {
        open: true,
        anime: anime(),
        episodes: episodes(),
        episodeIndex: 0,
        onClose: () => {},
      },
    });

    await fireEvent.click(await screen.findByTestId("modal-speed-test"));

    expect(await screen.findByTestId("modal-speed-error")).toHaveTextContent(
      /already running/i,
    );
  });
});