import { describe, it, expect } from "vitest";

import {
  positionPreview,
  CARET_MARGIN,
  PREVIEW_OVERLAP,
  PREVIEW_SIZE,
  VIEWPORT_MARGIN,
} from "./hover";

const SIZE = PREVIEW_SIZE;
const VIEWPORT = { width: 1280, height: 800 };

/** A card at the given top-left, 160x240 like the poster. */
function card(left: number, top: number) {
  return { left, top, right: left + 160, bottom: top + 240 };
}

describe("positionPreview", () => {
  it("sits above the card", () => {
    const { y, side } = positionPreview(card(100, 500), SIZE, VIEWPORT);

    expect(side).toBe("bottom");
    // Bottom edge laps over the card's top rather than resting against it.
    expect(y + SIZE.height).toBe(500 + PREVIEW_OVERLAP);
  });

  it("overlaps the card horizontally instead of sitting beside it", () => {
    const { x } = positionPreview(card(100, 500), SIZE, VIEWPORT);

    // Starts at the card's left edge, so the two overlap.
    expect(x).toBe(100);
  });

  it("keeps the caret inside the panel", () => {
    const { caretX } = positionPreview(card(100, 500), SIZE, VIEWPORT);

    expect(caretX).toBeGreaterThanOrEqual(CARET_MARGIN);
    expect(caretX).toBeLessThanOrEqual(SIZE.width - CARET_MARGIN);
  });

  it("aims the caret at the middle of the card", () => {
    // Card spans 100..260, so its centre is 180; the preview starts at 100.
    const { caretX } = positionPreview(card(100, 500), SIZE, VIEWPORT);

    expect(caretX).toBe(80);
  });

  it("holds the caret at the margin when the card extends past the panel", () => {
    // A wide card whose centre is far to the right of the preview.
    const wide = { left: 0, top: 500, right: 900, bottom: 740 };
    const { caretX } = positionPreview(wide, SIZE, VIEWPORT);

    // Clamped rather than drawn outside the panel.
    expect(caretX).toBe(SIZE.width - CARET_MARGIN);
  });

  it("flips below when there is no room above", () => {
    const { y, side } = positionPreview(card(100, 10), SIZE, VIEWPORT);

    expect(side).toBe("top");
    // Laps over the card's bottom edge.
    expect(y).toBe(10 + 240 - PREVIEW_OVERLAP);
  });

  it("pulls left when starting at the card would overflow", () => {
    const { x } = positionPreview(card(1200, 500), SIZE, VIEWPORT);

    expect(x).toBe(VIEWPORT.width - SIZE.width - VIEWPORT_MARGIN);
  });

  it("never leaves a negative x", () => {
    const narrow = { width: 200, height: 800 };
    const { x } = positionPreview(card(10, 500), SIZE, narrow);

    expect(x).toBe(VIEWPORT_MARGIN);
  });

  it("clamps rather than overflowing when the viewport is shorter than the preview", () => {
    const short = { width: 1280, height: 200 };
    const { y } = positionPreview(card(100, 150), SIZE, short);

    expect(y).toBeGreaterThanOrEqual(VIEWPORT_MARGIN);
  });

  it("honours a custom overlap", () => {
    const { y } = positionPreview(card(100, 500), SIZE, VIEWPORT, { overlap: 40 });

    expect(y + SIZE.height).toBe(540);
  });

  it("produces finite coordinates for fractional rects", () => {
    // getBoundingClientRect frequently returns fractions.
    const { x, y, caretX } = positionPreview(
      { left: 10.5, top: 500.25, right: 170.5, bottom: 740.25 },
      SIZE,
      VIEWPORT,
    );

    expect(Number.isFinite(x)).toBe(true);
    expect(Number.isFinite(y)).toBe(true);
    expect(Number.isFinite(caretX)).toBe(true);
  });
});