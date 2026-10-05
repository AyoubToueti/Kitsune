import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/svelte";

import type { Anime, EpisodeInfo } from "$lib/types";
import { clearNavigations, navigations } from "../../test/app-navigation-stub";

// Aliased to src/test/app-state-stub.ts in vitest.config.js. Each test sets
// the params (and URL) before rendering.
import { page as appState } from "$app/state";

const getAnimeMock = vi.hoisted(() => vi.fn());
const getEpisodesMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getAnime: getAnimeMock, getEpisodes: getEpisodesMock };
});

// The watch modal is always mounted, so its session subscribes to player
// events on every render. Mocked so `onPlayerExit` does not reach the real
// `listen`, which needs a Tauri runtime the unit suite does not have.
vi.mock("$lib/api/player", () => ({
  onPlayerExit: vi.fn(async () => () => {}),
  removeTorrent: vi.fn(async () => {}),
}));

// The now-playing store reads settings at load and on start.
vi.mock("$lib/api/settings", () => ({
  getSettings: vi.fn(async () => ({
    player: "mpv",
    playerArgs: [],
    downloadDir: null,
    preferredResolutions: ["1080p"],
    minSeeders: 0,
    readyFraction: 0.05,
    askEveryTime: true,
    theme: "system",
  })),
  setSettings: vi.fn(),
  onSettingsChanged: vi.fn(async () => () => {}),
}));

import Page from "./[id]/+page.svelte";
import { requestOpen, startPlaying } from "$lib/now-playing.svelte";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: ["Action", "Adventure"],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

/** A promise that never settles, to hold the page in its loading state. */
function pending(): Promise<Anime | null> {
  return new Promise(() => {});
}

/** A Jikan catalogue entry, which is where the episode list comes from now. */
function info(number: number, title?: string): EpisodeInfo {
  return { number, title, filler: false, recap: false };
}

/**
 * A work whose catalogue lists `count` episodes.
 *
 * The episode grid is built from the catalogue, not from AniList's streaming
 * links, so a test that expects rows has to say what Jikan returns.
 */
function withCatalogue(count: number, overrides: Partial<Anime> = {}): Anime {
  getEpisodesMock.mockResolvedValue(
    Array.from({ length: count }, (_, i) => info(i + 1, `Episode ${i + 1}`)),
  );
  return anime({ idMal: 21, episodeCount: count, ...overrides });
}

/**
 * Point the stub at a route param. Same cast workaround as the search page
 * test: svelte-check resolves `$app/state` to SvelteKit's real types which
 * narrow `params` to a route-specific record.
 */
function setId(id: string | number) {
  (appState as { url: URL; params: Record<string, string> }).url = new URL(
    `http://localhost/anime/${id}`,
  );
  (appState as { url: URL; params: Record<string, string> }).params = {
    id: String(id),
  };
}

beforeEach(() => {
  getAnimeMock.mockReset();
  // Default: no catalogue, so the page falls back to a synthesised list and
  // tests that do not care about episodes are unaffected.
  getEpisodesMock.mockReset().mockResolvedValue([]);
  clearNavigations();
  setId(21);
});

