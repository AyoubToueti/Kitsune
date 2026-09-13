import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js. Each test sets
// the URL before rendering; the component reads it once per render.
import { page as appState } from "$app/state";

const searchAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, searchAnime: searchAnimeMock };
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

function withQuery(q: string) {
  setUrl(new URL(`http://localhost/search?q=${encodeURIComponent(q)}`));
}

beforeEach(() => {
  searchAnimeMock.mockReset();
  setUrl(new URL("http://localhost/search"));
});

describe("search page", () => {
  it("prompts for input when there is no query", () => {
    render(Page);

    expect(screen.getByText(/search box above/i)).toBeInTheDocument();
    expect(searchAnimeMock).not.toHaveBeenCalled();
  });

  it("does not search for a whitespace-only query", () => {
    withQuery("   ");

    render(Page);

    expect(screen.getByText(/search box above/i)).toBeInTheDocument();
    expect(searchAnimeMock).not.toHaveBeenCalled();
  });

  it("shows a loading state while searching", () => {
    withQuery("one piece");
    searchAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/searching/i)).toBeInTheDocument();
  });

  it("passes the trimmed query and a limit to the backend", async () => {
    withQuery("  one piece  ");
    searchAnimeMock.mockResolvedValue([anime(1, "One Piece")]);

    render(Page);
    await screen.findByRole("heading", { name: /results for/i });

    expect(searchAnimeMock).toHaveBeenCalledWith("one piece", 30);
  });

  it("renders a card per result", async () => {
    withQuery("a");
    searchAnimeMock.mockResolvedValue([anime(1, "Alpha"), anime(2, "Beta")]);

    render(Page);

    expect(await screen.findByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("echoes the query in the results heading", async () => {
    withQuery("naruto");
    searchAnimeMock.mockResolvedValue([anime(1, "Naruto")]);

    render(Page);

    expect(
      await screen.findByRole("heading", { name: /naruto/i }),
    ).toBeInTheDocument();
  });

  it("says so when a search returned nothing", async () => {
    withQuery("zzzzz");
    searchAnimeMock.mockResolvedValue([]);

    render(Page);

    expect(await screen.findByText(/no results/i)).toBeInTheDocument();
    // The empty state names the term so the user knows what was searched.
    expect(screen.getByText(/zzzzz/)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    withQuery("a");
    searchAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });
});