import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

import { page as appState } from "$app/state";

const getListMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getList: getListMock };
});

// The test lives beside the route, so the import path matches the real
// relative path to the page component.
import Page from "./[filter]/+page.svelte";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
  };
}

function pending(): Promise<Anime[]> {
  return new Promise(() => {});
}

function withFilter(filter: string | undefined) {
  (appState as { params: Record<string, string> }).params =
    filter === undefined ? {} : { filter };
}

beforeEach(() => {
  getListMock.mockReset();
  withFilter("topAiring");
});

describe("browse page", () => {
  it("shows a loading state while the request is in flight", () => {
    getListMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("requests the filter from the URL", async () => {
    getListMock.mockResolvedValue([anime(1, "A")]);

    render(Page);
    await screen.findByRole("heading", { name: "Top airing" });

    expect(getListMock).toHaveBeenCalledWith("topAiring", 30);
  });

  it("renders the human label, not the wire value, as the heading", async () => {
    withFilter("latestCompleted");
    getListMock.mockResolvedValue([anime(1, "A")]);

    render(Page);

    expect(
      await screen.findByRole("heading", { name: "Latest completed" }),
    ).toBeInTheDocument();
  });

  it("renders a card per result", async () => {
    getListMock.mockResolvedValue([anime(1, "A"), anime(2, "B")]);

    render(Page);
    await screen.findByRole("heading", { name: "Top airing" });

    expect(screen.getAllByRole("link")).toHaveLength(2);
  });

  it("rejects an unknown filter without calling the backend", () => {
    withFilter("notAFilter");

    render(Page);

    expect(screen.getByText(/unknown list/i)).toBeInTheDocument();
    // Sending it would fail deserialisation with a confusing message.
    expect(getListMock).not.toHaveBeenCalled();
  });

  it("rejects a missing filter without calling the backend", () => {
    withFilter(undefined);

    render(Page);

    expect(screen.getByText(/unknown list/i)).toBeInTheDocument();
    expect(getListMock).not.toHaveBeenCalled();
  });

  it("says so when the list is empty", async () => {
    getListMock.mockResolvedValue([]);

    render(Page);

    expect(await screen.findByText(/nothing in top airing/i)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    getListMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });
});