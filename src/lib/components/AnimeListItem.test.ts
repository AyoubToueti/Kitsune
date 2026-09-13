import { describe, it, expect } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

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

  // --- hover preview ------------------------------------------------------

  it("shows the same preview as a poster card", async () => {
    render(AnimeListItem, { props: { anime: anime() } });

    await fireEvent.mouseEnter(screen.getByTestId("thumb"));

    expect(screen.getByTestId("hover-preview")).toBeInTheDocument();
  });

  it("does not open when the pointer is over the title, only the thumbnail", async () => {
    render(AnimeListItem, { props: { anime: anime() } });

    await fireEvent.mouseEnter(screen.getByRole("link", { name: "One Piece" }));

    expect(screen.queryByTestId("hover-preview")).toBeNull();
  });

  it("opens on keyboard focus, not only on hover", async () => {
    render(AnimeListItem, { props: { anime: anime() } });

    await fireEvent.focusIn(screen.getByRole("link", { name: "One Piece" }));

    expect(screen.getByTestId("hover-preview")).toBeInTheDocument();
  });

  it("carries the preview's facts", async () => {
    render(AnimeListItem, {
      props: { anime: anime({ episodeCount: 1100, format: "TV" }) },
    });

    await fireEvent.mouseEnter(screen.getByTestId("thumb"));

    expect(screen.getByTestId("hover-preview")).toHaveTextContent("1100 eps");
  });

  it("closes after leaving", async () => {
    render(AnimeListItem, { props: { anime: anime() } });
    const thumb = screen.getByTestId("thumb");

    await fireEvent.mouseEnter(thumb);
    // Asserted open first: without this the close assertion below would pass
    // even if the preview never rendered at all.
    expect(screen.getByTestId("hover-preview")).toBeInTheDocument();

    await fireEvent.mouseLeave(thumb);

    await waitFor(() => expect(screen.queryByTestId("hover-preview")).toBeNull());
  });
});