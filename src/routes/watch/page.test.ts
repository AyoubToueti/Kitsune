import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within, waitFor } from "@testing-library/svelte";

import type { Anime, ProbeOutcome, Release, TorrentHandle } from "$lib/types";

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
const addMagnetMock = vi.hoisted(() => vi.fn());
const getStreamUrlMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/player", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/player")>("$lib/api/player");
  return {
    ...actual,
    addTorrent: addTorrentMock,
    addMagnet: addMagnetMock,
    getStreamUrl: getStreamUrlMock,
    // The sidebar button fetches these; stub them so the page renders.
    getPlayer: vi.fn().mockResolvedValue("mpv"),
    suggestedPlayers: vi.fn().mockResolvedValue(["mpv"]),
  };
});

const searchReleasesMock = vi.hoisted(() => vi.fn());
const probeReleasesMock = vi.hoisted(() => vi.fn());
const onProbeResultMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/releases", () => ({
  searchReleases: searchReleasesMock,
  probeReleases: probeReleasesMock,
  onProbeResult: onProbeResultMock,
}));

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

function release(overrides: Partial<Release> = {}): Release {
  return {
    title: "[Group] Show - 01 [1080p]",
    indexer: "nyaa",
    magnetUri: "magnet:?xt=urn:btih:abc",
    infoHash: "abc",
    sizeBytes: 1_400_000_000,
    seeders: 12,
    resolution: "1080p",
    source: "webdl",
    remux: false,
    trusted: false,
    parsed: { title: "Show", absoluteEpisode: 1 },
    score: 0,
    ...overrides,
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
  addMagnetMock.mockReset().mockResolvedValue(handle());
  getStreamUrlMock
    .mockReset()
    .mockResolvedValue("http://127.0.0.1:3030/torrents/5/stream/0");
  // Default: no releases, so tests that do not care are unaffected.
  searchReleasesMock.mockReset().mockResolvedValue([]);
  // Probing resolves with nothing and reports no progress by default. It
  // returns an unlisten function the page awaits, so the mock must resolve
  // rather than return undefined.
  probeReleasesMock.mockReset().mockResolvedValue([]);
  onProbeResultMock.mockReset().mockResolvedValue(() => {});
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

    // Scoped to the player's own copy: the releases panel also mentions
    // loading a torrent, so a bare /load a torrent/i would match twice.
    expect(
    screen.getByText(/load a torrent to start watching/i),
    ).toBeInTheDocument();
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

  it("searches for releases using the work title", async () => {
    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    expect(await screen.findByTestId("releases-empty")).toBeInTheDocument();
    expect(searchReleasesMock).toHaveBeenCalledWith("Attack on Titan", undefined);
  });

  it("plays a release the app found on its own", async () => {
    searchReleasesMock.mockResolvedValue([release()]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByRole("button"));

    expect(addMagnetMock).toHaveBeenCalledWith("magnet:?xt=urn:btih:abc");
    // No episode selected, so it falls back to the first playable file.
    expect(getStreamUrlMock).toHaveBeenCalledWith(5, 0);
  });

  it("narrows the search to the selected episode", async () => {
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

    const list = await screen.findByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[1]);

    expect(searchReleasesMock).toHaveBeenLastCalledWith("Attack on Titan", 2);
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

  it("marks releases as unprobed until a verdict arrives", async () => {
    searchReleasesMock.mockResolvedValue([release()]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const badge = await screen.findByTestId("release-badge");
    expect(badge).toHaveAttribute("data-badge", "pending");
  });

  it("applies a probe verdict to the matching release", async () => {
    searchReleasesMock.mockResolvedValue([release()]);

    // Capture the handler the page registers, so a probe event can be fired
    // as the backend would. Resolving with a no-op unlisten keeps the page's
    // cleanup path valid.
    let fire: ((outcome: ProbeOutcome) => void) | undefined;
    onProbeResultMock.mockImplementation(
      (handler: (outcome: ProbeOutcome) => void) => {
        fire = handler;
        return Promise.resolve(() => {});
      },
    );

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");

    // The listener is registered from an effect, so wait for it before firing.
    await waitFor(() => expect(fire).toBeDefined());
    fire!({
      index: 0,
      badge: "green",
      combinedScore: 500,
      probe: {
        infoHash: "abc",
        metadata: { resolved: true, fileCount: 1, totalBytes: 10, durationMs: 5 },
        scrape: {
          seeders: 42,
          leechers: 1,
          completed: 3,
          trackerUrl: "udp://t.test:80",
          durationMs: 2,
        },
        totalDurationMs: 7,
      },
    });

    await waitFor(() =>
      expect(screen.getByTestId("release-badge")).toHaveAttribute(
        "data-badge",
        "green",
      ),
    );
  });

  it("re-ranks a probed release above an unprobed one", async () => {
    // Two releases: the first leads on static score, the second does not.
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA static leader", score: 900 }),
      release({ title: "BBB static follower", score: 100, infoHash: "def" }),
    ]);

    let fire: ((outcome: ProbeOutcome) => void) | undefined;
    onProbeResultMock.mockImplementation(
      (handler: (outcome: ProbeOutcome) => void) => {
        fire = handler;
        return Promise.resolve(() => {});
      },
    );

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");

    // Before probing, the static order stands.
    const titles = () =>
      within(screen.getByTestId("releases"))
        .getAllByRole("button")
        .map((button) => button.textContent ?? "");
    expect(titles()[0]).toContain("AAA static leader");

    await waitFor(() => expect(fire).toBeDefined());
    // The second release turns out to be alive, with a combined score that
    // beats the first's static 900.
    fire!({
      index: 1,
      badge: "green",
      combinedScore: 5000,
      probe: {
        infoHash: "def",
        metadata: { resolved: true, durationMs: 5 },
        totalDurationMs: 5,
      },
    });

    await waitFor(() =>
      expect(titles()[0]).toContain("BBB static follower"),
    );
  });

  it("gives a reordered release its own badge, not its position's", async () => {
    // The regression this guards: if the sorted list dropped the original
    // index, a row that moved would display whatever badge belonged to the
    // slot it landed in.
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA static leader", score: 900 }),
      release({ title: "BBB static follower", score: 100, infoHash: "def" }),
    ]);

    let fire: ((outcome: ProbeOutcome) => void) | undefined;
    onProbeResultMock.mockImplementation(
      (handler: (outcome: ProbeOutcome) => void) => {
        fire = handler;
        return Promise.resolve(() => {});
      },
    );

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");
    await waitFor(() => expect(fire).toBeDefined());

    // Only the second release is probed, and it goes green.
    fire!({
      index: 1,
      badge: "green",
      combinedScore: 5000,
      probe: {
        infoHash: "def",
        metadata: { resolved: true, durationMs: 5 },
        totalDurationMs: 5,
      },
    });

    await waitFor(() => {
      const rows = within(screen.getByTestId("releases")).getAllByRole("listitem");
      // The green release is first, and the badge beside it is green -- not
      // the first release's unprobed badge.
      expect(rows[0].textContent).toContain("BBB static follower");
      expect(within(rows[0]).getByTestId("release-badge")).toHaveAttribute(
        "data-badge",
        "green",
      );
      expect(within(rows[1]).getByTestId("release-badge")).toHaveAttribute(
        "data-badge",
        "pending",
      );
    });
  });

  it("offers a chip only for resolutions the search returned", async () => {
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA 1080p", resolution: "1080p" }),
      release({ title: "BBB 720p", resolution: "720p", infoHash: "b" }),
      release({ title: "CCC 720p", resolution: "720p", infoHash: "c" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const filter = await screen.findByTestId("resolution-filter");
    const chips = within(filter).getAllByRole("button");

    // Two distinct resolutions, so exactly two chips -- never the full range.
    expect(chips).toHaveLength(2);
    expect(chips[0]).toHaveAttribute("data-resolution", "1080p");
    expect(chips[1]).toHaveAttribute("data-resolution", "720p");
  });

  it("hides releases whose resolution is not selected", async () => {
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA 1080p release", resolution: "1080p" }),
      release({ title: "BBB 720p release", resolution: "720p", infoHash: "b" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const filter = await screen.findByTestId("resolution-filter");
    await fireEvent.click(within(filter).getByRole("button", { name: /720p/ }));

    await waitFor(() => {
      const rows = within(screen.getByTestId("releases")).getAllByRole("listitem");
      expect(rows).toHaveLength(1);
      expect(rows[0].textContent).toContain("BBB 720p release");
    });
  });

  it("toggles a chip back off to restore every release", async () => {
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA 1080p release", resolution: "1080p" }),
      release({ title: "BBB 720p release", resolution: "720p", infoHash: "b" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const filter = await screen.findByTestId("resolution-filter");
    const chip = () => within(filter).getByRole("button", { name: /720p/ });

    await fireEvent.click(chip());
    await waitFor(() =>
      expect(
        within(screen.getByTestId("releases")).getAllByRole("listitem"),
      ).toHaveLength(1),
    );

    await fireEvent.click(chip());
    await waitFor(() =>
      expect(
        within(screen.getByTestId("releases")).getAllByRole("listitem"),
      ).toHaveLength(2),
    );
  });

  it("keeps each surviving release's own badge when filtering", async () => {
    // The regression this guards: filtering must not renumber, or a surviving
    // row would show the badge of whatever release used to sit at that index.
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA 1080p", resolution: "1080p", infoHash: "aaa" }),
      release({ title: "BBB 720p", resolution: "720p", infoHash: "bbb" }),
    ]);

    let fire: ((outcome: ProbeOutcome) => void) | undefined;
    onProbeResultMock.mockImplementation(
      (handler: (outcome: ProbeOutcome) => void) => {
        fire = handler;
        return Promise.resolve(() => {});
      },
    );

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");
    await waitFor(() => expect(fire).toBeDefined());

    // Only the second release (720p) is probed, and it goes green.
    fire!({
      index: 1,
      badge: "green",
      combinedScore: 500,
      probe: {
        infoHash: "bbb",
        metadata: { resolved: true, durationMs: 5 },
        totalDurationMs: 5,
      },
    });

    // Filtering to 720p leaves only the probed release.
    const filter = screen.getByTestId("resolution-filter");
    await fireEvent.click(within(filter).getByRole("button", { name: /720p/ }));

    await waitFor(() => {
      const rows = within(screen.getByTestId("releases")).getAllByRole("listitem");
      expect(rows).toHaveLength(1);
      expect(rows[0].textContent).toContain("BBB 720p");
      // Its own green badge, not release 0's pending one.
      expect(within(rows[0]).getByTestId("release-badge")).toHaveAttribute(
        "data-badge",
        "green",
      );
    });
  });

  it("clears the filter when the search re-runs", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        streamingEpisodes: [
          { url: "https://x.test/1", title: "Episode 1" },
          { url: "https://x.test/2", title: "Episode 2" },
        ],
      }),
    );
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA 1080p", resolution: "1080p" }),
      release({ title: "BBB 720p", resolution: "720p", infoHash: "b" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const filter = await screen.findByTestId("resolution-filter");
    await fireEvent.click(within(filter).getByRole("button", { name: /720p/ }));

    // Switching episode re-runs the search. The old selection may not exist in
    // the new results, so it must not survive and silently empty the list.
    const list = await screen.findByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[1]);

    await waitFor(() => {
      const chips = within(screen.getByTestId("resolution-filter")).getAllByRole(
        "button",
      );
      expect(chips.every((chip) => chip.getAttribute("aria-pressed") === "false")).toBe(
        true,
      );
    });
  });

  it("wraps the torrent file list in a scrollable container", async () => {
    // The file list has no bound of its own: a torrent can hold hundreds of
    // files, and without this the page grows to fit all of them.
    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await fireEvent.click(screen.getByRole("button", { name: /load torrent/i }));

    const scroller = await screen.findByTestId("file-scroller");
    expect(scroller.className).toContain("overflow-y-auto");

    // The list it wraps is still the one the reader interacts with.
    expect(within(scroller).getByTestId("torrent-files")).toBeInTheDocument();
  });
});
