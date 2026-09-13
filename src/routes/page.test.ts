import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

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

  it("requests a bounded schedule window", () => {
    render(Page);

    const [from, to] = getScheduleMock.mock.calls[0];
    expect(from).toBeLessThan(to);
    const days = (to - from) / (24 * 60 * 60);
    expect(days).toBeCloseTo(7, 0);
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

    expect(
      screen.queryByRole("heading", { name: /estimated schedule/i }),
    ).toBeNull();
  });
});