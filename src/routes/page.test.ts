import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

// Every command the page reaches for is stubbed, so the suite never touches
// the backend. `errorMessage` stays real so failures render as they would in
// the app.
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

  it("loads the hero from trending, not the first shelf query", () => {
    render(Page);

    // The carousel wants a handful of strong titles.
    expect(getListMock).toHaveBeenCalledWith("trending", 5);
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
    getTrendingMock.mockRejectedValue("provider returned HTTP 429");
    getListMock.mockResolvedValue([anime(1, "Popular")]);

    render(Page);

    expect(
      screen.getByRole("heading", { name: "Most popular" }),
    ).toBeInTheDocument();
    expect(await screen.findByText(/429/)).toBeInTheDocument();
    // A sibling shelf still produced content.
    expect(await screen.findAllByText("Popular")).not.toHaveLength(0);
  });

  it("omits the hero when nothing comes back for it", () => {
    getListMock.mockResolvedValue([]);

    render(Page);

    expect(screen.queryByRole("region", { name: /carousel/i })).toBeNull();
  });

  it("omits the genre grid when no genres come back", () => {
    getGenresMock.mockResolvedValue([]);

    render(Page);

    expect(screen.queryByRole("heading", { name: /browse by genre/i })).toBeNull();
  });

  it("omits the schedule when nothing is airing", () => {
    getScheduleMock.mockResolvedValue([]);

    render(Page);

    expect(screen.queryByRole("heading", { name: /airing soon/i })).toBeNull();
  });
});