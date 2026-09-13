// Where to place a hover preview relative to the card it belongs to.
//
// Pure so the edge cases -- flipping when there is no room above, keeping the
// caret inside the panel -- are testable without a browser.

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

/** How the preview is anchored to its card. */
export interface Placement {
  x: number;
  y: number;
  /**
   * Which edge the caret sits on.
   *
   * `bottom` means the preview is above the card and the caret points down at
   * it; `top` means the preview flipped below and the caret points up.
   */
  side: "bottom" | "top";
  /** Caret centre, in preview-local pixels. */
  caretX: number;
}

/**
 * How much of the card's height the preview covers.
 *
 * A quarter, so the panel visibly laps over the artwork and reads as attached
 * to it rather than floating above. Proportional rather than a fixed offset,
 * which would look like a hairline on a large card and swamp a small one.
 */
export const PREVIEW_OVERLAP_RATIO = 0.25;

/**
 * Floor for the overlap.
 *
 * Keeps a very short anchor from producing an overlap so small the caret
 * appears to float, and guards the arithmetic against a zero-height rect.
 */
export const MIN_PREVIEW_OVERLAP = 8;

/** Minimum distance from the caret to the preview's own corner. */
export const CARET_MARGIN = 16;

/** Minimum distance from the preview to the viewport edge. */
export const VIEWPORT_MARGIN = 8;

/** The default preview footprint, matching the component's classes. */
export const PREVIEW_SIZE: Size = { width: 288, height: 320 };

/** Half the caret's rendered width, so its centre lands where intended. */
export const CARET_SIZE = 12;

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export interface PlacementOptions {
  overlap?: number;
  margin?: number;
}

/**
 * Place the preview so it overlaps its card, with a caret aimed at it.
 *
 * Prefers sitting above the card with its bottom edge lapping over the card's
 * top. When there is not enough room above, it flips below and laps over the
 * card's bottom instead.
 *
 * Horizontally it starts at the card's left edge, so the two overlap rather
 * than sitting side by side, and is pulled left when that would overflow.
 */
export function positionPreview(
  anchor: Rect,
  size: Size,
  viewport: Size,
  options: PlacementOptions = {},
): Placement {
  // An explicit pixel overlap wins; otherwise a quarter of the card's height.
  const anchorHeight = anchor.bottom - anchor.top;
  const overlap =
    options.overlap ??
    Math.max(MIN_PREVIEW_OVERLAP, anchorHeight * PREVIEW_OVERLAP_RATIO);
  const margin = options.margin ?? VIEWPORT_MARGIN;

  // Vertical: above the card, lapping over its top edge.
  let side: Placement["side"] = "bottom";
  let y = anchor.top - size.height + overlap;

  // No room above: flip below and lap over the card's bottom instead.
  if (y < margin) {
    side = "top";
    y = anchor.bottom - overlap;
  }

  // A viewport shorter than the preview cannot honour the overlap, so the
  // position is clamped rather than left off-screen.
  if (y + size.height > viewport.height - margin) {
    y = Math.max(margin, viewport.height - size.height - margin);
  }

  // Horizontal: start at the card's left so the preview covers part of it.
  let x = anchor.left;
  if (x + size.width > viewport.width - margin) {
    x = viewport.width - size.width - margin;
  }
  x = Math.max(margin, x);

  // The caret aims at the middle of the card, kept inside the preview's own
  // width so it never floats off the edge of the panel.
  const anchorCentre = anchor.left + (anchor.right - anchor.left) / 2;
  const caretX = clamp(
    anchorCentre - x,
    CARET_MARGIN,
    Math.max(CARET_MARGIN, size.width - CARET_MARGIN),
  );

  return { x, y, side, caretX };
}