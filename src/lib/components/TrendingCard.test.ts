import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import TrendingCard from "./TrendingCard.svelte";
import type { Anime } from "$lib/types";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

describe("TrendingCard", () => {
  it("links to the detail route", () => {
    render(TrendingCard, { props: { anime: anime({ id: 42 }), rank: 1 } });

    expect(screen.getByRole("link", { name: "One Piece" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("shows the rank, zero-padded", () => {
    render(TrendingCard, { props: { anime: anime(), rank: 3 } });

    expect(screen.getByTestId("rank-badge")).toHaveTextContent("03");
  });

  it("keeps the rank two digits past nine", () => {
    render(TrendingCard, { props: { anime: anime(), rank: 12 } });

    expect(screen.getByTestId("rank-badge")).toHaveTextContent("12");
  });

  it("writes the title up the edge", () => {
    render(TrendingCard, { props: { anime: anime(), rank: 1 } });

    const title = screen.getByText("One Piece");
    expect(title.style.writingMode).toBe("vertical-rl");
    expect(title.style.transform).toContain("rotate(180deg)");
  });

  it("falls back to Untitled when no title form is present", () => {
    render(TrendingCard, { props: { anime: anime({ title: {} }), rank: 1 } });

    expect(screen.getByRole("link", { name: "Untitled" })).toBeInTheDocument();
  });

  it("shows the score when present", () => {
    render(TrendingCard, { props: { anime: anime({ averageScore: 88 }), rank: 1 } });

    expect(screen.getByText("88")).toBeInTheDocument();
  });

  it("shows a placeholder rather than a broken image when there is no cover", () => {
    render(TrendingCard, { props: { anime: anime(), rank: 1 } });

    expect(screen.getByText("No cover")).toBeInTheDocument();
  });
});