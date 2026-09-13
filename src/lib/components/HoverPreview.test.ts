import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import HoverPreview from "./HoverPreview.svelte";
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

const base = { x: 10, y: 20, onenter: () => {}, onleave: () => {} };

describe("HoverPreview", () => {
  it("shows the title", () => {
    render(HoverPreview, { props: { ...base, anime: anime() } });

    expect(screen.getByText("One Piece")).toBeInTheDocument();
  });

  it("falls back to Untitled when no title form is present", () => {
    render(HoverPreview, { props: { ...base, anime: anime({ title: {} }) } });

    expect(screen.getByText("Untitled")).toBeInTheDocument();
  });

  it("is exposed as a tooltip", () => {
    render(HoverPreview, { props: { ...base, anime: anime() } });

    expect(screen.getByRole("tooltip")).toBeInTheDocument();
  });

  it("sits at the position it is given", () => {
    render(HoverPreview, { props: { ...base, anime: anime() } });

    const el = screen.getByRole("tooltip");
    expect(el.style.left).toBe("10px");
    expect(el.style.top).toBe("20px");
  });

  it("shows the score when present", () => {
    render(HoverPreview, {
      props: { ...base, anime: anime({ averageScore: 88 }) },
    });

    expect(screen.getByText("88")).toBeInTheDocument();
  });

  it("shows episode count and duration as facts", () => {
    render(HoverPreview, {
      props: { ...base, anime: anime({ episodeCount: 1100, durationMinutes: 24 }) },
    });

    expect(screen.getByText("1100 eps")).toBeInTheDocument();
    expect(screen.getByText("24m")).toBeInTheDocument();
  });

  it("omits a fact the provider did not supply", () => {
    render(HoverPreview, { props: { ...base, anime: anime({ format: "TV" }) } });

    expect(screen.queryByText(/eps/)).toBeNull();
    expect(screen.queryByText(/^24m$/)).toBeNull();
  });

  it("shows the native title, year and status", () => {
    render(HoverPreview, {
      props: {
        ...base,
        anime: anime({
          title: { romaji: "One Piece", native: "ワンピース" },
          seasonYear: 1999,
          status: "RELEASING",
        }),
      },
    });

    expect(screen.getByText("ワンピース")).toBeInTheDocument();
    expect(screen.getByText("1999")).toBeInTheDocument();
    expect(screen.getByText("RELEASING")).toBeInTheDocument();
  });

  it("strips HTML from the synopsis", () => {
    render(HoverPreview, {
      props: { ...base, anime: anime({ description: "a<br>b" }) },
    });

    expect(screen.getByText("a b")).toBeInTheDocument();
  });

  it("links each genre to its browse route, encoding spaces", () => {
    render(HoverPreview, {
      props: { ...base, anime: anime({ genres: ["Slice of Life"] }) },
    });

    expect(screen.getByRole("link", { name: "Slice of Life" })).toHaveAttribute(
      "href",
      "/genre/Slice%20of%20Life",
    );
  });

  it("caps the genre list so it cannot wrap indefinitely", () => {
    render(HoverPreview, {
      props: {
        ...base,
        anime: anime({ genres: ["A", "B", "C", "D", "E", "F", "G"] }),
      },
    });

    // Five shown, the rest dropped.
    expect(screen.getByRole("link", { name: "E" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "F" })).toBeNull();
    expect(screen.queryByRole("link", { name: "G" })).toBeNull();
  });

  it("links to the detail route", () => {
    render(HoverPreview, { props: { ...base, anime: anime({ id: 42 }) } });

    expect(screen.getByRole("link", { name: /view details/i })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("reports pointer entry so the card can cancel its close", async () => {
    const onenter = vi.fn();
    render(HoverPreview, {
      props: { ...base, anime: anime(), onenter },
    });

    await fireEvent.mouseEnter(screen.getByRole("tooltip"));

    expect(onenter).toHaveBeenCalledTimes(1);
  });

  it("reports pointer exit so the card can schedule a close", async () => {
    const onleave = vi.fn();
    render(HoverPreview, {
      props: { ...base, anime: anime(), onleave },
    });

    await fireEvent.mouseLeave(screen.getByRole("tooltip"));

    expect(onleave).toHaveBeenCalledTimes(1);
  });
});