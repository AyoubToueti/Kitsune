import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import TrendingRail from "./TrendingRail.svelte";
import type { Anime } from "$lib/types";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
  };
}

/** The rank badges, in DOM order. */
function ranks(): string[] {
  return Array.from(document.querySelectorAll("[data-testid='rail-title']"))
    .map((p) => p.parentElement?.parentElement?.querySelector("span")?.textContent)
    .map((t) => (t ?? "").trim());
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

  it("renders one link per entry", () => {
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
    render(TrendingRail, {
      props: { anime: [anime(1, "A"), anime(2, "B")] },
    });

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

  it("rotates the rank so it reads up the edge", () => {
    const { container } = render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    // The vertical presentation is the whole point of this rail, so it is
    // asserted rather than left to the stylesheet.
    const badge = container.querySelector("span[style]") as HTMLElement;
    expect(badge.style.writingMode).toBe("vertical-rl");
    expect(badge.style.transform).toContain("rotate(180deg)");
  });

  it("falls back to Untitled when no title form is present", () => {
    render(TrendingRail, { props: { anime: [{ ...anime(1, "x"), title: {} }] } });

    expect(screen.getByRole("link", { name: "Untitled" })).toBeInTheDocument();
  });

  it("shows a placeholder rather than a broken image when there is no cover", () => {
    render(TrendingRail, { props: { anime: [anime(1, "A")] } });

    expect(screen.getByText("No cover")).toBeInTheDocument();
  });
});