import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import HeroCarousel from "./HeroCarousel.svelte";
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

const THREE = [anime(1, "First"), anime(2, "Second"), anime(3, "Third")];

/**
 * jsdom does not implement `matchMedia`, and the component guards for that.
 * These tests stub it so the reduced-motion branch can be exercised.
 */
function stubMatchMedia(matches: boolean) {
  window.matchMedia = vi.fn().mockReturnValue({
    matches,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }) as unknown as typeof window.matchMedia;
}

describe("HeroCarousel", () => {
  beforeEach(() => {
    stubMatchMedia(false);
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    // @ts-expect-error the stub is test-only and may not be installed
    delete window.matchMedia;
  });

  it("shows the first title initially", () => {
    render(HeroCarousel, { props: { anime: THREE } });

    expect(
      screen.getByRole("heading", { name: "First", level: 1 }),
    ).toBeInTheDocument();
  });

  it("renders nothing for an empty list", () => {
    const { container } = render(HeroCarousel, { props: { anime: [] } });

    expect(container.querySelector("section")).toBeNull();
  });

  it("advances to the next title after the interval", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    await vi.advanceTimersByTimeAsync(7000);

    expect(
      screen.getByRole("heading", { name: "Second", level: 1 }),
    ).toBeInTheDocument();
  });

  it("wraps from the last title back to the first", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    // Start on the LAST slide, so a wrap is the only way to reach the first.
    // Advancing from slide one would land on "First" either way, making the
    // assertion pass even if the timer never fired.
    await fireEvent.click(
      screen.getByRole("button", { name: /show featured title 3 of 3/i }),
    );
    await vi.advanceTimersByTimeAsync(7000);

    expect(
      screen.getByRole("heading", { name: "First", level: 1 }),
    ).toBeInTheDocument();
  });

  it("the next button advances immediately", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    await fireEvent.click(screen.getByRole("button", { name: /next featured/i }));

    expect(
      screen.getByRole("heading", { name: "Second", level: 1 }),
    ).toBeInTheDocument();
  });

  it("the previous button wraps backwards", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    await fireEvent.click(screen.getByRole("button", { name: /previous featured/i }));

    expect(
      screen.getByRole("heading", { name: "Third", level: 1 }),
    ).toBeInTheDocument();
  });

  it("a dot jumps straight to its slide", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    await fireEvent.click(
      screen.getByRole("button", { name: /show featured title 3 of 3/i }),
    );

    expect(
      screen.getByRole("heading", { name: "Third", level: 1 }),
    ).toBeInTheDocument();
  });

  it("marks the active dot for assistive tech", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    expect(
      screen.getByRole("button", { name: /show featured title 1 of 3/i }),
    ).toHaveAttribute("aria-current", "true");
  });

  it("pauses auto-advance while the pointer is over it", async () => {
    const { container } = render(HeroCarousel, { props: { anime: THREE } });
    const region = container.querySelector("[role='region']")!;

    await fireEvent.mouseEnter(region);
    await vi.advanceTimersByTimeAsync(21_000);

    // Still on the first slide: hovering suppresses the timer entirely.
    expect(
      screen.getByRole("heading", { name: "First", level: 1 }),
    ).toBeInTheDocument();
  });

  it("resumes auto-advance when the pointer leaves", async () => {
    const { container } = render(HeroCarousel, { props: { anime: THREE } });
    const region = container.querySelector("[role='region']")!;

    await fireEvent.mouseEnter(region);
    await fireEvent.mouseLeave(region);
    await vi.advanceTimersByTimeAsync(7000);

    expect(
      screen.getByRole("heading", { name: "Second", level: 1 }),
    ).toBeInTheDocument();
  });

  it("does not auto-advance when reduced motion is requested", async () => {
    stubMatchMedia(true);
    render(HeroCarousel, { props: { anime: THREE } });

    await vi.advanceTimersByTimeAsync(21_000);

    // Controls still work; only the automatic movement is suppressed.
    expect(
      screen.getByRole("heading", { name: "First", level: 1 }),
    ).toBeInTheDocument();
  });

  it("hides the controls for a single title", () => {
    render(HeroCarousel, { props: { anime: [anime(1, "Only")] } });

    expect(screen.queryByRole("button", { name: /next featured/i })).toBeNull();
  });
});