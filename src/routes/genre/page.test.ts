import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js. Each test sets the
// params before rendering.
import { page as appState } from "$app/state";

const getByGenreMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getByGenre: getByGenreMock };
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

/** A promise that never settles, to hold the page in its loading state. */
function pending(): Promise<Anime[]> {
  return new Promise(() => {});
}

/**
 * Point the stub at a genre.
 *
 * SvelteKit decodes route params, so the component receives an already
 * readable name -- "Slice of Life", not "Slice%20of%20Life".
 */
function withGenre(name: string | undefined) {
  (appState as { params: Record<string, string> }).params =
    name === undefined ? {} : { name };
}

beforeEach(() => {
  getByGenreMock.mockReset();
  withGenre("Action");
});

describe("genre page", () => {
  it("shows a loading state while the request is in flight", () => {
    getByGenreMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("asks for a bounded number of titles in that genre", async () => {
    getByGenreMock.mockResolvedValue([anime(1, "A")]);

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(getByGenreMock).toHaveBeenCalledWith("Action", 30);
  });

  it("passes a multi-word genre through unchanged", async () => {
    withGenre("Slice of Life");
    getByGenreMock.mockResolvedValue([anime(1, "A")]);

    render(Page);
    await screen.findByRole("heading", { name: "Slice of Life" });

    expect(getByGenreMock).toHaveBeenCalledWith("Slice of Life", 30);
  });

  it("renders a card per result", async () => {
    getByGenreMock.mockResolvedValue([anime(1, "A"), anime(2, "B")]);

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(screen.getAllByRole("link")).toHaveLength(2);
  });

  it("links each card to its detail route", async () => {
    getByGenreMock.mockResolvedValue([anime(42, "A")]);

    render(Page);
    await screen.findByRole("heading", { name: "Action" });

    expect(screen.getByRole("link", { name: "A" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("says so when the genre has no titles", async () => {
    getByGenreMock.mockResolvedValue([]);

    render(Page);

    expect(await screen.findByText(/nothing found/i)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    getByGenreMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("does not query when no genre is present", () => {
    withGenre(undefined);

    render(Page);

    expect(screen.getByText(/no genre selected/i)).toBeInTheDocument();
    expect(getByGenreMock).not.toHaveBeenCalled();
  });
});