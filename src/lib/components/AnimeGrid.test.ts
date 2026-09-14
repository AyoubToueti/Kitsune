import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import AnimeGrid from "./AnimeGrid.svelte";
import type { Anime } from "$lib/types";

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

describe("AnimeGrid", () => {
  it("renders one card per entry", () => {
    render(AnimeGrid, {
      props: { anime: [anime(1, "A"), anime(2, "B"), anime(3, "C")] },
    });

    expect(screen.getAllByRole("link")).toHaveLength(3);
  });

  it("shows each title", () => {
    render(AnimeGrid, { props: { anime: [anime(1, "One Piece")] } });

    expect(screen.getByText("One Piece")).toBeInTheDocument();
  });

  it("renders an empty list rather than failing", () => {
    const { container } = render(AnimeGrid, { props: { anime: [] } });

    // The grid element is still present so a caller can rely on it existing.
    expect(container.querySelector("[data-testid='anime-grid']")).not.toBeNull();
    expect(screen.queryAllByRole("link")).toHaveLength(0);
  });

  it("links each card to its detail route", () => {
    render(AnimeGrid, { props: { anime: [anime(42, "A")] } });

    expect(screen.getByRole("link", { name: "A" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });
});