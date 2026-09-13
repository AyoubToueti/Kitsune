import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import HeroCarousel from "./HeroCarousel.svelte";
import type { Anime } from "$lib/types";

function anime(id: number, title: string, overrides: Partial<Anime> = {}): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
    ...overrides,
  };
}

const THREE = [anime(1, "First"), anime(2, "Second"), anime(3, "Third")];

/** The element that actually slides. */
function track(): HTMLElement {
  return screen.getByTestId("carousel-track");
}

/** Every slide, in DOM order. */
function slides(): HTMLElement[] {
  return Array.from(track().children) as HTMLElement[];
}

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

  // --- fixed height -------------------------------------------------------

  it("gives the container a fixed height so slides cannot resize it", () => {
    const { container } = render(HeroCarousel, { props: { anime: THREE } });
    const section = container.querySelector("section")!;

    // A fixed height plus overflow-hidden is what stops a long synopsis from
    // making one slide taller than the rest.
    expect(section.className).toMatch(/\bh-\[/);
    expect(section.className).toMatch(/overflow-hidden/);
  });

  it("clamps the synopsis rather than letting it grow the slide", () => {
    const long = "word ".repeat(400);
    render(HeroCarousel, {
      props: { anime: [anime(1, "First", { description: long })] },
    });

    const synopsis = screen.getByTestId("hero-synopsis");
    expect(synopsis.style.webkitLineClamp).toBe("3");
    expect(synopsis.style.overflow).toBe("hidden");
  });

  // --- slide animation ----------------------------------------------------

  it("lays slides out side by side so the track can slide", () => {
    render(HeroCarousel, { props: { anime: THREE } });

    expect(track().className).toMatch(/\bflex\b/);
    expect(slides()).toHaveLength(3);
    // Each slide is full width and must not shrink, or they would squash up
    // instead of sitting off-screen to the right.
    for (const slide of slides()) {
      expect(slide.className).toMatch(/w-full/);
      expect(slide.className).toMatch(/shrink-0/);
    }
  });

  it("translates the track to bring a slide into view", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    expect(track().style.transform).toBe("translateX(-0%)");

    await fireEvent.click(screen.getByRole("button", { name: /next featured/i }));

    // One slide width across, which is the slide effect.
    expect(track().style.transform).toBe("translateX(-100%)");
  });

  it("animates the movement, not an instant jump", () => {
    render(HeroCarousel, { props: { anime: THREE } });

    expect(track().className).toMatch(/transition-transform/);
  });

  it("drops the transition when reduced motion is requested", () => {
    stubMatchMedia(true);
    render(HeroCarousel, { props: { anime: THREE } });

    expect(track().className).not.toMatch(/transition-transform/);
  });

  it("hides inactive slides from assistive tech", () => {
    render(HeroCarousel, { props: { anime: THREE } });

    const [first, second] = slides();
    expect(first).not.toHaveAttribute("aria-hidden");
    expect(second).toHaveAttribute("aria-hidden", "true");
  });

  // --- navigation ---------------------------------------------------------

  it("advances to the next title after the interval", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    await vi.advanceTimersByTimeAsync(7000);

    expect(track().style.transform).toBe("translateX(-100%)");
  });

  it("wraps from the last title back to the first", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    // Start on the LAST slide, so a wrap is the only way to reach the first.
    // Advancing from slide one would land on "First" either way, making the
    // assertion pass even if the timer never fired.
    await fireEvent.click(
      screen.getByRole("button", { name: /show featured title 3 of 3/i }),
    );
    expect(track().style.transform).toBe("translateX(-200%)");

    await vi.advanceTimersByTimeAsync(7000);

    expect(track().style.transform).toBe("translateX(-0%)");
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

    expect(track().style.transform).toBe("translateX(-200%)");
  });

  it("a dot jumps straight to its slide", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    await fireEvent.click(
      screen.getByRole("button", { name: /show featured title 3 of 3/i }),
    );

    expect(track().style.transform).toBe("translateX(-200%)");
  });

  it("marks the active dot for assistive tech", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    expect(
      screen.getByRole("button", { name: /show featured title 1 of 3/i }),
    ).toHaveAttribute("aria-current", "true");
  });

  it("pauses auto-advance while the pointer is over it", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    // Queried by role rather than by attribute: a labelled <section> is a
    // region implicitly, so this also proves the accessible name is wired up.
    await fireEvent.mouseEnter(screen.getByRole("region", { name: /featured/i }));
    await vi.advanceTimersByTimeAsync(21_000);

    // Still on the first slide: hovering suppresses the timer entirely.
    expect(track().style.transform).toBe("translateX(-0%)");
  });

  it("resumes auto-advance when the pointer leaves", async () => {
    render(HeroCarousel, { props: { anime: THREE } });
    const region = screen.getByRole("region", { name: /featured/i });

    await fireEvent.mouseEnter(region);
    await fireEvent.mouseLeave(region);
    await vi.advanceTimersByTimeAsync(7000);

    expect(track().style.transform).toBe("translateX(-100%)");
  });

  it("does not auto-advance when reduced motion is requested", async () => {
    stubMatchMedia(true);
    render(HeroCarousel, { props: { anime: THREE } });

    await vi.advanceTimersByTimeAsync(21_000);

    // Controls still work; only the automatic movement is suppressed.
    expect(track().style.transform).toBe("translateX(-0%)");
  });

  it("hides the controls for a single title", () => {
    render(HeroCarousel, { props: { anime: [anime(1, "Only")] } });

    expect(screen.queryByRole("button", { name: /next featured/i })).toBeNull();
  });

  // --- content ------------------------------------------------------------

  it("falls back to the cover when there is no banner", () => {
    const { container } = render(HeroCarousel, {
      props: {
        anime: [anime(1, "First", { coverImage: "https://example.test/c.jpg" })],
      },
    });

    // The artwork is decorative (`alt=""`), which gives it a presentational
    // role, so it is found by DOM query rather than by role.
    const img = container.querySelector("img");
    expect(img).toHaveAttribute("src", "https://example.test/c.jpg");
  });

  it("prefers the banner over the cover when both are present", () => {
    const { container } = render(HeroCarousel, {
      props: {
        anime: [
          anime(1, "First", {
            bannerImage: "https://example.test/banner.jpg",
            coverImage: "https://example.test/cover.jpg",
          }),
        ],
      },
    });

    expect(container.querySelector("img")).toHaveAttribute(
      "src",
      "https://example.test/banner.jpg",
    );
  });

  it("shows format, duration and year as facts", () => {
    render(HeroCarousel, {
      props: {
        anime: [
          anime(1, "First", {
            format: "TV",
            durationMinutes: 24,
            seasonYear: 2013,
          }),
        ],
      },
    });

    expect(screen.getByText("TV")).toBeInTheDocument();
    expect(screen.getByText("24m")).toBeInTheDocument();
    expect(screen.getByText("2013")).toBeInTheDocument();
  });

  it("strips HTML from the synopsis rather than rendering it", () => {
    render(HeroCarousel, {
      props: { anime: [anime(1, "First", { description: "a<br>b" })] },
    });

    expect(screen.getByTestId("hero-synopsis")).toHaveTextContent("a b");
  });

  it("links to the detail route for the visible slide", () => {
    render(HeroCarousel, { props: { anime: THREE } });

    // Only the active slide is exposed: the others are inert and aria-hidden,
    // so they are absent from the accessibility tree rather than merely
    // off-screen. This is what stops a screen reader reading every slide at
    // once, or Tab walking into a hidden one.
    const links = screen.getAllByRole("link", { name: /view details/i });
    expect(links).toHaveLength(1);
    expect(links[0]).toHaveAttribute("href", "/anime/1");
  });

  it("exposes the next slide's link once it becomes active", async () => {
    render(HeroCarousel, { props: { anime: THREE } });

    await fireEvent.click(screen.getByRole("button", { name: /next featured/i }));

    const links = screen.getAllByRole("link", { name: /view details/i });
    expect(links).toHaveLength(1);
    expect(links[0]).toHaveAttribute("href", "/anime/2");
  });
});