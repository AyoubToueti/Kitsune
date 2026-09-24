import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import type { Anime, ContinueWatchingItem } from "$lib/types";

const getContinueWatchingMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/auth", () => ({
  getContinueWatching: getContinueWatchingMock,
}));

import ContinueWatching from "./ContinueWatching.svelte";

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

/** A continue-watching entry: the work plus how far the reader got. */
function entry(id: number, title: string, progress = 0): ContinueWatchingItem {
  return { anime: anime(id, title), progress };
}

beforeEach(() => {
  getContinueWatchingMock.mockReset().mockResolvedValue([]);
});

describe("ContinueWatching", () => {
  it("shows nothing while signed out", async () => {
    // The backend answers an empty list when there is no token.
    getContinueWatchingMock.mockResolvedValue([]);

    render(ContinueWatching);

    // Settle the load, then confirm the heading never appeared.
    await screen.findByText(/continue watching/i).catch(() => {});
    await Promise.resolve();

    expect(screen.queryByText(/continue watching/i)).toBeNull();
  });

  it("renders a card per work", async () => {
    getContinueWatchingMock.mockResolvedValue([
      entry(1, "One Piece"),
      entry(2, "Naruto"),
    ]);

    render(ContinueWatching);

    expect(await screen.findByText("One Piece")).toBeInTheDocument();
    expect(screen.getByText("Naruto")).toBeInTheDocument();
  });

  it("links to the full list", async () => {
    getContinueWatchingMock.mockResolvedValue([entry(1, "One Piece")]);

    render(ContinueWatching);
    await screen.findByText("One Piece");

    expect(screen.getByRole("link", { name: /view all/i })).toHaveAttribute(
      "href",
      "/list",
    );
  });

  it("resumes at the next episode after the one watched", async () => {
    // `progress` is the last episode NUMBER started; the watch page's `?ep=`
    // is a 0-based index, so episode 3 is index 2.
    getContinueWatchingMock.mockResolvedValue([entry(1, "One Piece", 3)]);

    render(ContinueWatching);

    const resume = await screen.findByTestId("resume");
    expect(resume).toHaveAttribute("href", "/watch/1?ep=2");
  });

  it("starts from the beginning when nothing has been watched", async () => {
    getContinueWatchingMock.mockResolvedValue([entry(1, "One Piece", 0)]);

    render(ContinueWatching);

    // Never clamped to -1: an unstarted work resumes at the first episode.
    expect(await screen.findByTestId("resume")).toHaveAttribute(
      "href",
      "/watch/1?ep=0",
    );
  });

  it("asks for a bounded number of works", async () => {
    render(ContinueWatching);

    await screen.findByText(/continue watching/i).catch(() => {});
    await Promise.resolve();

    expect(getContinueWatchingMock).toHaveBeenCalledTimes(1);
    const [limit] = getContinueWatchingMock.mock.calls[0];
    expect(typeof limit).toBe("number");
    expect(limit).toBeGreaterThan(0);
  });

  it("reports a failure and offers a retry", async () => {
    getContinueWatchingMock.mockRejectedValue(new Error("offline"));

    render(ContinueWatching);

    expect(await screen.findByText(/could not load/i)).toBeInTheDocument();

    getContinueWatchingMock.mockResolvedValue([entry(1, "One Piece")]);
    await fireEvent.click(screen.getByRole("button", { name: /try again/i }));

    expect(await screen.findByText("One Piece")).toBeInTheDocument();
  });
});