import { describe, it, expect, vi, beforeEach } from "vitest";
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

beforeEach(() => {
  browseAnimeMock.mockReset();
  withGenre("Action");
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

  it("requests the page named in the URL", async () => {
    withGenre("Action", 4);
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(browseAnimeMock).toHaveBeenCalledWith(expect.anything(), 4, 30);
  });

  it("shows no pagination for a single page of results", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(screen.queryByTestId("pagination")).toBeNull();
  });

  it("offers pagination when there are more pages", async () => {
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], {
        total: 90,
        currentPage: 1,
        lastPage: 3,
        hasNextPage: true,
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(screen.getByTestId("pagination")).toBeInTheDocument();
  });

  it("page links keep the genre in the path", async () => {
    withGenre("Slice of Life");
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], {
        total: 90,
        currentPage: 1,
        lastPage: 3,
        hasNextPage: true,
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "Slice of Life" });

    // Losing the genre would turn paging into a browse of everything.
    expect(screen.getByRole("link", { name: "Next page" })).toHaveAttribute(
      "href",
      "/genre/Slice%20of%20Life?page=2",
    );
  });

  it("omits the query string for the first page", async () => {
    withGenre("Action", 2);
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], {
        total: 90,
        currentPage: 2,
        lastPage: 3,
        hasNextPage: true,
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    // Page 1 has one canonical URL rather than two.
    expect(screen.getByRole("link", { name: "Page 1" })).toHaveAttribute(
      "href",
      "/genre/Action",
    );
  });
});