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

const base = {
  placement: { x: 10, y: 20, side: "bottom" as const, caretX: 60 },
  // A no-op stand-in: the real action is supplied by the hover controller,
  // which AnimeCard's tests cover.
  measure: () => ({ destroy: () => {} }),
  onenter: () => {},
  onleave: () => {},
};

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

  // --- caret --------------------------------------------------------------

  it("renders a caret", () => {
    render(HoverPreview, { props: { ...base, anime: anime() } });

    expect(screen.getByTestId("hover-caret")).toBeInTheDocument();
  });

  it("places the caret at the given offset", () => {
    render(HoverPreview, {
      props: {
        ...base,
        anime: anime(),
        placement: { ...base.placement, caretX: 80 },
      },
    });

    // Offset is the centre, so the element starts half a caret earlier.
    expect(screen.getByTestId("hover-caret").style.left).toBe("74px");
  });

  it("points the caret down when the preview is above the card", () => {
    render(HoverPreview, {
      props: {
        ...base,
        anime: anime(),
        placement: { ...base.placement, side: "bottom" },
      },
    });

    const caret = screen.getByTestId("hover-caret");
    expect(caret.className).toMatch(/-bottom/);
    expect(screen.getByRole("tooltip")).toHaveAttribute("data-side", "bottom");
  });

  it("points the caret up when the preview flipped below", () => {
    render(HoverPreview, {
      props: {
        ...base,
        anime: anime(),
        placement: { ...base.placement, side: "top" },
      },
    });

    const caret = screen.getByTestId("hover-caret");
    expect(caret.className).toMatch(/-top/);
    expect(screen.getByRole("tooltip")).toHaveAttribute("data-side", "top");
  });

  // --- content ------------------------------------------------------------

  it("shows the score when present", () => {
    render(HoverPreview, {
      props: { ...base, anime: anime({ averageScore: 88 }) },
    });

    expect(screen.getByText("88")).toBeInTheDocument();
  });

  it("shows episode count and duration as facts", () => {
    render(HoverPreview, {
      props: {
        ...base,
        anime: anime({ episodeCount: 1100, durationMinutes: 24 }),
      },
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

  // --- pointer reporting --------------------------------------------------

  it("reports pointer entry so the card can cancel its close", async () => {
    const onenter = vi.fn();
    render(HoverPreview, { props: { ...base, anime: anime(), onenter } });

    await fireEvent.mouseEnter(screen.getByRole("tooltip"));

    expect(onenter).toHaveBeenCalledTimes(1);
  });

  it("reports pointer exit so the card can schedule a close", async () => {
    const onleave = vi.fn();
    render(HoverPreview, { props: { ...base, anime: anime(), onleave } });

    await fireEvent.mouseLeave(screen.getByRole("tooltip"));

    expect(onleave).toHaveBeenCalledTimes(1);
  });

  // --- measuring ----------------------------------------------------------

  it("hands its element to the measure action", () => {
    // The height drives the vertical offset, and an estimate left a visible
    // gap between the caret and the card. Without this the panel would never
    // be measured.
    // Explicitly typed: a zero-arg mock makes `calls[0][0]` a type error.
    const measure = vi.fn((_node: HTMLElement) => ({ destroy: () => {} }));

    render(HoverPreview, { props: { ...base, anime: anime(), measure } });

    expect(measure).toHaveBeenCalledTimes(1);
    expect(measure.mock.calls[0][0]).toBe(screen.getByRole("tooltip"));
  });
});