import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import type { RelatedAnime } from "$lib/types";

import RelatedAnimeList from "./RelatedAnimeList.svelte";

function related(overrides: Partial<RelatedAnime> = {}): RelatedAnime {
  return {
    id: 865,
    title: { romaji: "Attack on Titan Season 2" },
    coverImage: "https://example.test/s2.jpg",
    format: "TV",
    episodeCount: 12,
    relationType: "SEQUEL",
    ...overrides,
  };
}

describe("RelatedAnimeList", () => {
  it("renders one row per relation", () => {
    render(RelatedAnimeList, {
      props: {
        relations: [
          related({ id: 865, title: { romaji: "Season 2" } }),
          related({ id: 866, title: { romaji: "Season 3" }, relationType: "SEQUEL" }),
        ],
      },
    });

    expect(screen.getAllByRole("link")).toHaveLength(2);
  });

  it("links each row to its anime detail route", () => {
    render(RelatedAnimeList, {
      props: { relations: [related({ id: 866, title: { romaji: "Season 3" } })] },
    });

    expect(screen.getByRole("link")).toHaveAttribute("href", "/anime/866");
  });

  it("shows the title", () => {
    render(RelatedAnimeList, {
      props: { relations: [related({ title: { romaji: "Attack on Titan Season 2" } })] },
    });

    expect(screen.getByText("Attack on Titan Season 2")).toBeInTheDocument();
  });

  it("renders the relation type as a readable label", () => {
    render(RelatedAnimeList, {
      props: { relations: [related({ relationType: "SIDE_STORY" })] },
    });

    // SCREAMING_SNAKE becomes words.
    expect(screen.getByText("Side Story")).toBeInTheDocument();
  });

  it("renders a single-word relation type unchanged apart from case", () => {
    render(RelatedAnimeList, {
      props: { relations: [related({ relationType: "PREQUEL" })] },
    });

    expect(screen.getByText("Prequel")).toBeInTheDocument();
  });

  it("shows the episode count when present", () => {
    render(RelatedAnimeList, {
      props: { relations: [related({ episodeCount: 25 })] },
    });

    expect(screen.getByText("25")).toBeInTheDocument();
  });

  it("renders the format", () => {
    render(RelatedAnimeList, {
      props: { relations: [related({ format: "MOVIE" })] },
    });

    expect(screen.getByText("MOVIE")).toBeInTheDocument();
  });

  it("falls back to Untitled when every title form is blank", () => {
    render(RelatedAnimeList, {
      props: { relations: [related({ title: {} })] },
    });

    expect(screen.getByText("Untitled")).toBeInTheDocument();
  });

  it("renders nothing when the list is empty", () => {
    const { container } = render(RelatedAnimeList, { props: { relations: [] } });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /related/i })).toBeNull();
  });

  it("renders nothing when the prop is undefined", () => {
    const { container } = render(RelatedAnimeList, { props: {} });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
  });

  it("caps the list in a three-row scrollable container", () => {
    render(RelatedAnimeList, { props: { relations: [related()] } });

    const scroller = screen.getByTestId("related-scroller");
    expect(scroller.className).toContain("overflow-y-auto");
    // 3 rows × 88px + 2 gaps × 8px = 280px = 17.5rem.
    expect(scroller.className).toContain("max-h-[17.5rem]");
  });
});