describe("detail page", () => {
  it("shows a loading state while the request is in flight", () => {
    getAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("passes the numeric id to getAnime", () => {
    setId(42);
    getAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(getAnimeMock).toHaveBeenCalledWith(42);
  });

  it("renders the title and metadata when loaded", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        format: "TV",
        episodeCount: 24,
        durationMinutes: 23,
        status: "FINISHED",
        seasonYear: 2023,
        averageScore: 85,
      }),
    );

    render(Page);

    expect(
      await screen.findByRole("heading", { name: "One Piece" }),
    ).toBeInTheDocument();
    expect(screen.getByText("TV")).toBeInTheDocument();
    expect(screen.getByText("24 eps")).toBeInTheDocument();
    expect(screen.getByText("23 min")).toBeInTheDocument();
    expect(screen.getByText("FINISHED")).toBeInTheDocument();
    expect(screen.getByText("2023")).toBeInTheDocument();
    expect(screen.getByText("85")).toBeInTheDocument();
  });

  it("shows not found when getAnime returns null", async () => {
    getAnimeMock.mockResolvedValue(null);

    render(Page);

    expect(await screen.findByText(/not found/i)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    getAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  // Regression: AniList returns no streaming links for many titles (e.g. Re:ZERO
  // season 4), and the backend once omitted the key entirely for an empty list.
  // The page read `anime.streamingEpisodes.length` unguarded, threw mid-render,
  // and -- with no error boundary -- left the skeleton on screen forever. A
  // response with the array keys absent must render, not hang.
  it("renders when the backend omits empty array fields", async () => {
    const sparse = anime();
    // Simulate the keys being absent rather than empty, which is what the
    // omission produced on the wire.
    delete (sparse as Partial<Anime>).genres;
    delete (sparse as Partial<Anime>).streamingEpisodes;
    delete (sparse as Partial<Anime>).relations;
    delete (sparse as Partial<Anime>).recommendations;
    getAnimeMock.mockResolvedValue(sparse);

    render(Page);

    expect(
      await screen.findByRole("heading", { name: "One Piece" }),
    ).toBeInTheDocument();
    // No episode data at all: the page still renders rather than hanging, and
    // says so instead of showing an empty grid. The copy was shortened in the
    // detail-page redesign to "Episode info unavailable".
    expect(
      screen.getByText(/episode info unavailable/i),
    ).toBeInTheDocument();
  });

  it("synthesises a clickable episode grid when there are no streaming episodes", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        episodeCount: 12,
        streamingEpisodes: [],
        coverImage: "https://x.test/c.jpg",
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    // The synthesised grid: 12 cards inside the list. The list also has its own
    // jump-to-episode form, so the count is scoped to the cards.
    const cards = within(screen.getByTestId("episode-list")).getAllByRole("button");
    expect(cards).toHaveLength(12);
    for (const card of cards) {
      expect(card).not.toBeDisabled();
    }
  });

  it("shows episode thumbnails from the catalogue cover", async () => {
    getAnimeMock.mockResolvedValue(
      withCatalogue(2, { coverImage: "https://example.test/cover.jpg" }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    const list = screen.getByTestId("episode-list");
    expect(list.querySelectorAll("button")).toHaveLength(2);
    // The catalogue fixture titles entries "Episode N", which already announce
    // their number, so no prefix is added.
    expect(
      within(list).getByRole("button", { name: "Episode 1" }),
    ).toBeInTheDocument();
    // Jikan has no per-episode stills, so the card artwork is the work's cover.
    expect(list.querySelector("img")).toHaveAttribute(
      "src",
      "https://example.test/cover.jpg",
    );
  });

  it("fills a short catalogue for a finished work and drops the aired label", async () => {
    // The reported case: MAL has published one episode of a finished
    // twelve-episode work. The grid must still reach all twelve, and because
    // every episode has aired the "N of M aired" label would be a lie.
    getAnimeMock.mockResolvedValue(
      withCatalogue(1, { episodeCount: 12, status: "FINISHED" }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    const list = screen.getByTestId("episode-list");
    expect(list.querySelectorAll("button")).toHaveLength(12);
    expect(screen.queryByText(/of 12 aired/)).toBeNull();
  });

  it("shows how many episodes have aired when the work is still airing", async () => {
    // Four catalogue entries, but the work claims twelve and is still airing:
    // the remaining eight may not have aired, so they are not invented and the
    // header says how many actually have.
    getAnimeMock.mockResolvedValue(
      withCatalogue(4, { episodeCount: 12, status: "RELEASING" }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    expect(screen.getByText("4 of 12 aired")).toBeInTheDocument();
  });

  it("renders the related anime sidebar", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        relations: [
          {
            id: 865,
            title: { romaji: "Attack on Titan Season 2" },
            relationType: "SEQUEL",
          },
        ],
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    expect(
      screen.getByRole("heading", { name: /related anime/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /season 2/i })).toHaveAttribute(
      "href",
      "/anime/865",
    );
  });

  it("renders the recommendations row", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        recommendations: [
          {
            anime: {
              id: 16498,
              provider: "anilist",
              title: { romaji: "Fullmetal Alchemist" },
              genres: [],
              streamingEpisodes: [],
              relations: [],
              recommendations: [],
            },
            rating: 42,
          },
        ],
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    expect(
      screen.getByRole("heading", { name: /recommended/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /fullmetal/i })).toHaveAttribute(
      "href",
      "/anime/16498",
    );
  });

  it("does not render a where-to-watch section", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        streamingEpisodes: [
          {
            url: "https://crunchyroll.example/watch",
            title: "Episode 1",
            site: "Crunchyroll",
          },
        ],
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    // The section moved to the dedicated watch page, so the detail page must
    // not render it -- otherwise the same links appear in two places.
    expect(
      screen.queryByRole("heading", { name: /where to watch/i }),
    ).toBeNull();
  });

  it("does not render streaming links section when empty", async () => {
    getAnimeMock.mockResolvedValue(anime({ streamingEpisodes: [] }));

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    expect(
      screen.queryByRole("heading", { name: /where to watch/i }),
    ).not.toBeInTheDocument();
  });

  // --- watch modal wiring --------------------------------------------------

  it("opens the watch modal when an episode is clicked", async () => {
    getAnimeMock.mockResolvedValue(withCatalogue(2));

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    const list = screen.getByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[1]);

    // The modal opens in place; the reader is not navigated away.
    expect(await screen.findByTestId("modal-dialog")).toBeInTheDocument();
    expect(navigations).toHaveLength(0);
  });

  it("does not navigate or open AniList's licensed link from the grid", async () => {
    getAnimeMock.mockResolvedValue(withCatalogue(1));

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    const list = screen.getByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[0]);

    // The click opens the in-app modal, not a navigation and not an external
    // open of the provider's licensed link.
    expect(await screen.findByTestId("modal-dialog")).toBeInTheDocument();
    expect(navigations).toHaveLength(0);
  });

  it("opens the modal for the ?ep= episode on load", async () => {
    // A resume link points here with the 0-based index; the modal must open
    // without a click.
    (appState as { url: URL }).url = new URL("http://localhost/anime/21?ep=1");
    getAnimeMock.mockResolvedValue(withCatalogue(3));

    render(Page);

    expect(await screen.findByTestId("modal-dialog")).toBeInTheDocument();
  });

  it("ignores a ?ep= index past the end of the list", async () => {
    (appState as { url: URL }).url = new URL("http://localhost/anime/21?ep=99");
    getAnimeMock.mockResolvedValue(withCatalogue(2));

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    // Out of range is treated as "no request", not a phantom selection.
    expect(screen.queryByTestId("modal-dialog")).toBeNull();
  });

  it("opens the modal when the now-playing disc asks", async () => {
    // The URL carries NO `?ep=`: the disc must open the modal from a bare page,
    // which the query-param path cannot do because nothing changes.
    getAnimeMock.mockResolvedValue(withCatalogue(3));

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    // The reader is watching episode index 1 of this work elsewhere.
    const playing = [
      { number: 1, title: "Episode 1" },
      { number: 2, title: "Episode 2" },
      { number: 3, title: "Episode 3" },
    ];
    startPlaying(anime({ id: 21 }), 1, playing);
    requestOpen();

    expect(await screen.findByTestId("modal-dialog")).toBeInTheDocument();
  });
});