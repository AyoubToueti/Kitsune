import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime, AnimePage } from "$lib/types";

const getTrendingMock = vi.hoisted(() => vi.fn());
const getListMock = vi.hoisted(() => vi.fn());
const getGenresMock = vi.hoisted(() => vi.fn());
const browseAnimeMock = vi.hoisted(() => vi.fn());

vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return {
    ...actual,
    getTrending: getTrendingMock,
    getList: getListMock,
    getGenres: getGenresMock,
    browseAnime: browseAnimeMock,
  };
});

import Page from "./+page.svelte";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
  };
}

function many(count: number): Anime[] {
  return Array.from({ length: count }, (_, i) => anime(i + 1, `Title ${i + 1}`));
}

/** An empty page, for the two sections that page through browse. */
    function emptyPage(): AnimePage {
      return {
        items: [],
        pageInfo: {
          total: 0,
          currentPage: 1,
          lastPage: 1,
          hasNextPage: false,
        },
      };
    }

    beforeEach(() => {
      getTrendingMock.mockReset().mockResolvedValue([]);
      getListMock.mockReset().mockResolvedValue([]);
      getGenresMock.mockReset().mockResolvedValue([]);
      browseAnimeMock.mockReset().mockResolvedValue(emptyPage());
});

describe("home page", () => {
  it("renders the four list block headings", () => {
    render(Page);

    for (const title of [
      "Top airing",
      "Most popular",
      "Top rated",
      "Latest completed",
    ]) {
      expect(screen.getByRole("heading", { name: title })).toBeInTheDocument();
    }
  });

  it("requests each block from its own list filter", () => {
    render(Page);

    const filters = getListMock.mock.calls.map((call) => call[0]);
    expect(filters).toContain("topAiring");
    expect(filters).toContain("mostPopular");
    expect(filters).toContain("topRated");
    expect(filters).toContain("latestCompleted");
  });

  it("fetches trending once, shared by the hero and the rail", () => {
    render(Page);

    expect(getTrendingMock).toHaveBeenCalledTimes(1);
  });

  it("does not repeat the hero's titles in the rail", async () => {
    getTrendingMock.mockResolvedValue(many(20));

    render(Page);
    await screen.findByRole("heading", { name: "Title 1", level: 1 });

    // The rail starts at HERO_LIMIT, so the hero's titles appear once.
    await screen.findByRole("heading", { name: "Trending" });
    expect(screen.queryAllByText("Title 1")).toHaveLength(1);
  });

  it("renders the schedule section, which loads its own days", () => {
    render(Page);

    // The page no longer fetches the schedule: the widget owns its own
    // day-on-demand loading, so all the page must do is mount it.
    expect(
      screen.getByRole("heading", { name: /estimated schedule/i }),
    ).toBeInTheDocument();
  });

  it("still renders the other blocks when one fails", async () => {
    getListMock.mockImplementation((filter: string) =>
      filter === "mostPopular"
        ? Promise.resolve([anime(1, "Popular")])
        : Promise.reject("provider returned HTTP 429"),
    );

    render(Page);

    expect(await screen.findAllByText("Popular")).not.toHaveLength(0);
    // A sibling block surfaced its own failure without taking the page down.
    expect(await screen.findAllByText(/429/)).not.toHaveLength(0);
  });

  it("omits the hero when trending comes back empty", () => {
    getTrendingMock.mockResolvedValue([]);

    render(Page);

    expect(screen.queryByRole("region", { name: /featured/i })).toBeNull();
  });

  it("omits the trending rail when there is nothing beyond the hero", async () => {
    getTrendingMock.mockResolvedValue(many(5));

    render(Page);
    await screen.findByRole("heading", { name: "Title 1", level: 1 });

    // Exactly HERO_LIMIT came back, so the rail has nothing left to rank.
    expect(screen.queryByRole("heading", { name: "Trending" })).toBeNull();
  });

  it("renders the upcoming-next-season section", () => {
    render(Page);

    expect(
      screen.getByRole("heading", { name: /upcoming next season/i }),
    ).toBeInTheDocument();
  });

  /// The season section asks for unreleased titles specifically: a season
  /// includes titles that already aired, so without the status filter
  /// "upcoming" would show the past.
  it("asks for unreleased titles in the next season", () => {
    render(Page);

    const query = browseAnimeMock.mock.calls.find(
      (call) => call[0]?.status === "notYetReleased",
    )?.[0];

    expect(query).toBeDefined();
    expect(query.season).toBeDefined();
  });

  it("renders the Top 100 section", () => {
    render(Page);

    expect(
      screen.getByRole("heading", { name: /top 100 anime/i }),
    ).toBeInTheDocument();
  });

  /// The preview and the full list must rank the same way, or "View All" would
  /// show a different ordering than the rows it was reached from.
  it("asks for the top list in score order", () => {
    render(Page);

    const query = browseAnimeMock.mock.calls.find(
      (call) => call[0]?.sort === "score",
    )?.[0];

    expect(query).toBeDefined();
  });

  it("offers a View All link to the full ranking", () => {
    render(Page);

    // Scoped by href: the season section has its own "View all" link, so the
    // accessible name alone is ambiguous.
    const links = screen
      .getAllByRole("link")
      .filter((link) => link.getAttribute("href") === "/top");

    expect(links).toHaveLength(1);
  });

  /// Each section owns its own failure, so one rate-limited request must not
  /// blank the rest of the page. The list blocks use a different command, so
  /// they must still render their data while the two paged sections error.
  it("keeps the page up when the extra sections fail", async () => {
    getListMock.mockResolvedValue([anime(1, "Block Title")]);
    browseAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findAllByText(/429/)).not.toHaveLength(0);
    // The block headings survive, and so does their content.
    expect(
      screen.getByRole("heading", { name: "Top airing" }),
    ).toBeInTheDocument();
    expect(screen.getAllByText("Block Title")).not.toHaveLength(0);
  });

  it("omits the genre grid when no genres come back", () => {
    getGenresMock.mockResolvedValue([]);

    render(Page);

    expect(
      screen.queryByRole("heading", { name: /browse by genre/i }),
    ).toBeNull();
  });

  it("mounts the schedule without the page passing it any data", () => {
    render(Page);

    // A day strip renders from date arithmetic alone, so the section is
    // present before any schedule request resolves.
    expect(screen.getByTestId("day-strip")).toBeInTheDocument();
  });
});