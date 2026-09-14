import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";

import type { Anime, AnimePage } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js.
import { page as appState } from "$app/state";

const browseAnimeMock = vi.hoisted(() => vi.fn());
const getGenresMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, browseAnime: browseAnimeMock, getGenres: getGenresMock };
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

function pending(): Promise<AnimePage> {
  return new Promise(() => {});
}

/** Point the stub at a filter URL. */
function withQuery(qs: string) {
  (appState as { url: URL }).url = new URL(`http://localhost/filter${qs}`);
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
  browseAnimeMock.mockReset().mockResolvedValue(page([]));
  getGenresMock.mockReset().mockResolvedValue(["Action", "Comedy"]);
  observerCallbacks.length = 0;
  withQuery("");
});

afterEach(() => {
  delete (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver;
});

describe("filter page", () => {
  it("loads with default filters", async () => {
    render(Page);

    expect(browseAnimeMock).toHaveBeenCalledWith({ sort: "popularity" }, 1, 30);
  });

  it("shows a loading state while the request is in flight", () => {
    browseAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("sends the filters named in the URL", async () => {
    withQuery("?format=tv&status=finished&season=fall&year=2024&score=70&sort=score");

    render(Page);

    expect(browseAnimeMock).toHaveBeenCalledWith(
      {
        format: "tv",
        status: "finished",
        season: "fall",
        seasonYear: 2024,
        minScore: 70,
        sort: "score",
      },
      1,
      30,
    );
  });

  it("sends every selected genre", async () => {
    withQuery("?genre=Action&genre=Comedy");

    render(Page);

    expect(browseAnimeMock).toHaveBeenCalledWith(
      { genres: ["Action", "Comedy"], sort: "popularity" },
      1,
      30,
    );
  });

  it("drops an unrecognised filter rather than forwarding it", async () => {
    // The backend rejects an unknown enum, so forwarding one would surface as
    // a confusing request failure instead of the bad link it is.
    withQuery("?format=bluray&season=monsoon&sort=chaos");

    render(Page);

    expect(browseAnimeMock).toHaveBeenCalledWith({ sort: "popularity" }, 1, 30);
  });

  it("starts from the first page whatever the URL says", async () => {
    // The list is grown by scrolling, so `?page=3` has no meaning here: a
    // refresh starts from the top rather than restoring a scroll position.
    withQuery("?format=tv&page=3");

    render(Page);

    expect(browseAnimeMock).toHaveBeenCalledWith(
      { format: "tv", sort: "popularity" },
      1,
      30,
    );
  });

  it("renders a card per result", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "Alpha"), anime(2, "Beta")]));

    render(Page);

    expect(await screen.findByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("says so when nothing matches", async () => {
    browseAnimeMock.mockResolvedValue(page([]));

    render(Page);

    expect(await screen.findByText(/no titles match/i)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    browseAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("loads the genre list for the panel", async () => {
    render(Page);

    // The panel lives in the dropdown, and the genres live in the catalogue
    // inside it, which starts collapsed. Both disclosures have to be opened
    // before the checkboxes exist in the DOM.
    await fireEvent.click(await screen.findByTestId("open-filters"));
    await fireEvent.click(await screen.findByTestId("toggle-catalogue"));

    expect(
      await screen.findByRole("checkbox", { name: "Action" }),
    ).toBeInTheDocument();
  });

  it("still renders the other filters when the genre list fails", async () => {
    getGenresMock.mockRejectedValue("nope");

    render(Page);
    await fireEvent.click(await screen.findByTestId("open-filters"));

    // No checkboxes, but the form is usable.
    expect(await screen.findByTestId("filter-form")).toBeInTheDocument();
    expect(screen.getByLabelText("Type")).toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).toBeNull();
  });

  // --- infinite scroll ------------------------------------------------------

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
    await screen.findByText("A");

    expect(screen.getByTestId("scroll-sentinel")).toBeInTheDocument();
  });

  it("renders no sentinel on the last page", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByText("A");

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

  it("retries the failed page", async () => {
    browseAnimeMock
      .mockRejectedValueOnce("provider returned HTTP 429")
      .mockResolvedValueOnce(page([anime(1, "A")]));

    render(Page);
    await screen.findByText(/429/);

    await fireEvent.click(screen.getByRole("button", { name: /try again/i }));

    expect(await screen.findByText("A")).toBeInTheDocument();
    expect(browseAnimeMock).toHaveBeenLastCalledWith(expect.anything(), 1, 30);
  });
});