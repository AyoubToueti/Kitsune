import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";

import type { Anime, AnimePage } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js. Each test sets the
// URL before rendering.
import { page as appState } from "$app/state";

const browseAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, browseAnime: browseAnimeMock };
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

/** A page of results, with the paging metadata the UI needs. */
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
 * Point the stub at a URL.
 *
 * The cast is needed because svelte-check resolves `$app/state` to
 * SvelteKit's real types, which narrow `pathname` to a union of the app's
 * routes. A plain `URL` cannot satisfy that, and the stub is deliberately
 * not route-aware.
 */
function setUrl(url: URL) {
  (appState as { url: URL }).url = url;
}

function withQuery(q: string, page?: number) {
  const suffix = page === undefined ? "" : `&page=${page}`;
  setUrl(new URL(`http://localhost/search?q=${encodeURIComponent(q)}${suffix}`));
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
  setUrl(new URL("http://localhost/search"));
});

afterEach(() => {
  delete (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver;
});

describe("search page", () => {
  it("prompts for input when there is no query", () => {
    render(Page);

    expect(screen.getByText(/search box above/i)).toBeInTheDocument();
    expect(browseAnimeMock).not.toHaveBeenCalled();
  });

  it("does not search for a whitespace-only query", () => {
    withQuery("   ");

    render(Page);

    expect(screen.getByText(/search box above/i)).toBeInTheDocument();
    expect(browseAnimeMock).not.toHaveBeenCalled();
  });

  it("shows a loading state while searching", () => {
    withQuery("one piece");
    browseAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/searching/i)).toBeInTheDocument();
  });

  it("searches the trimmed term with relevance ordering", async () => {
    withQuery("  one piece  ");
    browseAnimeMock.mockResolvedValue(page([anime(1, "One Piece")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    // Relevance, not popularity: a search should return the best matches.
    expect(browseAnimeMock).toHaveBeenCalledWith(
      { search: "one piece", sort: "searchMatch" },
      1,
      30,
    );
  });

  it("starts from the first page whatever the URL says", async () => {
    // The list is grown by scrolling, so `?page=3` has no meaning here: a
    // refresh starts from the top rather than restoring a scroll position.
    withQuery("naruto", 3);
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    expect(browseAnimeMock).toHaveBeenCalledWith(expect.anything(), 1, 30);
  });

  it("renders a card per result", async () => {
    withQuery("a");
    browseAnimeMock.mockResolvedValue(page([anime(1, "Alpha"), anime(2, "Beta")]));

    render(Page);

    expect(await screen.findByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("echoes the query in the results heading", async () => {
    withQuery("naruto");
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);

    expect(
      await screen.findByRole("heading", { name: /naruto/i }),
    ).toBeInTheDocument();
  });

  it("says so when a search returned nothing", async () => {
    withQuery("zzzzz");
    browseAnimeMock.mockResolvedValue(page([]));

    render(Page);

    expect(await screen.findByText(/no results/i)).toBeInTheDocument();
    // The empty state names the term so the user knows what was searched.
    expect(screen.getByText(/zzzzz/)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    withQuery("a");
    browseAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  // --- infinite scroll ------------------------------------------------------

  it("renders a sentinel when there are more pages", async () => {
    withQuery("a");
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], { total: 60, currentPage: 1, lastPage: 2, hasNextPage: true }),
    );

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    expect(screen.getByTestId("scroll-sentinel")).toBeInTheDocument();
  });

  it("renders no sentinel on the last page", async () => {
    withQuery("a");
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    // Nothing left to load, so there is nothing to trigger.
    expect(screen.queryByTestId("scroll-sentinel")).toBeNull();
  });

  it("appends the next page when the sentinel scrolls into view", async () => {
    installObserver();
    withQuery("a");
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

  it("keeps the search term when loading the next page", async () => {
    installObserver();
    withQuery("one piece");
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

    // Losing the query would turn the second page into a search for nothing.
    await screen.findByText("Beta");
    expect(browseAnimeMock).toHaveBeenLastCalledWith(
      expect.objectContaining({ search: "one piece" }),
      2,
      30,
    );
  });

  // --- filters on top of the search -----------------------------------------

  it("carries the term through filter submission", async () => {
    withQuery("naruto");
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });
    // The form only exists once the dropdown is open.
    await fireEvent.click(screen.getByTestId("open-filters"));

    // A GET submit replaces the whole query string, so if the term is not
    // re-sent as a hidden field, changing any filter would silently wipe it.
    const data = new FormData(screen.getByTestId("filter-form") as HTMLFormElement);
    expect(data.get("q")).toBe("naruto");
  });

  it("does not render its own search field", async () => {
    withQuery("naruto");
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);

    // The navbar's search box owns the term; two boxes writing different
    // parameters to one URL would lose it.
    expect(screen.queryByLabelText("Search")).toBeNull();
  });

  it("does not render sort options", async () => {
    withQuery("naruto");
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);

    // A text search is ranked by relevance, and the panel deliberately does
    // not offer relevance -- so it must not offer any other order either.
    expect(screen.queryByLabelText("Sort")).toBeNull();
  });

  it("applies a tag filter alongside the search term", async () => {
    // Built directly rather than through `withQuery`, which encodes its whole
    // argument -- turning `&` into literal text and hiding the filter.
    setUrl(new URL("http://localhost/search?q=naruto&tag=Isekai"));
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    // Both must reach the backend: the term AND the filter, with relevance
    // still winning the ordering.
    expect(browseAnimeMock).toHaveBeenCalledWith(
      expect.objectContaining({ search: "naruto", tags: ["Isekai"] }),
      1,
      30,
    );
  });

  it("shows a pill for an applied tag", async () => {
    setUrl(new URL("http://localhost/search?q=naruto&tag=Isekai"));
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    expect(
      screen.getByRole("link", { name: /remove isekai filter/i }),
    ).toBeInTheDocument();
  });

  it("links filter removal back to /search", async () => {
    setUrl(new URL("http://localhost/search?q=naruto&tag=Isekai"));
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    const href =
      screen
        .getByRole("link", { name: /remove isekai filter/i })
        .getAttribute("href") ?? "";

    // Removing a filter must not bounce the user to /filter and lose the term.
    expect(href.startsWith("/search")).toBe(true);
    expect(href).toContain("q=naruto");
  });

  it("does not query for an empty term", () => {
    render(Page);

    // The key folds in the term; with none, the composable resolves an empty
    // page itself rather than asking the backend for nothing.
    expect(browseAnimeMock).not.toHaveBeenCalled();
  });
});