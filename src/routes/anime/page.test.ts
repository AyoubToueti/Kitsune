import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/svelte";

import type { Anime } from "$lib/types";
import { clearNavigations, navigations } from "../../test/app-navigation-stub";

// Aliased to src/test/app-state-stub.ts in vitest.config.js. Each test sets
// the params (and URL) before rendering.
import { page as appState } from "$app/state";

const getAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getAnime: getAnimeMock };
});

import Page from "./[id]/+page.svelte";

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
    // says so instead of showing an empty grid.
    expect(
      screen.getByText(/episode information unavailable/i),
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

    it("shows episode thumbnails when streaming episodes are present", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        streamingEpisodes: [
          {
            url: "https://crunchyroll.example/1",
            title: "Episode 1",
            thumbnail: "https://example.test/1.jpg",
          },
          {
            url: "https://crunchyroll.example/2",
            title: "Episode 2",
            thumbnail: "https://example.test/2.jpg",
          },
        ],
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    // Scoped to the episode list: the "Where to watch" section below renders its
    // own button carrying the same title, so an unscoped query is ambiguous.
    const list = screen.getByTestId("episode-list");
    expect(list.querySelectorAll("button")).toHaveLength(2);
    expect(
      within(list).getByRole("button", { name: "Episode 1" }),
    ).toBeInTheDocument();
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

  // --- watch page wiring ---------------------------------------------------

  it("navigates to the watch page when an episode is clicked", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        streamingEpisodes: [
          { url: "https://x.test/1", title: "Episode 1" },
          { url: "https://x.test/2", title: "Episode 2" },
        ],
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    const list = screen.getByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[1]);

    // The episode index travels in the query string so the watch page opens
    // on the entry the reader picked rather than the top of the list.
    expect(navigations).toEqual(["/watch/21?ep=1"]);
  });

  it("does not open AniList's licensed link from the grid", async () => {
    getAnimeMock.mockResolvedValue(
      anime({ streamingEpisodes: [{ url: "https://x.test/1", title: "Episode 1" }] }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    const list = screen.getByTestId("episode-list");
    await fireEvent.click(within(list).getAllByRole("button")[0]);

    // The click must be a navigation now, not an external open.
    expect(navigations).toHaveLength(1);
  });
});