import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import PosterRow from "./PosterRow.svelte";
import type { Anime } from "$lib/types";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
  };
}

describe("PosterRow", () => {
  it("shows the row heading", () => {
    render(PosterRow, {
      props: { title: "Trending now", anime: [anime(1, "A")] },
    });

    expect(
      screen.getByRole("heading", { name: "Trending now" }),
    ).toBeInTheDocument();
  });

  it("renders one card per entry", () => {
    render(PosterRow, {
      props: {
        title: "Trending now",
        anime: [anime(1, "A"), anime(2, "B"), anime(3, "C")],
      },
    });

    expect(screen.getAllByRole("link")).toHaveLength(3);
    expect(screen.getByText("B")).toBeInTheDocument();
  });

  it("renders nothing at all when the list is empty", () => {
    // An empty heading would be a stray label with nothing beneath it.
    render(PosterRow, { props: { title: "Trending now", anime: [] } });

    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
  });
});