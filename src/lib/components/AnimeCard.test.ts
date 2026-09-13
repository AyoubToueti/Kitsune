import { describe, it, expect } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

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

/** The preview, if one is currently shown. */
function preview(): HTMLElement | null {
  return screen.queryByTestId("hover-preview");
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

  it("shows a placeholder when there is no cover", () => {
    render(AnimeCard, { props: { anime: anime() } });

    expect(screen.getByText("No cover")).toBeInTheDocument();
  });

  it("renders the cover as decorative, since the link names the title", () => {
    const { container } = render(AnimeCard, {
      props: { anime: anime({ coverImage: "https://example.test/c.jpg" }) },
    });

    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      "https://example.test/c.jpg",
    );
    expect(container.querySelector("img")).toHaveAttribute("alt", "");
  });

  // --- hover preview ------------------------------------------------------

  it("shows no preview until hovered", () => {
    render(AnimeCard, { props: { anime: anime() } });

    expect(preview()).toBeNull();
  });

  it("shows a preview on hover", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(screen.getByRole("link", { name: "One Piece" }));

    expect(preview()).toBeInTheDocument();
  });

  it("blurs the poster and shows a play affordance on hover", () => {
    const { container } = render(AnimeCard, {
      props: { anime: anime({ coverImage: "https://example.test/c.jpg" }) },
    });

    // The blur and the overlay are pure CSS, so the classes that drive them
    // are what can be asserted.
    expect(container.querySelector("img")?.className).toMatch(
      /group-hover:blur/,
    );
    expect(screen.getByTestId("play-overlay").className).toMatch(
      /group-hover:opacity-100/,
    );
  });

  it("keeps the preview open while the pointer is over it", async () => {
    render(AnimeCard, { props: { anime: anime() } });
    const link = screen.getByRole("link", { name: "One Piece" });

    await fireEvent.mouseEnter(link);
    const card = preview()!;

    // Leaving the card for the preview: the close is scheduled, then cancelled.
    await fireEvent.mouseLeave(link);
    await fireEvent.mouseEnter(card);

    // Still open after the grace period would have elapsed.
    await new Promise((r) => setTimeout(r, 200));
    expect(preview()).toBeInTheDocument();
  });

  it("closes once the pointer leaves the preview too", async () => {
    render(AnimeCard, { props: { anime: anime() } });
    const link = screen.getByRole("link", { name: "One Piece" });

    await fireEvent.mouseEnter(link);
    const card = preview()!;
    await fireEvent.mouseLeave(link);
    await fireEvent.mouseEnter(card);

    await fireEvent.mouseLeave(card);

    await waitFor(() => expect(preview()).toBeNull());
  });

  it("closes after leaving the card", async () => {
    render(AnimeCard, { props: { anime: anime() } });
    const link = screen.getByRole("link", { name: "One Piece" });

    await fireEvent.mouseEnter(link);
    await fireEvent.mouseLeave(link);

    await waitFor(() => expect(preview()).toBeNull());
  });

  it("opens on keyboard focus, not only on hover", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    // A pointer-only preview would be unreachable by keyboard.
    await fireEvent.focusIn(screen.getByRole("link", { name: "One Piece" }));

    expect(preview()).toBeInTheDocument();
  });

  it("shows the preview's facts and a details link", async () => {
    render(AnimeCard, {
      props: {
        anime: anime({ episodeCount: 1100, format: "TV", seasonYear: 1999 }),
      },
    });

    await fireEvent.mouseEnter(screen.getByRole("link", { name: "One Piece" }));

    const card = preview()!;
    expect(card).toHaveTextContent("1100 eps");
    expect(card).toHaveTextContent("TV");
    expect(card).toHaveTextContent("1999");
    expect(
      screen.getByRole("link", { name: /view details/i }),
    ).toHaveAttribute("href", "/anime/21");
  });

  it("links each genre in the preview to its browse route", async () => {
    render(AnimeCard, {
      props: { anime: anime({ genres: ["Slice of Life"] }) },
    });

    await fireEvent.mouseEnter(screen.getByRole("link", { name: "One Piece" }));

    expect(screen.getByRole("link", { name: "Slice of Life" })).toHaveAttribute(
      "href",
      "/genre/Slice%20of%20Life",
    );
  });

  it("strips HTML from the preview's synopsis", async () => {
    render(AnimeCard, {
      props: { anime: anime({ description: "a<br>b" }) },
    });

    await fireEvent.mouseEnter(screen.getByRole("link", { name: "One Piece" }));

    expect(preview()).toHaveTextContent("a b");
  });

  it("positions the preview from the poster's rect", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(screen.getByRole("link", { name: "One Piece" }));

    // jsdom reports a zero rect, so the preview lands at the left margin
    // rather than off-screen. The exact number is hover.test.ts's concern.
    expect(preview()!.style.left).toBeTruthy();
    expect(preview()!.style.top).toBeTruthy();
  });
});