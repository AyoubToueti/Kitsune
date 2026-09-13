import { describe, it, expect } from "vitest";

import {
  positionPreview,
  CARET_MARGIN,
  PREVIEW_SIZE,
  VIEWPORT_MARGIN,
} from "./hover";

const SIZE = PREVIEW_SIZE;
const VIEWPORT = { width: 1280, height: 800 };

/** A card at the given top-left, 160x240 like the poster. */
function card(left: number, top: number) {
  return { left, top, right: left + 160, bottom: top + 240 };
}

/** Centre of a rect, for readable expectations. */
function centre(r: ReturnType<typeof card>) {
  return {
    x: r.left + (r.right - r.left) / 2,
    y: r.top + (r.bottom - r.top) / 2,
  };
}

describe("positionPreview", () => {
  it("puts its bottom-left corner at the card's centre", () => {
    const anchor = card(100, 500);
    const { x, y } = positionPreview(anchor, SIZE, VIEWPORT);
    const c = centre(anchor);

    // The whole rule in one assertion: the panel's left edge and bottom edge
    // both meet at the card's centre.
    expect(x).toBe(c.x);
    expect(y + SIZE.height).toBe(c.y);
  });

  it("covers the card's top-right quadrant", () => {
    const anchor = card(100, 500);
    const { x, y } = positionPreview(anchor, SIZE, VIEWPORT);

    // Horizontally the panel starts halfway across, so it covers the right
    // half. Vertically it ends halfway down, so it covers the top half.
    expect(x).toBe(180);
    expect(y + SIZE.height).toBe(620);
  });

  it("sits above the card", () => {
    const { side } = positionPreview(card(100, 500), SIZE, VIEWPORT);

    expect(side).toBe("bottom");
  });

  it("overlaps the card rather than resting against it", () => {
    const anchor = card(100, 500);
    const { y } = positionPreview(anchor, SIZE, VIEWPORT);

    // The panel's bottom edge is inside the card's vertical span, which is what
    // leaves no gap for the pointer to cross.
    expect(y + SIZE.height).toBeGreaterThan(anchor.top);
    expect(y + SIZE.height).toBeLessThan(anchor.bottom);
  });

  it("keeps the caret inside the panel", () => {
    const { caretX } = positionPreview(card(100, 500), SIZE, VIEWPORT);

    expect(caretX).toBeGreaterThanOrEqual(CARET_MARGIN);
    expect(caretX).toBeLessThanOrEqual(SIZE.width - CARET_MARGIN);
  });

  it("holds the caret at its margin, since the anchor is at the panel's left", () => {
    const { caretX } = positionPreview(card(100, 500), SIZE, VIEWPORT);

    // centreX - x is zero by construction, so the clamp decides.
    expect(caretX).toBe(CARET_MARGIN);
  });

  it("flips below when there is no room above", () => {
    const anchor = card(100, 10);
    const { y, side } = positionPreview(anchor, SIZE, VIEWPORT);

    expect(side).toBe("top");
    // Top edge at the card's centre, so the bottom-right quadrant is covered.
    expect(y).toBe(centre(anchor).y);
  });

  it("keeps the caret at its margin when flipped, too", () => {
    const { caretX } = positionPreview(card(100, 10), SIZE, VIEWPORT);

    expect(caretX).toBe(CARET_MARGIN);
  });

  it("pulls left when the anchored position would overflow", () => {
    const { x } = positionPreview(card(1200, 500), SIZE, VIEWPORT);

    expect(x).toBe(VIEWPORT.width - SIZE.width - VIEWPORT_MARGIN);
  });

  it("keeps the caret inside the panel after being pulled left", () => {
    const { x, caretX } = positionPreview(card(1200, 500), SIZE, VIEWPORT);

    // Pulled left means the card's centre is now well to the right of the
    // panel's left edge, so the caret clamps at the other end.
    expect(caretX).toBe(SIZE.width - CARET_MARGIN);
    expect(x + caretX).toBeGreaterThan(0);
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

  it("honours a custom margin", () => {
    const { x } = positionPreview(card(1200, 500), SIZE, VIEWPORT, { margin: 40 });

    expect(x).toBe(VIEWPORT.width - SIZE.width - 40);
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

  it("handles a zero-sized anchor without producing NaN", () => {
    // Guards the centre arithmetic against a rect that has not laid out yet.
    const { x, y, caretX } = positionPreview(
      { left: 0, top: 0, right: 0, bottom: 0 },
      SIZE,
      VIEWPORT,
    );

    expect(Number.isFinite(x)).toBe(true);
    expect(Number.isFinite(y)).toBe(true);
    expect(Number.isFinite(caretX)).toBe(true);
  });
});