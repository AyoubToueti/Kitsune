import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within, waitFor } from "@testing-library/svelte";

import type {
  Anime,
  EpisodeInfo,
  ProbeOutcome,
  Release,
  TorrentHandle,
} from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js.
import { page as appState } from "$app/state";

const getAnimeMock = vi.hoisted(() => vi.fn());
const getEpisodesMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getAnime: getAnimeMock, getEpisodes: getEpisodesMock };
});

const openDialogMock = vi.hoisted(() => vi.fn());
const saveDialogMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: openDialogMock,
  save: saveDialogMock,
}));

const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

const addTorrentMock = vi.hoisted(() => vi.fn());
const addMagnetMock = vi.hoisted(() => vi.fn());
const removeTorrentMock = vi.hoisted(() => vi.fn());
const getStreamUrlMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/player", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/player")>("$lib/api/player");
  return {
    ...actual,
    addTorrent: addTorrentMock,
    addMagnet: addMagnetMock,
    // Mocked because @testing-library/svelte unmounts the page after every
    // test, and teardown calls this. Left real it would reach `invoke` with no
    // Tauri runtime behind it.
    removeTorrent: removeTorrentMock,
    getStreamUrl: getStreamUrlMock,
    // The sidebar button fetches these; stub them so the page renders.
    getPlayer: vi.fn().mockResolvedValue("mpv"),
    suggestedPlayers: vi.fn().mockResolvedValue(["mpv"]),
  };
});

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

/** A Jikan catalogue entry, the shape the watch page now lists episodes from. */
function episodeInfo(number: number): EpisodeInfo {
  return {
    number,
    title: `Episode ${number}`,
    aired: "2024-10-02T00:00:00+00:00",
    filler: false,
    recap: false,
  };
}

/**
 * An anime with a MAL link and `count` catalogued episodes.
 *
 * The watch page lists episodes from the Jikan catalogue now, not from AniList's
 * streaming links, so a test that wants an episode list has to say what the
 * catalogue returns.
 */
