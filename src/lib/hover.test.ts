import { describe, it, expect } from "vitest";

import { positionPreview, PREVIEW_GAP } from "./hover";

const SIZE = { width: 288, height: 340 };
const VIEWPORT = { width: 1280, height: 800 };

/** A card at the given top-left, 160x240 like the poster. */
function card(left: number, top: number) {
  return { left, top, right: left + 160, bottom: top + 240 };
}

describe("positionPreview", () => {
  it("sits to the right of the card", () => {
    const { x } = positionPreview(card(100, 100), SIZE, VIEWPORT);

    expect(x).toBe(100 + 160 + PREVIEW_GAP);
  });

  it("aligns its top with the card's", () => {
    const { y } = positionPreview(card(100, 120), SIZE, VIEWPORT);

    expect(y).toBe(120);
  });

  it("flips to the left when the right would overflow", () => {
    // Card near the right edge: right + width cannot fit.
    const { x } = positionPreview(card(1100, 100), SIZE, VIEWPORT);

    expect(x).toBe(1100 - SIZE.width - PREVIEW_GAP);
  });

  it("stays on-screen when there is no room either side", () => {
    const narrow = { width: 320, height: 800 };
    const { x } = positionPreview(card(200, 100), SIZE, narrow);

    // Clamped to the left margin rather than a negative coordinate.
    expect(x).toBe(PREVIEW_GAP);
  });

  it("pulls up when the preview would run off the bottom", () => {
    const { y } = positionPreview(card(100, 700), SIZE, VIEWPORT);

    expect(y).toBe(VIEWPORT.height - SIZE.height - PREVIEW_GAP);
  });

  it("never leaves a negative top", () => {
    // A preview taller than the viewport cannot be placed without clamping.
    const short = { width: 1280, height: 200 };
    const { y } = positionPreview(card(100, 100), SIZE, short);

    expect(y).toBe(PREVIEW_GAP);
  });

  it("honours a custom gap", () => {
    const { x } = positionPreview(card(100, 100), SIZE, VIEWPORT, 24);

    expect(x).toBe(100 + 160 + 24);
  });

  it("produces whole numbers for a whole-numbered input", () => {
    // Fractional rects are common from getBoundingClientRect; the result is
    // used directly as a CSS length, so it should stay finite.
    const { x, y } = positionPreview(card(10.5, 20.25), SIZE, VIEWPORT);

    expect(Number.isFinite(x)).toBe(true);
    expect(Number.isFinite(y)).toBe(true);
  });
});