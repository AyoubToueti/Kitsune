import { describe, it, expect, vi, afterEach } from "vitest";
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
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

/** The preview, if one is currently shown. */
function preview(): HTMLElement | null {
  return screen.queryByTestId("hover-preview");
}

const link = () => screen.getByRole("link", { name: "One Piece" });
/** The artwork, asserted for its own sake; it is no longer the trigger. */
const poster = () => screen.getByTestId("poster");
/** The top-left circle, which is what opens the preview. */
const trigger = () => screen.getByTestId("info-trigger");

afterEach(() => {
  vi.restoreAllMocks();
});

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

    // The badge carries a star glyph, so match on the number alone.
    expect(screen.getByText(/88/)).toBeInTheDocument();
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

  it("zooms the poster on hover", () => {
    const { container } = render(AnimeCard, {
      props: { anime: anime({ coverImage: "https://example.test/c.jpg" }) },
    });

    // The zoom is pure CSS, so the class driving it is what can be asserted.
    expect(container.querySelector("img")?.className).toMatch(
      /group-hover:scale-105/,
    );
  });

  it("renders the info trigger in the hover quick bar", () => {
    render(AnimeCard, { props: { anime: anime() } });

    const button = screen.getByTestId("info-trigger");
    // The trigger lives in the quick bar that slides up on hover, so it is a
    // sibling of the Watch link there.
    expect(button.className).not.toMatch(/opacity-0/);
    expect(button.parentElement?.className).toMatch(/translate-y-full/);
    expect(button.parentElement?.className).toMatch(/group-hover:translate-y-0/);
  });

  // --- hover preview ------------------------------------------------------

  it("shows no preview until hovered", () => {
    render(AnimeCard, { props: { anime: anime() } });

    expect(preview()).toBeNull();
  });

  it("shows a preview on hover", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(trigger());

    expect(preview()).toBeInTheDocument();
  });

  it("animates in rather than appearing instantly", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(trigger());

    // The element is created on open, so the animation class is what makes it
      // ease in. Without it the panel would blink into place. Asserted on the
      // utility rather than a bare "preview-in" substring, which both the
      // per-side animation classes would satisfy even if neither were applied.
      expect(preview()!.className).toMatch(/animate-preview-in-(bottom|top)/);
  });

  it("opens on keyboard focus of the trigger, not only on hover", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    // A pointer-only preview would be unreachable by keyboard.
    await fireEvent.focusIn(trigger());

    expect(preview()).toBeInTheDocument();
  });

  it("does not open when the pointer is over the poster", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    // The poster is no longer the trigger; only the top-left circle is.
    await fireEvent.mouseEnter(poster());

    expect(preview()).toBeNull();
  });

  it("keeps the preview open while the pointer is over it", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(trigger());
    const panel = preview()!;

    // Leaving the card for the preview: the close is scheduled, then cancelled.
    await fireEvent.mouseLeave(trigger());
    await fireEvent.mouseEnter(panel);

    // Still open after the grace period would have elapsed.
    await new Promise((r) => setTimeout(r, 220));
    expect(preview()).toBeInTheDocument();
  });

  it("closes once the pointer leaves the preview too", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(trigger());
    const panel = preview()!;
    await fireEvent.mouseLeave(trigger());
    await fireEvent.mouseEnter(panel);

    await fireEvent.mouseLeave(panel);

    await waitFor(() => expect(preview()).toBeNull());
  });

  it("stays open when the panel opens under a stationary pointer", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(trigger());
    const panel = preview()!;

    // The panel can appear under the pointer, so the browser fires the panel's
    // enter BEFORE the trigger's leave: cancel-then-schedule. That stray timer
    // used to close the panel out from under a pointer that never left it.
    await fireEvent.mouseEnter(panel);
    await fireEvent.mouseLeave(trigger());

    await new Promise((r) => setTimeout(r, 220));
    expect(preview()).toBeInTheDocument();
  });

  it("closes after leaving the card", async () => {
    render(AnimeCard, { props: { anime: anime() } });

    await fireEvent.mouseEnter(trigger());
    await fireEvent.mouseLeave(trigger());

    await waitFor(() => expect(preview()).toBeNull());
  });

  // --- following the card -------------------------------------------------

  it("follows the card when the page scrolls", async () => {
    let cardTop = 500;
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
      () =>
        ({
          left: 100,
          top: cardTop,
          right: 260,
          bottom: cardTop + 240,
          width: 160,
          height: 240,
          x: 100,
          y: cardTop,
          toJSON: () => ({}),
        }) as DOMRect,
    );

    render(AnimeCard, { props: { anime: anime() } });
    await fireEvent.mouseEnter(trigger());

    const panel = preview()!;
    const before = panel.style.top;

    // The card moves up as the page scrolls.
    cardTop = 300;
    await fireEvent.scroll(window);

    // Repositioned, rather than left where it first appeared.
    expect(panel.style.top).not.toBe(before);
  });

  it("anchors the preview above the card, overlapping it", async () => {
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
      () =>
        ({
          left: 100,
          top: 500,
          right: 260,
          bottom: 740,
          width: 160,
          height: 240,
          x: 100,
          y: 500,
          toJSON: () => ({}),
        }) as DOMRect,
    );

    render(AnimeCard, { props: { anime: anime() } });
    await fireEvent.mouseEnter(trigger());

    const panel = preview()!;
    // Left edge at the card's horizontal centre (100 + 160/2).
    expect(panel.style.left).toBe("180px");
    expect(panel).toHaveAttribute("data-side", "bottom");
  });

  it("uses the panel's real height, so the caret touches the card", async () => {
    // A panel shorter than the estimated PREVIEW_SIZE. Using the estimate
    // positioned it too high, leaving a visible gap between the caret and the
    // card -- and a dead zone that closed the preview as the pointer crossed.
    const REAL_HEIGHT = 200;
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
      configurable: true,
      get() {
        return this.getAttribute("data-testid") === "hover-preview"
          ? REAL_HEIGHT
          : 0;
      },
    });
    Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
      configurable: true,
      get() {
        return this.getAttribute("data-testid") === "hover-preview" ? 288 : 0;
      },
    });

    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
      () =>
        ({
          left: 100,
          top: 500,
          right: 260,
          bottom: 740,
          width: 160,
          height: 240,
          x: 100,
          y: 500,
          toJSON: () => ({}),
        }) as DOMRect,
    );

    render(AnimeCard, { props: { anime: anime() } });
    await fireEvent.mouseEnter(trigger());

    // Card centre Y (500 + 240/2 = 620) minus the panel's real height (200),
    // so its bottom edge meets the card's centre.
    expect(preview()!.style.top).toBe("420px");
  });

  // --- preview content ----------------------------------------------------

  it("shows the preview's facts and a details link", async () => {
    render(AnimeCard, {
      props: {
        anime: anime({ episodeCount: 1100, format: "TV", seasonYear: 1999 }),
      },
    });

    await fireEvent.mouseEnter(trigger());

    const panel = preview()!;
    expect(panel).toHaveTextContent("1100 eps");
    expect(panel).toHaveTextContent("TV");
    expect(panel).toHaveTextContent("1999");
    expect(
      screen.getByRole("link", { name: /view details/i }),
    ).toHaveAttribute("href", "/anime/21");
  });

  it("links each genre in the preview to its browse route", async () => {
    render(AnimeCard, {
      props: { anime: anime({ genres: ["Slice of Life"] }) },
    });

    await fireEvent.mouseEnter(trigger());

    expect(screen.getByRole("link", { name: "Slice of Life" })).toHaveAttribute(
      "href",
      "/filter?sort=popularity&genre=Slice+of+Life",
    );
  });

  it("strips HTML from the preview's synopsis", async () => {
    render(AnimeCard, { props: { anime: anime({ description: "a<br>b" }) } });

    await fireEvent.mouseEnter(trigger());

    expect(preview()).toHaveTextContent("a b");
  });
});