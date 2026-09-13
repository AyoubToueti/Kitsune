import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

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

beforeEach(() => {
  browseAnimeMock.mockReset().mockResolvedValue(page([]));
  getGenresMock.mockReset().mockResolvedValue(["Action", "Comedy"]);
  withQuery("");
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

  it("requests the page named in the URL", async () => {
    withQuery("?format=tv&page=3");

    render(Page);

    expect(browseAnimeMock).toHaveBeenCalledWith(
      { format: "tv", sort: "popularity" },
      3,
      30,
    );
  });

  it("treats a nonsense page as the first page", async () => {
    withQuery("?page=abc");

    render(Page);

    expect(browseAnimeMock).toHaveBeenCalledWith({ sort: "popularity" }, 1, 30);
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

    expect(
      await screen.findByRole("checkbox", { name: "Action" }),
    ).toBeInTheDocument();
  });

  it("still renders the other filters when the genre list fails", async () => {
    getGenresMock.mockRejectedValue("nope");

    render(Page);

    // No checkboxes, but the form is usable.
    expect(await screen.findByTestId("filter-form")).toBeInTheDocument();
    expect(screen.getByLabelText("Type")).toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).toBeNull();
  });

  // --- pagination ---------------------------------------------------------

  it("shows no pagination for a single page of results", async () => {
    browseAnimeMock.mockResolvedValue(page([anime(1, "A")]));

    render(Page);
    await screen.findByText("A");

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
    await screen.findByText("A");

    expect(screen.getByTestId("pagination")).toBeInTheDocument();
  });

  it("page links preserve the filters", async () => {
    withQuery("?format=tv&genre=Action");
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], {
        total: 90,
        currentPage: 1,
        lastPage: 3,
        hasNextPage: true,
      }),
    );

    render(Page);
    await screen.findByText("A");

    // Losing the filters would silently reset the view on every page change.
    const href = screen.getByRole("link", { name: "Next page" }).getAttribute("href")!;
    expect(href).toContain("format=tv");
    expect(href).toContain("genre=Action");
    expect(href).toContain("page=2");
  });

  it("page links omit the page parameter for the first page", async () => {
    withQuery("?format=tv&page=2");
    browseAnimeMock.mockResolvedValue(
      page([anime(1, "A")], {
        total: 90,
        currentPage: 2,
        lastPage: 3,
        hasNextPage: true,
      }),
    );

    render(Page);
    await screen.findByText("A");

    // One canonical URL per view rather than two spellings of the same results.
    expect(screen.getByRole("link", { name: "Page 1" })).toHaveAttribute(
      "href",
      "/filter?format=tv",
    );
  });
});