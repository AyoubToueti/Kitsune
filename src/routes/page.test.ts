import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

// Only `getTrending` is stubbed; `errorMessage` stays real so the test
// exercises the actual rendering of a backend failure.
const getTrendingMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getTrending: getTrendingMock };
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

/** A promise that never settles, to hold the page in its loading state. */
function pending(): Promise<Anime[]> {
  return new Promise(() => {});
}

beforeEach(() => {
  getTrendingMock.mockReset();
});

describe("home page", () => {
  it("shows a loading state while the request is in flight", () => {
    getTrendingMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("asks for a bounded number of titles", async () => {
    getTrendingMock.mockResolvedValue([anime(1, "A")]);

    render(Page);
    await screen.findByRole("heading", { name: "A" });

    expect(getTrendingMock).toHaveBeenCalledWith(24);
  });

  it("features the first title in the hero", async () => {
    getTrendingMock.mockResolvedValue([anime(1, "First"), anime(2, "Second")]);

    render(Page);

    expect(
      await screen.findByRole("heading", { name: "First", level: 1 }),
    ).toBeInTheDocument();
  });

  it("shows the remaining titles in the row", async () => {
    getTrendingMock.mockResolvedValue([anime(1, "First"), anime(2, "Second")]);

    render(Page);
    await screen.findByRole("heading", { name: "First", level: 1 });

    expect(screen.getByText("Second")).toBeInTheDocument();
    // The featured title is not repeated in the row.
    expect(screen.queryAllByText("First")).toHaveLength(1);
  });

  it("omits the row when only one title came back", async () => {
    getTrendingMock.mockResolvedValue([anime(1, "Only")]);

    render(Page);
    await screen.findByRole("heading", { name: "Only", level: 1 });

    // PosterRow renders nothing for an empty list.
    expect(
      screen.queryByRole("heading", { name: "Trending now" }),
    ).not.toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    getTrendingMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("offers a retry that re-requests", async () => {
    getTrendingMock.mockRejectedValueOnce("provider returned HTTP 429");

    render(Page);
    await screen.findByText(/429/);

    getTrendingMock.mockResolvedValue([anime(1, "Recovered")]);
    screen.getByRole("button", { name: "Try again" }).click();

    expect(
      await screen.findByRole("heading", { name: "Recovered", level: 1 }),
    ).toBeInTheDocument();
    expect(getTrendingMock).toHaveBeenCalledTimes(2);
  });

  it("says so when there are no titles", async () => {
    getTrendingMock.mockResolvedValue([]);

    render(Page);

    expect(await screen.findByText(/no titles available/i)).toBeInTheDocument();
  });
});