import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";

import type { ContinueWatchingItem } from "$lib/types";

// Aliased to src/test/app-state-stub.ts in vitest.config.js, so the URL can be
// set per test to exercise the hide-on-/watch rule.
import { page as appState } from "$app/state";

const getContinueWatchingMock = vi.hoisted(() => vi.fn());
const getLastPlayedMock = vi.hoisted(() => vi.fn());
const onAuthChangedMock = vi.hoisted(() => vi.fn());
const onListChangedMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/auth", () => ({
  getContinueWatching: getContinueWatchingMock,
  getLastPlayed: getLastPlayedMock,
  onAuthChanged: onAuthChangedMock,
  onListChanged: onListChangedMock,
}));

// The disc looks the recorded work up for its cover and title, so `getAnime`
// is mocked too.
const getAnimeMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", () => ({ getAnime: getAnimeMock }));

import ResumeDisc from "./ResumeDisc.svelte";

/** An `Anime` fixture for the work a record points at. */
function anime(id: number, title: string) {
  return {
    id,
    provider: "anilist" as const,
    title: { romaji: title },
    coverImage: "https://example.test/cover.jpg",
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
  };
}

function item(progress: number): ContinueWatchingItem {
  return {
    anime: {
      id: 21,
      provider: "anilist",
      title: { romaji: "One Piece" },
      coverImage: "https://example.test/cover.jpg",
      genres: [],
      streamingEpisodes: [],
      relations: [],
      recommendations: [],
    },
    progress,
  };
}

/** Point the stubbed `$app/state` at a path. */
function setPath(pathname: string) {
  (appState as { url: URL }).url = new URL(`http://localhost${pathname}`);
}

beforeEach(() => {
  // Default: nothing recorded, so tests that do not care fall through to the
  // list.
  getLastPlayedMock.mockReset().mockResolvedValue(null);
  getContinueWatchingMock.mockReset().mockResolvedValue([]);
  getAnimeMock.mockReset().mockImplementation((id: number) =>
    Promise.resolve(anime(id, "One Piece")),
  );
  onAuthChangedMock.mockReset().mockResolvedValue(() => {});
  onListChangedMock.mockReset().mockResolvedValue(() => {});
  setPath("/");
});

describe("ResumeDisc", () => {
  it("renders nothing while signed out", async () => {
    // The backend answers an empty list when there is no token.
    getContinueWatchingMock.mockResolvedValue([]);

    render(ResumeDisc);

    // Settle the load, then confirm the disc never appeared.
    await waitFor(() => expect(getContinueWatchingMock).toHaveBeenCalled());
    expect(screen.queryByTestId("resume-disc")).toBeNull();
  });

  it("links to the last-watched episode", async () => {
    getContinueWatchingMock.mockResolvedValue([item(3)]);

    render(ResumeDisc);

    // progress 3 is a 1-based number; ?ep= is a 0-based index, so episode 3.
    expect(await screen.findByTestId("resume-disc")).toHaveAttribute(
      "href",
      "/watch/21?ep=2",
    );
  });

  it("resumes a never-started work at the first episode", async () => {
    getContinueWatchingMock.mockResolvedValue([item(0)]);

    render(ResumeDisc);

    expect(await screen.findByTestId("resume-disc")).toHaveAttribute(
      "href",
      "/watch/21?ep=0",
    );
  });

  it("names the work and episode for screen readers", async () => {
    getContinueWatchingMock.mockResolvedValue([item(5)]);

    render(ResumeDisc);

    expect(
      await screen.findByLabelText("Resume One Piece at episode 5"),
    ).toBeInTheDocument();
  });

  it("shows the anime and episode on the now-playing card", async () => {
    getContinueWatchingMock.mockResolvedValue([item(5)]);

    render(ResumeDisc);

    const card = await screen.findByTestId("resume-disc-card");
    expect(card).toHaveTextContent("One Piece");
    expect(card).toHaveTextContent("Episode 5");
  });

  it("says a work has not been started rather than episode 0", async () => {
    getContinueWatchingMock.mockResolvedValue([item(0)]);

    render(ResumeDisc);

    const card = await screen.findByTestId("resume-disc-card");
    expect(card).toHaveTextContent("Not started");
  });

  /// The whole point of the local record: it answers "what did I last open",
  /// which the list cannot, because AniList only reorders on a CHANGE.
  it("prefers the locally recorded work over the list", async () => {
    getLastPlayedMock.mockResolvedValue({
      animeId: 99,
      episode: 6,
      at: 1_700_000_000,
    });
    getAnimeMock.mockImplementation((id: number) =>
      Promise.resolve(anime(id, "Smoking Behind the Supermarket")),
    );
    // The list says something else entirely; the record must win.
    getContinueWatchingMock.mockResolvedValue([item(3)]);

    render(ResumeDisc);

    const disc = await screen.findByTestId("resume-disc");
    expect(disc).toHaveAttribute("href", "/watch/99?ep=5");
    expect(disc).toHaveTextContent("Smoking Behind the Supermarket");
    // The list was never consulted, because a record existed.
    expect(getContinueWatchingMock).not.toHaveBeenCalled();
  });

  /// A record for a work that cannot be fetched falls back to the list rather
  /// than showing an empty disc.
  it("falls back to the list when the recorded work cannot be fetched", async () => {
    getLastPlayedMock.mockResolvedValue({ animeId: 99, episode: 6, at: 1 });
    getAnimeMock.mockResolvedValue(null);
    getContinueWatchingMock.mockResolvedValue([item(3)]);

    render(ResumeDisc);

    expect(await screen.findByTestId("resume-disc")).toHaveAttribute(
      "href",
      "/watch/21?ep=2",
    );
  });

  /// The disc lives in the layout, which never remounts -- so without this it
  /// would show whatever was newest when the app started, all session.
  it("refetches when the list changes", async () => {
    getContinueWatchingMock.mockResolvedValue([item(3)]);

    let fire: (() => void) | undefined;
    onListChangedMock.mockImplementation((cb: () => void) => {
      fire = cb;
      return Promise.resolve(() => {});
    });

    render(ResumeDisc);
    await screen.findByTestId("resume-disc");
    expect(getContinueWatchingMock).toHaveBeenCalledTimes(1);

    // A write elsewhere in the app fires the event...
    getContinueWatchingMock.mockResolvedValue([item(9)]);
    fire!();

    // ...and the disc re-reads, picking up the newer work.
    await waitFor(() => expect(getContinueWatchingMock).toHaveBeenCalledTimes(2));
  });

  /// Resuming what is already on screen is nonsense, and the disc would sit
  /// over the player.
  it("hides itself on the watch page", async () => {
    setPath("/watch/21");
    getContinueWatchingMock.mockResolvedValue([item(3)]);

    render(ResumeDisc);

    await waitFor(() => expect(getContinueWatchingMock).toHaveBeenCalled());
    expect(screen.queryByTestId("resume-disc")).toBeNull();
  });
});