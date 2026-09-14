import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime, AnimePage } from "$lib/types";

const browseAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, browseAnime: browseAnimeMock };
});

import SeasonSection from "./SeasonSection.svelte";

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

function page(items: Anime[]): AnimePage {
  return {
    items,
    pageInfo: {
      total: items.length,
      currentPage: 1,
      lastPage: 1,
      hasNextPage: false,
    },
  };
}

/** A promise that never settles, to hold the section in its loading state. */
function pending(): Promise<AnimePage> {
  return new Promise(() => {});
}

beforeEach(() => {
  browseAnimeMock.mockReset().mockResolvedValue(page([]));
});

describe("SeasonSection", () => {
  it("asks only for unreleased titles", async () => {
    render(SeasonSection);

    // A season includes titles that already aired, so the status filter is what
    // makes this "upcoming" rather than a season catalogue.
    const query = browseAnimeMock.mock.calls[0][0];
    expect(query.status).toBe("notYetReleased");
    expect(query.seasonYear).toBeGreaterThan(new Date().getFullYear() - 1);
  });

  it("asks for the next season, not the current one", async () => {
    render(SeasonSection);

    const query = browseAnimeMock.mock.calls[0][0];
    const now = new Date();
    const currentQuarter = Math.floor(now.getMonth() / 3);
    const seasons = ["winter", "spring", "summer", "fall"];

    expect(query.season).toBe(seasons[(currentQuarter + 1) % 4]);
  });

  it("shows a loading state first", () => {
    browseAnimeMock.mockReturnValue(pending());

    render(SeasonSection);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("renders a card per announcement", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "Kusuriya 3rd Season")]));

    render(SeasonSection);

    expect(await screen.findByText("Kusuriya 3rd Season")).toBeInTheDocument();
  });

  it("links View all to the same season it fetched", async () => {
    render(SeasonSection);
    await screen.findByText(/nothing announced/i);

    const query = browseAnimeMock.mock.calls[0][0];
    const href = screen.getByRole("link", { name: /view all/i }).getAttribute("href");

    // The link and the request must agree, or "View all" would open a
    // different season than the one previewed.
    expect(href).toContain(`season=${query.season}`);
    expect(href).toContain(`year=${query.seasonYear}`);
    expect(href).toContain("status=notYetReleased");
  });

  it("says so when nothing is announced", async () => {
    render(SeasonSection);

    expect(await screen.findByText(/nothing announced/i)).toBeInTheDocument();
  });

  it("surfaces the backend error and offers a retry", async () => {
    browseAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(SeasonSection);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /try again/i })).toBeInTheDocument();
  });
});