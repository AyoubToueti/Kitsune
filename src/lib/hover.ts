// Where to place a hover preview relative to the card it belongs to.
//
// Pure so the edge cases -- flipping near the right edge, pulling up near the
// bottom -- are testable without a browser.

/** A rectangle, as `getBoundingClientRect` reports it. */
export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

/** Distance between the card and the preview. */
export const PREVIEW_GAP = 8;

/**
 * Place the preview beside `anchor`.
 *
 * Prefers the card's right, flipping to the left when it would overflow. The
 * top is aligned with the card's, pulled up when the preview is taller than the
 * space below. Both axes are clamped so it can never sit off-screen.
 */
export function positionPreview(
  anchor: Rect,
  size: Size,
  viewport: Size,
  gap: number = PREVIEW_GAP,
): Point {
  // Horizontal: right of the card, flipped when that would overflow.
  let x = anchor.right + gap;
  if (x + size.width > viewport.width) {
    x = anchor.left - size.width - gap;
  }
  // Clamped last so a flipped preview on a narrow viewport still starts
  // on-screen rather than at a negative coordinate.
  x = Math.max(gap, x);

  // Vertical: aligned with the card, pulled up when it would overflow.
  let y = anchor.top;
  if (y + size.height > viewport.height) {
    y = viewport.height - size.height - gap;
  }
  y = Math.max(gap, y);

  return { x, y };
}

/** The default preview footprint, matching the component's classes. */
export const PREVIEW_SIZE: Size = { width: 288, height: 340 };