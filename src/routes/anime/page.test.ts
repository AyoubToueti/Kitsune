import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js. Each test sets
// the params (and URL) before rendering.
import { page as appState } from "$app/state";

const getAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getAnime: getAnimeMock };
});

import Page from "./[id]/+page.svelte";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: ["Action", "Adventure"],
    streamingEpisodes: [],
    ...overrides,
  };
}

/** A promise that never settles, to hold the page in its loading state. */
function pending(): Promise<Anime | null> {
  return new Promise(() => {});
}

/**
 * Point the stub at a route param. Same cast workaround as the search page
 * test: svelte-check resolves `$app/state` to SvelteKit's real types which
 * narrow `params` to a route-specific record.
 */
function setId(id: string | number) {
  (appState as { url: URL; params: Record<string, string> }).url = new URL(
    `http://localhost/anime/${id}`,
  );
  (appState as { url: URL; params: Record<string, string> }).params = {
    id: String(id),
  };
}

beforeEach(() => {
  getAnimeMock.mockReset();
  setId(21);
});

describe("detail page", () => {
  it("shows a loading state while the request is in flight", () => {
    getAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("passes the numeric id to getAnime", () => {
    setId(42);
    getAnimeMock.mockReturnValue(pending());

    render(Page);

    expect(getAnimeMock).toHaveBeenCalledWith(42);
  });

  it("renders the title and metadata when loaded", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        format: "TV",
        episodeCount: 24,
        durationMinutes: 23,
        status: "FINISHED",
        seasonYear: 2023,
        averageScore: 85,
      }),
    );

    render(Page);

    expect(
      await screen.findByRole("heading", { name: "One Piece" }),
    ).toBeInTheDocument();
    expect(screen.getByText("TV")).toBeInTheDocument();
    expect(screen.getByText("24 eps")).toBeInTheDocument();
    expect(screen.getByText("23 min")).toBeInTheDocument();
    expect(screen.getByText("FINISHED")).toBeInTheDocument();
    expect(screen.getByText("2023")).toBeInTheDocument();
    expect(screen.getByText("85")).toBeInTheDocument();
  });

  it("shows not found when getAnime returns null", async () => {
    getAnimeMock.mockResolvedValue(null);

    render(Page);

    expect(await screen.findByText(/not found/i)).toBeInTheDocument();
  });

  it("surfaces the backend error message", async () => {
    getAnimeMock.mockRejectedValue("provider returned HTTP 429");

    render(Page);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("renders the episode grid with the correct count", async () => {
    getAnimeMock.mockResolvedValue(anime({ episodeCount: 12 }));

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    // 12 disabled episode buttons
    expect(screen.getAllByRole("button")).toHaveLength(12);
  });

  it("renders streaming links when present", async () => {
    getAnimeMock.mockResolvedValue(
      anime({
        streamingEpisodes: [
          {
            url: "https://crunchyroll.example/watch",
            site: "Crunchyroll",
          },
        ],
      }),
    );

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    expect(screen.getByText("Crunchyroll")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /where to watch/i }),
    ).toBeInTheDocument();
  });

  it("does not render streaming links section when empty", async () => {
    getAnimeMock.mockResolvedValue(anime({ streamingEpisodes: [] }));

    render(Page);
    await screen.findByRole("heading", { name: "One Piece" });

    expect(
      screen.queryByRole("heading", { name: /where to watch/i }),
    ).not.toBeInTheDocument();
  });
});