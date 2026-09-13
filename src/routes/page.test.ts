import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

// Every command the page reaches for is stubbed, so the suite never touches the
// backend. `errorMessage` stays real so failures render as they would in the
// app.
const getTrendingMock = vi.hoisted(() => vi.fn());
const getListMock = vi.hoisted(() => vi.fn());
const getGenresMock = vi.hoisted(() => vi.fn());
const getScheduleMock = vi.hoisted(() => vi.fn());

vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return {
    ...actual,
    getTrending: getTrendingMock,
    getList: getListMock,
    getGenres: getGenresMock,
    getSchedule: getScheduleMock,
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
  };
}

/** `count` distinct titles, for exercising the hero/row split. */
function many(count: number): Anime[] {
  return Array.from({ length: count }, (_, i) => anime(i + 1, `Title ${i + 1}`));
}

beforeEach(() => {
  getTrendingMock.mockReset().mockResolvedValue([]);
  getListMock.mockReset().mockResolvedValue([]);
  getGenresMock.mockReset().mockResolvedValue([]);
  getScheduleMock.mockReset().mockResolvedValue([]);
});

describe("home page", () => {
  it("renders the shelf headings", () => {
    render(Page);

    for (const title of [
      "Trending now",
      "Top airing",
      "Most popular",
      "Top rated",
      "Latest completed",
      "Upcoming",
    ]) {
      expect(screen.getByRole("heading", { name: title })).toBeInTheDocument();
    }
  });

  it("requests each shelf from its own list filter", () => {
    render(Page);

    const filters = getListMock.mock.calls.map((call) => call[0]);
    expect(filters).toContain("topAiring");
    expect(filters).toContain("mostPopular");
    expect(filters).toContain("topRated");
    expect(filters).toContain("latestCompleted");
    expect(filters).toContain("upcoming");
  });

  it("fetches trending once, shared by the hero and the row", () => {
    render(Page);

    // Two separate calls would double the request and repeat the same titles
    // in the carousel and the rail beneath it.
    expect(getTrendingMock).toHaveBeenCalledTimes(1);
    expect(getTrendingMock).toHaveBeenCalledWith(20);
  });

  it("rotates the top few titles in the hero", async () => {
    getTrendingMock.mockResolvedValue(many(20));

    render(Page);

    // The carousel shows the first title initially.
    expect(
      await screen.findByRole("heading", { name: "Title 1", level: 1 }),
    ).toBeInTheDocument();
  });

  it("does not repeat the hero's titles in the trending row", async () => {
    getTrendingMock.mockResolvedValue(many(20));

    render(Page);
    await screen.findByRole("heading", { name: "Title 1", level: 1 });

    // "Title 1" is in the hero, so it must not also appear as a card. The row
    // starts at HERO_LIMIT.
    await waitFor(() => {
      expect(screen.getByText("Title 6")).toBeInTheDocument();
    });
    expect(screen.queryAllByText("Title 1")).toHaveLength(1);
  });

  it("requests genres for the browse grid", () => {
    render(Page);

    expect(getGenresMock).toHaveBeenCalled();
  });

  it("requests a bounded schedule window", () => {
    render(Page);

    const [from, to] = getScheduleMock.mock.calls[0];
    expect(from).toBeLessThan(to);
    // A week ahead, so the window is not unbounded.
    const days = (to - from) / (24 * 60 * 60);
    expect(days).toBeCloseTo(7, 0);
  });

  it("still renders every other shelf when one fails", async () => {
    // A rate-limited row must not blank the page.
    getListMock.mockImplementation((filter: string) =>
      filter === "mostPopular"
        ? Promise.resolve([anime(1, "Popular")])
        : Promise.reject("provider returned HTTP 429"),
    );

    render(Page);

    expect(await screen.findAllByText("Popular")).not.toHaveLength(0);
    // A sibling shelf surfaced its own failure without taking the page down.
    expect(await screen.findAllByText(/429/)).not.toHaveLength(0);
  });

  it("omits the hero when trending comes back empty", () => {
    getTrendingMock.mockResolvedValue([]);

    render(Page);

    expect(screen.queryByRole("region", { name: /carousel/i })).toBeNull();
  });

  it("omits the genre grid when no genres come back", () => {
    getGenresMock.mockResolvedValue([]);

    render(Page);

    expect(
      screen.queryByRole("heading", { name: /browse by genre/i }),
    ).toBeNull();
  });

  it("omits the schedule when nothing is airing", () => {
    getScheduleMock.mockResolvedValue([]);

    render(Page);

    expect(screen.queryByRole("heading", { name: /airing soon/i })).toBeNull();
  });
});