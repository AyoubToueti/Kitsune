import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import type { Anime } from "$lib/types";

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
      anime(1, "One Piece"),
      anime(2, "Naruto"),
    ]);

    render(ContinueWatching);

    expect(await screen.findByText("One Piece")).toBeInTheDocument();
    expect(screen.getByText("Naruto")).toBeInTheDocument();
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

    getContinueWatchingMock.mockResolvedValue([anime(1, "One Piece")]);
    await fireEvent.click(screen.getByRole("button", { name: /try again/i }));

    expect(await screen.findByText("One Piece")).toBeInTheDocument();
  });
});