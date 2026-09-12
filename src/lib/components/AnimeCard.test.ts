import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import AnimeCard from "./AnimeCard.svelte";
import type { Anime } from "$lib/types";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: [],
    streamingEpisodes: [],
    ...overrides,
  };
}

describe("AnimeCard", () => {
  it("shows the title", () => {
    render(AnimeCard, { props: { anime: anime() } });

    expect(screen.getByText("One Piece")).toBeInTheDocument();
  });

  it("links to the detail route for its id", () => {
    render(AnimeCard, { props: { anime: anime({ id: 42 }) } });

    expect(screen.getByRole("link", { name: "One Piece" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("prefers the english title", () => {
    render(AnimeCard, {
      props: { anime: anime({ title: { romaji: "Romaji", english: "English" } }) },
    });

    expect(screen.getByText("English")).toBeInTheDocument();
  });

  it("falls back to Untitled when no title form is present", () => {
    render(AnimeCard, { props: { anime: anime({ title: {} }) } });

    expect(screen.getByText("Untitled")).toBeInTheDocument();
  });

  it("shows the score when there is one", () => {
    render(AnimeCard, { props: { anime: anime({ averageScore: 88 }) } });

    expect(screen.getByText("88")).toBeInTheDocument();
  });

  it("omits the score badge when absent", () => {
    render(AnimeCard, { props: { anime: anime() } });

    // Nothing in the 0-100 range should be rendered.
    expect(screen.queryByText("0")).not.toBeInTheDocument();
  });

  it("shows a placeholder when there is no cover", () => {
    render(AnimeCard, { props: { anime: anime() } });

    expect(screen.getByText("No cover")).toBeInTheDocument();
  });

  it("renders the cover image when present", () => {
    render(AnimeCard, {
      props: { anime: anime({ coverImage: "https://example.test/c.jpg" }) },
    });

    // Decorative alt: the link already carries the accessible name.
    const img = document.querySelector("img");
    expect(img).toHaveAttribute("src", "https://example.test/c.jpg");
  });
});