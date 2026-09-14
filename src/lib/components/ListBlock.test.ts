import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import ListBlock from "./ListBlock.svelte";
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

describe("ListBlock", () => {
  it("renders nothing for an empty list", () => {
    render(ListBlock, {
      props: { title: "Top airing", anime: [], filter: "topAiring" },
    });

    expect(screen.queryByRole("heading")).toBeNull();
  });

  it("shows the block heading", () => {
    render(ListBlock, {
      props: {
        title: "Top airing",
        anime: [anime(1, "A")],
        filter: "topAiring",
      },
    });

    expect(
      screen.getByRole("heading", { name: "Top airing" }),
    ).toBeInTheDocument();
  });

  it("renders one row per entry", () => {
    render(ListBlock, {
      props: {
        title: "Top airing",
        anime: [anime(1, "A"), anime(2, "B"), anime(3, "C")],
        filter: "topAiring",
      },
    });

    // Three detail links plus the "View more" link.
    expect(screen.getAllByRole("link")).toHaveLength(4);
  });

  it("links View more to the browse route for its filter", () => {
    render(ListBlock, {
      props: {
        title: "Top airing",
        anime: [anime(1, "A")],
        filter: "topAiring",
      },
    });

    expect(screen.getByRole("link", { name: /view more/i })).toHaveAttribute(
      "href",
      "/browse/topAiring",
    );
  });

  it("builds the View more href from the filter, not the title", () => {
    // The label is prose; the filter is the wire value. Mixing them up would
    // produce a URL the backend cannot deserialise.
    render(ListBlock, {
      props: {
        title: "Latest completed",
        anime: [anime(1, "A")],
        filter: "latestCompleted",
      },
    });

    expect(screen.getByRole("link", { name: /view more/i })).toHaveAttribute(
      "href",
      "/browse/latestCompleted",
    );
  });
});