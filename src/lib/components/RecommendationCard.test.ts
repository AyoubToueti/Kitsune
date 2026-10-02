import { describe, it, expect, vi, afterEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";

import type { RecommendedAnime } from "$lib/types";

/**
 * The card's inner `AnimeCard` reads the shared progress store, which reaches
 * Tauri. Mocked as in `AnimeCard.test.ts` so the test stays off the runtime.
 */
const progressForMock = vi.hoisted(() => vi.fn(() => 0));
vi.mock("$lib/watch-progress.svelte", () => ({
  progressFor: progressForMock,
}));

// The vote path reaches the auth API (status, sign-in, and the mutation), all
// of which hit Tauri. Mocked so the test drives them directly.
const authStatusMock = vi.hoisted(() => vi.fn(() => Promise.resolve(true)));
const beginLoginMock = vi.hoisted(() => vi.fn(() => Promise.resolve("https://anilist.test/auth")));
const rateRecommendationMock = vi.hoisted(() => vi.fn(() => Promise.resolve(0)));
vi.mock("$lib/api/auth", () => ({
  authStatus: authStatusMock,
  beginLogin: beginLoginMock,
  rateRecommendation: rateRecommendationMock,
}));

const openUrlMock = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

import RecommendationCard from "./RecommendationCard.svelte";

function recommendation(overrides: Partial<RecommendedAnime> = {}): RecommendedAnime {
  return {
    anime: {
      id: 1,
      provider: "anilist",
      title: { romaji: "Fullmetal Alchemist" },
      genres: [],
      streamingEpisodes: [],
      relations: [],
      recommendations: [],
    },
    rating: 42,
    ...overrides,
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  progressForMock.mockReturnValue(0);
  authStatusMock.mockReset().mockResolvedValue(true);
  beginLoginMock.mockReset().mockResolvedValue("https://anilist.test/auth");
  rateRecommendationMock.mockReset().mockResolvedValue(0);
  openUrlMock.mockReset().mockResolvedValue(undefined);
});

describe("RecommendationCard", () => {
  it("shows the upvote count", () => {
    render(RecommendationCard, { props: { recommendation: recommendation() } });

    expect(screen.getByTestId("recommendation-rating")).toHaveTextContent("42");
  });

  it("credits the recommender by name", () => {
    render(RecommendationCard, {
      props: {
        recommendation: recommendation({
          user: { name: "Suggester", avatar: "https://example.test/a.jpg" },
        }),
      },
    });

    expect(screen.getByText("Suggester")).toBeInTheDocument();
  });

  it("shows the avatar when the recommender has one", () => {
    const { container } = render(RecommendationCard, {
      props: {
        recommendation: recommendation({
          user: { name: "Suggester", avatar: "https://example.test/a.jpg" },
        }),
      },
    });

    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      "https://example.test/a.jpg",
    );
  });

  it("omits the credit when no recommender is reported", () => {
    // An anonymous or deleted account leaves only the votes.
    render(RecommendationCard, { props: { recommendation: recommendation() } });

    expect(screen.queryByTestId("recommendation-user")).toBeNull();
    // The votes still render, so the caption is not empty.
    expect(screen.getByTestId("recommendation-rating")).toBeInTheDocument();
  });

  it("still renders the work's card", () => {
    render(RecommendationCard, { props: { recommendation: recommendation() } });

    expect(screen.getByTestId("anime-card")).toBeInTheDocument();
  });

  // --- voting -------------------------------------------------------------

  it("hides the vote buttons unless votable", () => {
    render(RecommendationCard, { props: { recommendation: recommendation() } });

    expect(screen.queryByTestId("vote-up")).toBeNull();
    expect(screen.queryByTestId("vote-down")).toBeNull();
  });

  it("shows the vote buttons when votable and given a base work", () => {
    render(RecommendationCard, {
      props: { recommendation: recommendation(), votable: true, mediaId: 21 },
    });

    expect(screen.getByTestId("vote-up")).toBeInTheDocument();
    expect(screen.getByTestId("vote-down")).toBeInTheDocument();
  });

  it("sends RATE_UP and shows the returned tally", async () => {
    rateRecommendationMock.mockResolvedValue(43);
    render(RecommendationCard, {
      props: { recommendation: recommendation(), votable: true, mediaId: 21 },
    });

    await fireEvent.click(screen.getByTestId("vote-up"));

    await waitFor(() =>
      expect(rateRecommendationMock).toHaveBeenCalledWith(21, 1, "rateUp"),
    );
    expect(screen.getByTestId("recommendation-rating")).toHaveTextContent("43");
  });

  it("sends RATE_DOWN when the down arrow is clicked", async () => {
    render(RecommendationCard, {
      props: { recommendation: recommendation(), votable: true, mediaId: 21 },
    });

    await fireEvent.click(screen.getByTestId("vote-down"));

    await waitFor(() =>
      expect(rateRecommendationMock).toHaveBeenCalledWith(21, 1, "rateDown"),
    );
  });

  it("clears the vote when the same arrow is pressed twice", async () => {
    render(RecommendationCard, {
      props: {
        recommendation: recommendation({ userRating: "rateUp" }),
        votable: true,
        mediaId: 21,
      },
    });

    await fireEvent.click(screen.getByTestId("vote-up"));

    await waitFor(() =>
      expect(rateRecommendationMock).toHaveBeenCalledWith(21, 1, "noRating"),
    );
  });

  it("reverts the tally when the write fails", async () => {
    rateRecommendationMock.mockRejectedValue("nope");
    render(RecommendationCard, {
      props: { recommendation: recommendation(), votable: true, mediaId: 21 },
    });

    await fireEvent.click(screen.getByTestId("vote-up"));

    // Optimistically 43, then back to 42 once the write is rejected.
    await waitFor(() =>
      expect(screen.getByTestId("recommendation-rating")).toHaveTextContent("42"),
    );
    expect(screen.getByTestId("vote-error")).toBeInTheDocument();
  });

  it("prompts sign-in instead of voting when signed out", async () => {
    authStatusMock.mockResolvedValue(false);
    render(RecommendationCard, {
      props: { recommendation: recommendation(), votable: true, mediaId: 21 },
    });

    await fireEvent.click(screen.getByTestId("vote-up"));

    await waitFor(() => expect(beginLoginMock).toHaveBeenCalled());
    expect(openUrlMock).toHaveBeenCalledWith("https://anilist.test/auth");
    expect(rateRecommendationMock).not.toHaveBeenCalled();
  });

  it("highlights the reader's current vote", () => {
    render(RecommendationCard, {
      props: {
        recommendation: recommendation({ userRating: "rateDown" }),
        votable: true,
        mediaId: 21,
      },
    });

    expect(screen.getByTestId("vote-down")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByTestId("vote-up")).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });
});