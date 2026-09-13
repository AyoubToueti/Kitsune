// Where to place a hover preview relative to the card it belongs to.
//
// Pure so the edge cases -- flipping when there is no room above, keeping the
// caret inside the panel, clamping at the viewport edge -- are testable without
// a browser.

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
  margin?: number;
}

/**
 * Place the preview so it covers one quadrant of its card.
 *
 * The rule is an anchor point rather than an amount: the preview's bottom-left
 * corner sits at the card's centre, so it covers the card's top-right quadrant.
 * This is what makes the two read as one attached unit -- the panel visibly
 * laps over the artwork instead of floating above it, and the pointer can move
 * from card to preview without crossing empty space.
 *
 * When there is not enough room above, the preview flips below and its top-left
 * corner takes the card's centre, covering the bottom-right quadrant instead.
 */
export function positionPreview(
  anchor: Rect,
  size: Size,
  viewport: Size,
  options: PlacementOptions = {},
): Placement {
  const margin = options.margin ?? VIEWPORT_MARGIN;

  const centreX = anchor.left + (anchor.right - anchor.left) / 2;
  const centreY = anchor.top + (anchor.bottom - anchor.top) / 2;

  // Above the card: bottom edge at the card's vertical centre.
  let side: Placement["side"] = "bottom";
  let y = centreY - size.height;

  // No room above: flip below, top edge at the centre instead.
  if (y < margin) {
    side = "top";
    y = centreY;
  }

  // A viewport shorter than the preview cannot honour the anchor, so the
  // position is clamped rather than left off-screen.
  if (y + size.height > viewport.height - margin) {
    y = Math.max(margin, viewport.height - size.height - margin);
  }

  // Left edge at the card's horizontal centre.
  let x = centreX;
  if (x + size.width > viewport.width - margin) {
    x = viewport.width - size.width - margin;
  }
  x = Math.max(margin, x);

  // The caret sits as close to the panel's left as its margin allows, which
  // puts it just inside the card's right half, pointing back at the artwork.
  const caretX = clamp(
    centreX - x,
    CARET_MARGIN,
    Math.max(CARET_MARGIN, size.width - CARET_MARGIN),
  );

  return { x, y, side, caretX };
}