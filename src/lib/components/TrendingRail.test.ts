import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";

import TrendingRail from "./TrendingRail.svelte";
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

/** The rank badges, in DOM order. */
function ranks(): string[] {
  return Array.from(document.querySelectorAll("[data-testid='rank-badge']")).map(
    (el) => el.textContent?.trim() ?? "",
  );
}

describe("TrendingRail", () => {
  it("renders nothing for an empty list", () => {
    render(TrendingRail, { props: { anime: [] } });

    expect(screen.queryByRole("heading")).toBeNull();
  });

  it("shows the section heading", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    expect(screen.getByRole("heading", { name: "Trending" })).toBeInTheDocument();
  });

  it("renders one card per entry", () => {
    render(TrendingRail, {
      props: { anime: [anime(1, "A"), anime(2, "B"), anime(3, "C")] },
    });

    expect(screen.getAllByRole("link")).toHaveLength(3);
  });

  it("links each entry to its detail route", () => {
    render(TrendingRail, { props: { anime: [anime(42, "A")] } });

    expect(screen.getByRole("link", { name: "A" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("numbers entries from 01, zero-padded", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A"), anime(2, "B")] } });

    expect(ranks()).toEqual(["01", "02"]);
  });

  it("keeps ranks aligned past nine", () => {
    const many = Array.from({ length: 12 }, (_, i) => anime(i + 1, `T${i + 1}`));

    render(TrendingRail, { props: { anime: many } });

    // Both ends matter: single digits must stay padded, and two-digit ranks
    // must not be truncated. Asserting only "10" would pass with or without
    // padding, since String(10) is already two characters.
    expect(ranks()[0]).toBe("01");
    expect(ranks()[8]).toBe("09");
    expect(ranks()[9]).toBe("10");
    expect(ranks()[11]).toBe("12");
  });

  it("reads the title up the edge, vertically", () => {
    render(TrendingRail, { props: { anime: [anime(1, "Solo Leveling")] } });

    // The rail's vertical title is its defining shape, so it is asserted
    // rather than left to the stylesheet.
    const title = screen.getByText("Solo Leveling");
    expect(title.style.writingMode).toBe("vertical-rl");
    expect(title.style.transform).toContain("rotate(180deg)");
  });

  it("sets the rank number over the poster, not rotated", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    const badge = document.querySelector(
      "[data-testid='rank-badge']",
    ) as HTMLElement;
    // Horizontal number over the artwork, unlike the old vertical badge.
    expect(badge.style.writingMode).toBe("");
    expect(badge.textContent?.trim()).toBe("01");
  });

  it("falls back to Untitled when no title form is present", () => {
    render(TrendingRail, { props: { anime: [{ ...anime(1, "x"), title: {} }] } });

    expect(screen.getByRole("link", { name: "Untitled" })).toBeInTheDocument();
  });

  it("shows a placeholder rather than a broken image when there is no cover", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    expect(screen.getByText("No cover")).toBeInTheDocument();
  });

  // --- scroll treatment --------------------------------------------------

  it("renders the scroll nav buttons", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    expect(screen.getByTestId("trending-prev")).toBeInTheDocument();
    expect(screen.getByTestId("trending-next")).toBeInTheDocument();
  });

  it("disables both nav buttons when the rail is not measured", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    // jsdom lays nothing out, so the rail reads as fitting and there is
    // nowhere to go in either direction.
    expect(screen.getByTestId("trending-prev")).toBeDisabled();
    expect(screen.getByTestId("trending-next")).toBeDisabled();
  });

  it("marks the rail for hidden-scrollbar snapping", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    const rail = screen.getByTestId("trending-rail");
    expect(rail.className).toContain("no-scrollbar");
    expect(rail.className).toContain("snap-strip");
    // Without this the flex row grows the page instead of scrolling itself.
    expect(rail.className).toContain("overflow-x-auto");
  });

  it("gives each slide the same width once measured", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A"), anime(2, "B")] } });

    const rail = screen.getByTestId("trending-rail");
    Object.defineProperty(rail, "clientWidth", { value: 800, configurable: true });
    Object.defineProperty(rail, "scrollWidth", { value: 2000, configurable: true });
    window.dispatchEvent(new Event("resize"));
    flushSync();

    const widths = Array.from(rail.querySelectorAll("li")).map((li) =>
      Number.parseFloat((li as HTMLElement).style.width),
    );
    expect(new Set(widths).size).toBe(1);
    // At 800px the rail table says 4 slides, gap 16: 4 * w + 3 * 16 === 800.
    expect(widths[0] * 4 + 16 * 3).toBeCloseTo(800, 5);
  });
});