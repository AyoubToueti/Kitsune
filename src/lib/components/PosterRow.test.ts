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
    relations: [],
    recommendations: [],
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

  it("omits rank badges by default, since most rows are not rankings", () => {
    render(PosterRow, {
      props: { title: "Trending now", anime: [anime(1, "A"), anime(2, "B")] },
    });

    expect(screen.queryAllByTestId("rank-badge")).toHaveLength(0);
  });

  it("numbers cards when asked", () => {
    render(PosterRow, {
      props: {
        title: "Top 10",
        anime: [anime(1, "A"), anime(2, "B")],
        numbered: true,
      },
    });

    const badges = screen.getAllByTestId("rank-badge");
    expect(badges).toHaveLength(2);
    expect(badges[0]).toHaveTextContent("01");
    expect(badges[1]).toHaveTextContent("02");
  });

  it("zero-pads ranks so the column stays aligned past nine", () => {
    const many = Array.from({ length: 10 }, (_, i) => anime(i + 1, `T${i + 1}`));

    render(PosterRow, {
      props: { title: "Top 10", anime: many, numbered: true },
    });

    const badges = screen.getAllByTestId("rank-badge");
    expect(badges[9]).toHaveTextContent("10");
  });

  it("hides the rank from assistive tech to avoid a duplicate announcement", () => {
    render(PosterRow, {
      props: { title: "Top 10", anime: [anime(1, "A")], numbered: true },
    });

    // The card link already carries the title as its accessible name.
    expect(screen.getByTestId("rank-badge")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });
});