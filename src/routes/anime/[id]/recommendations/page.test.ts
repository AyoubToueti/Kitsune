import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime, RecommendedAnime, RecommendationsPage } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js.
import { page as appState } from "$app/state";

const getAnimeMock = vi.hoisted(() => vi.fn());
const getRecommendationsMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return {
    ...actual,
    getAnime: getAnimeMock,
    getRecommendations: getRecommendationsMock,
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

function recommendation(id: number, title: string, rating = 10): RecommendedAnime {
  return { anime: anime(id, title), rating };
}

function page(
  items: RecommendedAnime[],
  overrides: Partial<RecommendationsPage["pageInfo"]> = {},
): RecommendationsPage {
  return {
    items,
    pageInfo: {
      total: items.length,
      currentPage: 1,
      lastPage: 1,
      hasNextPage: false,
      ...overrides,
    },
  };
}

function pending(): Promise<RecommendationsPage> {
  return new Promise(() => {});
}

/** jsdom has no IntersectionObserver; this drives the load-more path directly. */
const observerCallbacks: Array<(entries: { isIntersecting: boolean }[]) => void> = [];

function installObserver() {
  (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver = class {
    constructor(cb: (entries: { isIntersecting: boolean }[]) => void) {
      observerCallbacks.push(cb);
    }
    observe() {}
    disconnect() {}
  };
}

function scrollToBottom() {
  for (const cb of observerCallbacks) cb([{ isIntersecting: true }]);
}

beforeEach(() => {
  (appState as { params: Record<string, string> }).params = { id: "42" };
  getAnimeMock.mockReset().mockResolvedValue(anime(42, "Attack on Titan"));
  getRecommendationsMock.mockReset().mockResolvedValue(page([]));
  observerCallbacks.length = 0;
});

afterEach(() => {
  delete (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver;
});

describe("recommendations page", () => {
  it("loads recommendations for the route id", () => {
    render(Page);

    expect(getRecommendationsMock).toHaveBeenCalledWith(42, 1, 24);
  });

  it("shows a loading state while the first page is in flight", () => {
    getRecommendationsMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByTestId("grid-skeleton")).toBeInTheDocument();
  });

  it("renders a card per recommendation", async () => {
    getRecommendationsMock.mockResolvedValue(
      page([recommendation(1, "Alpha"), recommendation(2, "Beta")]),
    );

    render(Page);

    expect(await screen.findByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("shows each recommendation's upvote count", async () => {
    getRecommendationsMock.mockResolvedValue(
      page([recommendation(1, "Alpha", 77)]),
    );

    render(Page);

    expect(await screen.findByTestId("recommendation-rating")).toHaveTextContent(
      "77",
    );
  });

  it("appends the next page when the sentinel intersects", async () => {
    installObserver();
    getRecommendationsMock
      .mockResolvedValueOnce(
        page([recommendation(1, "Alpha")], { currentPage: 1, lastPage: 2, hasNextPage: true }),
      )
      .mockResolvedValueOnce(
        page([recommendation(2, "Beta")], { currentPage: 2, lastPage: 2 }),
      );

    render(Page);
    await screen.findByText("Alpha");

    scrollToBottom();

    expect(await screen.findByText("Beta")).toBeInTheDocument();
    // The first page is still shown: the list grows rather than replacing.
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(getRecommendationsMock).toHaveBeenCalledWith(42, 2, 24);
  });

  it("says so when a title has no recommendations", async () => {
    getRecommendationsMock.mockResolvedValue(page([]));

    render(Page);

    expect(
      await screen.findByText(/no recommendations/i),
    ).toBeInTheDocument();
  });

  it("offers voting on the full page", async () => {
    // The full page opts into voting and passes the base work id; the inline
    // detail row does not.
    getRecommendationsMock.mockResolvedValue(
      page([recommendation(1, "Alpha")]),
    );

    render(Page);

    expect(await screen.findByTestId("vote-up")).toBeInTheDocument();
    expect(screen.getByTestId("vote-down")).toBeInTheDocument();
  });
});