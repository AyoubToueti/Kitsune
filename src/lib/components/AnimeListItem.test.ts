import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import AnimeListItem from "./AnimeListItem.svelte";
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

describe("AnimeListItem", () => {
  it("shows the title", () => {
    render(AnimeListItem, { props: { anime: anime() } });

    expect(screen.getByText("One Piece")).toBeInTheDocument();
  });

  it("links to the detail route", () => {
    render(AnimeListItem, { props: { anime: anime({ id: 42 }) } });

    expect(screen.getByRole("link", { name: "One Piece" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("falls back to Untitled when no title form is present", () => {
    render(AnimeListItem, { props: { anime: anime({ title: {} }) } });

    expect(screen.getByRole("link", { name: "Untitled" })).toBeInTheDocument();
  });

  it("shows the score when present", () => {
    render(AnimeListItem, { props: { anime: anime({ averageScore: 88 }) } });

    expect(screen.getByTestId("score-chip")).toHaveTextContent("88");
  });

  it("omits the score chip when absent", () => {
    render(AnimeListItem, { props: { anime: anime() } });

    expect(screen.queryByTestId("score-chip")).toBeNull();
  });

  it("shows the episode count when present", () => {
    render(AnimeListItem, { props: { anime: anime({ episodeCount: 1100 }) } });

    expect(screen.getByTestId("episodes-chip")).toHaveTextContent("1100");
  });

  it("omits the episode chip when absent", () => {
    render(AnimeListItem, { props: { anime: anime() } });

    expect(screen.queryByTestId("episodes-chip")).toBeNull();
  });

  it("shows the format when present", () => {
    render(AnimeListItem, { props: { anime: anime({ format: "TV" }) } });

    expect(screen.getByText("TV")).toBeInTheDocument();
  });

  it("omits the separator when there is no format to follow it", () => {
    render(AnimeListItem, { props: { anime: anime({ episodeCount: 12 }) } });

    // A lone "•" with nothing after it reads as a rendering bug.
    expect(screen.queryByText("•")).toBeNull();
  });

  it("shows a placeholder rather than a broken image when there is no cover", () => {
    render(AnimeListItem, { props: { anime: anime() } });

    expect(screen.getByText("No cover")).toBeInTheDocument();
  });

  it("renders the cover as decorative, since the link already names the title", () => {
    const { container } = render(AnimeListItem, {
      props: { anime: anime({ coverImage: "https://example.test/c.jpg" }) },
    });

    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      "https://example.test/c.jpg",
    );
    expect(container.querySelector("img")).toHaveAttribute("alt", "");
  });
});