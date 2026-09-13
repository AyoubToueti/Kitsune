import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

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

beforeEach(() => {
  browseAnimeMock.mockReset();
  setUrl(new URL("http://localhost/search"));
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

  it("requests the page named in the URL", async () => {
    withQuery("naruto", 3);
    browseAnimeMock.mockResolvedValue(page([anime(1, "Naruto")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    expect(browseAnimeMock).toHaveBeenCalledWith(expect.anything(), 3, 30);
  });

  it("treats a nonsense page as the first page", async () => {
    // A hand-edited ?page=abc must not send NaN to the backend.
    setUrl(new URL("http://localhost/search?q=a&page=abc"));
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

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

  // --- pagination ---------------------------------------------------------

  it("shows no pagination for a single page of results", async () => {
    withQuery("a");
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    expect(screen.queryByTestId("pagination")).toBeNull();
  });

  it("offers pagination when there are more pages", async () => {
    withQuery("a");
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], { total: 90, currentPage: 1, lastPage: 3, hasNextPage: true }),
    );

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    expect(screen.getByTestId("pagination")).toBeInTheDocument();
  });

  it("page links keep the search term", async () => {
    withQuery("one piece");
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], { total: 90, currentPage: 1, lastPage: 3, hasNextPage: true }),
    );

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    // Losing the query would turn paging into a search for nothing.
    const link = screen.getByRole("link", { name: "Next page" });
    expect(link.getAttribute("href")).toContain("q=one%20piece");
    expect(link.getAttribute("href")).toContain("page=2");
  });

  it("marks the requested page as current", async () => {
    withQuery("a", 2);
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], { total: 90, currentPage: 2, lastPage: 3, hasNextPage: true }),
    );

    render(Page);

    expect(await screen.findByTestId("current-page")).toHaveTextContent("2");
  });
});