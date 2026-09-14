import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime, AnimePage } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js. Each test sets the
// params before rendering.
import { page as appState } from "$app/state";

const browseAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, browseAnime: browseAnimeMock };
});

// The test lives beside the route, so the import path must match the real
// relative path to the page component.
import Page from "./[name]/+page.svelte";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
  };
}

function page(
  items: Anime[],
  overrides: Partial<AnimePage["pageInfo"]> = {},
): AnimePage {
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

/** A promise that never settles, to hold the page in its loading state. */
function pending(): Promise<AnimePage> {
  return new Promise(() => {});
}

/**
 * Point the stub at a genre.
 *
 * SvelteKit decodes route params, so the component receives an already
 * readable name -- "Slice of Life", not "Slice%20of%20Life".
 */
function withGenre(name: string | undefined, page?: number) {
  const state = appState as { params: Record<string, string>; url: URL };
  state.params = name === undefined ? {} : { name };
  const suffix = page === undefined ? "" : `?page=${page}`;
  state.url = new URL(`http://localhost/genre/x${suffix}`);
}

/**
 * jsdom has no IntersectionObserver, so the sentinel is inert by default. This
 * stand-in records the callbacks so a test can fire the observer and drive the
 * load-more path exactly as scrolling would.
 */
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
  browseAnimeMock.mockReset();
  observerCallbacks.length = 0;
  withGenre("Action");
});

afterEach(() => {
  delete (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver;
});

describe("genre page", () => {
  it("shows a loading state while the request is in flight", () => {
    browseAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("filters by that genre, most popular first", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(browseAnimeMock).toHaveBeenCalledWith(
      { genres: ["Action"], sort: "popularity" },
      1,
      30,
    );
  });

  it("passes a multi-word genre through unchanged", async () => {
    withGenre("Slice of Life");
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: "Slice of Life" });

    expect(browseAnimeMock).toHaveBeenCalledWith(
      { genres: ["Slice of Life"], sort: "popularity" },
      1,
      30,
    );
  });

  it("renders a card per result", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "A"), anime(2, "B")]));

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(screen.getAllByRole("link")).toHaveLength(2);
  });

  it("links each card to its detail route", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(42, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(screen.getByRole("link", { name: "A" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("says so when the genre has no titles", async () => {
    browseAnimeMock.mockResolvedValue(page([]));

    render(Page);

    expect(await screen.findByText(/nothing found/i)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    browseAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("does not query when no genre is present", () => {
    withGenre(undefined);

    render(Page);

    expect(screen.getByText(/no genre selected/i)).toBeInTheDocument();
    expect(browseAnimeMock).not.toHaveBeenCalled();
  });

  // --- pagination ---------------------------------------------------------

  it("starts from the first page whatever the URL says", async () => {
    // The list is grown by scrolling, so `?page=4` has no meaning here: a
    // refresh starts from the top rather than restoring a scroll position.
    withGenre("Action", 4);
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(browseAnimeMock).toHaveBeenCalledWith(expect.anything(), 1, 30);
  });

  it("renders a sentinel when there are more pages", async () => {
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], {
        total: 60,
        currentPage: 1,
        lastPage: 2,
        hasNextPage: true,
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(screen.getByTestId("scroll-sentinel")).toBeInTheDocument();
  });

  it("renders no sentinel on the last page", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    // Nothing left to load, so there is nothing to trigger.
    expect(screen.queryByTestId("scroll-sentinel")).toBeNull();
  });

  it("appends the next page when the sentinel scrolls into view", async () => {
    installObserver();
    browseAnimeMock
      .mockResolvedValueOnce(
        page([anime(1, "Alpha")], {
          total: 60,
          currentPage: 1,
          lastPage: 2,
          hasNextPage: true,
        }),
      )
      .mockResolvedValueOnce(
        page([anime(2, "Beta")], {
          total: 60,
          currentPage: 2,
          lastPage: 2,
          hasNextPage: false,
        }),
      );

    render(Page);
    await screen.findByText("Alpha");

    scrollToBottom();

    // The earlier page stays: the list grows rather than jumping.
    expect(await screen.findByText("Beta")).toBeInTheDocument();
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(browseAnimeMock).toHaveBeenLastCalledWith(expect.anything(), 2, 30);
  });
});