function animeWithEpisodes(count: number, overrides: Partial<Anime> = {}): Anime {
  getEpisodesMock.mockResolvedValue(
    Array.from({ length: count }, (_, i) => episodeInfo(i + 1)),
  );
  return anime({ idMal: 21, episodeCount: count, ...overrides });
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
  // Default: an empty catalogue, so the page falls back to a synthesised list and
  // tests that do not care about episodes are unaffected.
  getEpisodesMock.mockReset().mockResolvedValue([]);
  openDialogMock.mockReset().mockResolvedValue("/tmp/show.torrent");
  saveDialogMock.mockReset().mockResolvedValue("/tmp/show.torrent");
  openUrlMock.mockReset().mockResolvedValue(undefined);
  downloadTorrentMock.mockReset().mockResolvedValue(undefined);
  addTorrentMock.mockReset().mockResolvedValue(handle());
  addMagnetMock.mockReset().mockResolvedValue(handle());
  removeTorrentMock.mockReset().mockResolvedValue(undefined);
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
    expect(searchReleasesMock).toHaveBeenCalledWith(
      ["Attack on Titan"],
      undefined,
      undefined,
    );
  });

  it("searches every title form the work has", async () => {
    // An uploader may have used the English or the romaji title, so both must be
    // sent. The backend expands each into its own query spellings.
    getAnimeMock.mockResolvedValue(
      anime({
        title: {
          english: "Attack on Titan",
          romaji: "Shingeki no Kyojin",
        },
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    expect(searchReleasesMock).toHaveBeenCalledWith(
      ["Attack on Titan", "Shingeki no Kyojin"],
      undefined,
      undefined,
    );
  });

  it("plays a release the app found on its own", async () => {
    searchReleasesMock.mockResolvedValue([release()]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    // Scoped to the row's play button: each row also has a magnet button, so an
    // unscoped getByRole would match two and throw.
    await fireEvent.click(within(list).getByTestId("release-play"));

    expect(addMagnetMock).toHaveBeenCalledWith("magnet:?xt=urn:btih:abc");
    // No episode selected, so it falls back to the first playable file.
    expect(getStreamUrlMock).toHaveBeenCalledWith(5, 0);
  });

  it("releases the torrent when the page is left", async () => {
    searchReleasesMock.mockResolvedValue([release()]);

    const { unmount } = render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByTestId("release-play"));

    // Live while the page is up: nothing has released it yet.
    expect(removeTorrentMock).not.toHaveBeenCalled();

    unmount();

    // Without this the torrent keeps downloading for the life of the process,
    // which is the leak: the session is created once and never torn down.
    expect(removeTorrentMock).toHaveBeenCalledWith(5);
  });

  it("releases the previous torrent when another release is played", async () => {
    searchReleasesMock.mockResolvedValue([
      release(),
      release({ title: "[Group] Show - 02 [1080p]", infoHash: "def" }),
    ]);
    // The second add resolves to a different torrent, as a real one would.
    addMagnetMock
      .mockResolvedValueOnce(handle())
      .mockResolvedValueOnce({ id: 6, files: handle().files });

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getAllByTestId("release-play")[0]);

    // Wait for the first torrent to be live, or the second click would be
    // ignored: `playRelease` returns early while another add is in flight.
    await waitFor(() => expect(getStreamUrlMock).toHaveBeenCalled());

    await fireEvent.click(
      within(screen.getByTestId("releases")).getAllByTestId("release-play")[1],
    );

    await waitFor(() => expect(removeTorrentMock).toHaveBeenCalledWith(5));
  });

  it("removes a torrent that resolves after the page is left", async () => {
    searchReleasesMock.mockResolvedValue([release()]);

    // Hold the add open, so the page can be torn down mid-flight. This is the
    // case the unmount effect cannot catch: `torrentId` is still null when the
    // component dies, so the handle it later returns would leak.
    let settle!: (value: TorrentHandle) => void;
    addMagnetMock.mockReturnValue(
      new Promise<TorrentHandle>((resolve) => {
        settle = resolve;
      }),
    );

    const { unmount } = render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByTestId("release-play"));

    unmount();
    settle(handle());

    await waitFor(() => expect(removeTorrentMock).toHaveBeenCalledWith(5));
  });

  it("hands a release's magnet to the OS", async () => {
    openUrlMock.mockResolvedValue(undefined);
    searchReleasesMock.mockResolvedValue([release()]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByTestId("release-magnet"));

    expect(openUrlMock).toHaveBeenCalledWith("magnet:?xt=urn:btih:abc");
    // Delegating to the OS must not also add the torrent to the app's own
    // engine: that is what the row's play button is for.
    expect(addMagnetMock).not.toHaveBeenCalled();
  });

  it("says so when no torrent client handles the magnet", async () => {
    openUrlMock.mockRejectedValue("no application registered for magnet");
    searchReleasesMock.mockResolvedValue([release()]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByTestId("release-magnet"));

    expect(await screen.findByText(/could not open a torrent client/i)).toBeInTheDocument();
  });

  it("saves a release's torrent file to the chosen path", async () => {
    saveDialogMock.mockResolvedValue("/home/u/Show - 01.torrent");
    downloadTorrentMock.mockResolvedValue(undefined);
    searchReleasesMock.mockResolvedValue([
      release({ torrentUrl: "https://nyaa.si/download/1.torrent" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByTestId("release-download"));

    expect(downloadTorrentMock).toHaveBeenCalledWith(
      "https://nyaa.si/download/1.torrent",
      "/home/u/Show - 01.torrent",
    );
  });

  it("does not fetch anything when the save dialog is cancelled", async () => {
    saveDialogMock.mockResolvedValue(null);
    searchReleasesMock.mockResolvedValue([
      release({ torrentUrl: "https://nyaa.si/download/1.torrent" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByTestId("release-download"));

    // A cancelled picker is not an error and must not start a download.
    expect(downloadTorrentMock).not.toHaveBeenCalled();
  });

  it("disables the download button when the release has no torrent url", async () => {
    searchReleasesMock.mockResolvedValue([release()]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    // Not every feed carries the direct link, so there is nothing to fetch.
    expect(within(list).getByTestId("release-download")).toBeDisabled();
  });

  it("says so when the torrent download fails", async () => {
    saveDialogMock.mockResolvedValue("/tmp/x.torrent");
    downloadTorrentMock.mockRejectedValue("the indexer answered with status 404");
    searchReleasesMock.mockResolvedValue([
      release({ torrentUrl: "https://nyaa.si/download/1.torrent" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("releases");
    await fireEvent.click(within(list).getByTestId("release-download"));

    expect(
      await screen.findByText(/could not download the torrent/i),
    ).toBeInTheDocument();
  });

  it("narrows the search to the selected episode", async () => {
        getAnimeMock.mockResolvedValue(animeWithEpisodes(2));
    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const list = await screen.findByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[1]);

    expect(searchReleasesMock).toHaveBeenLastCalledWith(
      ["Attack on Titan"],
      2,
      undefined,
    );
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
        getAnimeMock.mockResolvedValue(animeWithEpisodes(2));
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
    getAnimeMock.mockResolvedValue(animeWithEpisodes(2));
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

  it("wraps the releases list in a scrollable container", async () => {
    // Twenty search results should not stretch the page. The chips stay
    // OUTSIDE the scroller, since they control the list and must not scroll
    // away with it.
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA", resolution: "1080p" }),
      release({ title: "BBB", resolution: "720p", infoHash: "b" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const scroller = await screen.findByTestId("release-scroller");
    expect(scroller.className).toContain("overflow-y-auto");
    expect(within(scroller).getByTestId("releases")).toBeInTheDocument();

    // The filter is a sibling of the scroller, not a child.
    expect(within(scroller).queryByTestId("resolution-filter")).toBeNull();
  });

  it("narrows the releases to a typed query", async () => {
    searchReleasesMock.mockResolvedValue([
      release({ title: "[SubsPlease] Show - 01 [1080p]", infoHash: "aaa" }),
      release({ title: "[Erai-raws] Show - 01 [720p]", infoHash: "bbb" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");

    await fireEvent.input(screen.getByTestId("release-filter"), {
      target: { value: "erai" },
    });

    await waitFor(() => {
      const rows = within(screen.getByTestId("releases")).getAllByRole("listitem");
      expect(rows).toHaveLength(1);
      expect(rows[0].textContent).toContain("[Erai-raws]");
    });
  });

  it("restores every release when the query is cleared", async () => {
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA 1080p", infoHash: "aaa" }),
      release({ title: "BBB 720p", infoHash: "bbb" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");

    const box = screen.getByTestId("release-filter");
    await fireEvent.input(box, { target: { value: "AAA" } });
    await waitFor(() =>
      expect(
        within(screen.getByTestId("releases")).getAllByRole("listitem"),
      ).toHaveLength(1),
    );

    await fireEvent.input(box, { target: { value: "" } });
    await waitFor(() =>
      expect(
        within(screen.getByTestId("releases")).getAllByRole("listitem"),
      ).toHaveLength(2),
    );
  });

  it("says so when the query matches nothing", async () => {
    // An empty scroller would read as a failure rather than a filter.
    searchReleasesMock.mockResolvedValue([release({ title: "AAA 1080p" })]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");

    await fireEvent.input(screen.getByTestId("release-filter"), {
      target: { value: "nothing-like-this" },
    });

    expect(await screen.findByTestId("releases-no-match")).toBeInTheDocument();
  });

  it("keeps the filter controls reachable when a query empties the list", async () => {
    // The box and the chips must not vanish with the rows: they are the way
    // back, and the query is still in the box waiting to be cleared.
    searchReleasesMock.mockResolvedValue([
      release({ title: "AAA 1080p", resolution: "1080p" }),
      release({ title: "BBB 720p", resolution: "720p", infoHash: "b" }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");

    await fireEvent.input(screen.getByTestId("release-filter"), {
      target: { value: "nothing-like-this" },
    });

    expect(await screen.findByTestId("releases-no-match")).toBeInTheDocument();
    expect(screen.getByTestId("release-filter")).toBeInTheDocument();
    expect(screen.getByTestId("resolution-filter")).toBeInTheDocument();
  });

  it("offers the text filter even when there is only one resolution", async () => {
    // ResolutionFilter hides itself when there is nothing to choose between,
    // but a text filter is still useful then -- so it must not be nested inside
    // that guard.
    searchReleasesMock.mockResolvedValue([release({ title: "AAA 1080p" })]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });
    await screen.findByTestId("releases");

    expect(screen.queryByTestId("resolution-filter")).toBeNull();
    expect(screen.getByTestId("release-filter")).toBeInTheDocument();
  });

  it("marks a release that names its season and episode", async () => {
    // `S03E09` cannot be mistaken for another season's episode 9, so it is the
    // row worth drawing the eye to.
    searchReleasesMock.mockResolvedValue([
      release({
        title: "[SubsPlease] Show S03E09 [1080p]",
        parsed: { title: "Show", season: 3, episode: 9, absoluteEpisode: 9 },
      }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const row = (await screen.findAllByRole("listitem"))[0];
    expect(row).toHaveAttribute("data-clarity", "stated");
    expect(row.textContent).toContain("SxxExx");
  });

  it("does not mark a release that omits the season", async () => {
    // `Show - 09` is a genuine match but ambiguous about which season it is, so
    // it stays unmarked rather than getting equal billing.
    searchReleasesMock.mockResolvedValue([
      release({
        title: "[SubsPlease] Show - 09 [1080p]",
        parsed: { title: "Show", episode: 9, absoluteEpisode: 9 },
      }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const row = (await screen.findAllByRole("listitem"))[0];
    expect(row).toHaveAttribute("data-clarity", "episode");
    expect(row.textContent).not.toContain("SxxExx");
  });

  it("labels a release whose name carries no episode number", async () => {
    searchReleasesMock.mockResolvedValue([
      release({
        title: "[Group] Show Batch [1080p]",
        parsed: { title: "Show" },
      }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const row = (await screen.findAllByRole("listitem"))[0];
    expect(row).toHaveAttribute("data-clarity", "unclear");
    expect(row.textContent).toContain("No episode");
  });

  it("carries the clarity on every row and explains it on hover", async () => {
    // The emphasis must not rely on colour alone, so each row exposes the level
    // as data and says what it means in words.
    searchReleasesMock.mockResolvedValue([
      release({
        title: "[G] Show S03E09 [1080p]",
        parsed: { title: "Show", season: 3, episode: 9 },
      }),
      release({
        title: "[G] Show - 10 [1080p]",
        infoHash: "b",
        parsed: { title: "Show", episode: 10, absoluteEpisode: 10 },
      }),
    ]);

    render(Page);
    await screen.findByRole("heading", { name: /attack on titan/i });

    const rows = await screen.findAllByRole("listitem");
    expect(rows).toHaveLength(2);
    for (const row of rows) {
      expect(row).toHaveAttribute("data-clarity");
      expect(row.getAttribute("title")).toMatch(/episode/i);
    }
    expect(rows[0]).toHaveAttribute("data-clarity", "stated");
    expect(rows[1]).toHaveAttribute("data-clarity", "episode");
  });
});
