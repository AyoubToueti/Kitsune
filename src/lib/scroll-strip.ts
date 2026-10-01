// Pure scroll arithmetic for a horizontally scrolling strip.
//
// The component owns the element and its listeners; the decisions live here --
// how far one arrow press travels, whether an edge has been reached, where a
// clamped jump lands -- so they can be tested without a browser.

/**
 * How far one arrow press travels, as a fraction of the visible width.
 *
 * Less than a full viewport, so a press reveals the next items without skipping
 * any, and the overlap makes it obvious that more remain.
 */
export const STEP_FRACTION = 0.8;

/**
 * A one-pixel tolerance for edge detection.
 *
 * `scrollLeft`, `clientWidth` and `scrollWidth` are fractional in most
 * browsers, so an exact comparison would leave an arrow enabled forever while a
 * fraction of a pixel from the end.
 */
const EDGE_TOLERANCE = 1;

/** Leftmost valid `scrollLeft` for the given geometry. */
export function clampScrollLeft(
  value: number,
  clientWidth: number,
  scrollWidth: number,
): number {
  const max = Math.max(0, scrollWidth - clientWidth);
  return Math.min(Math.max(value, 0), max);
}

/** Whether content is hidden to the left of the viewport. */
export function canScrollLeft(scrollLeft: number): boolean {
  return scrollLeft > EDGE_TOLERANCE;
}

/** Whether content is hidden to the right of the viewport. */
export function canScrollRight(
  scrollLeft: number,
  clientWidth: number,
  scrollWidth: number,
): boolean {
  return scrollLeft + clientWidth < scrollWidth - EDGE_TOLERANCE;
}

/**
 * Where a press of `direction` (-1 left, +1 right) lands.
 *
 * Rounded, so the result is a whole number of pixels; a fractional `scrollLeft`
 * makes edge detection flicker on and off.
 */
export function stepScrollLeft(
  scrollLeft: number,
  clientWidth: number,
  scrollWidth: number,
  direction: 1 | -1,
): number {
  const step = Math.round(clientWidth * STEP_FRACTION) * direction;
  return clampScrollLeft(scrollLeft + step, clientWidth, scrollWidth);
}

/**
 * Where a press lands when the strip is measured in whole cards.
 *
 * `stride` is the distance from one card's left edge to the next -- the card's
 * width plus the gap. Advancing by exactly that keeps whole cards at the edges
 * instead of leaving a sliver of one behind, which is what a fraction of the
 * viewport does when the cards are not an even division of it.
 */
export function stepByStride(
  scrollLeft: number,
  clientWidth: number,
  scrollWidth: number,
  stride: number,
  direction: 1 | -1,
): number {
  return clampScrollLeft(scrollLeft + stride * direction, clientWidth, scrollWidth);
}

/**
 * How many cards of at least `minCardWidth` fit across `visibleWidth`.
 *
 * `n` cards occupy `n * minCardWidth + (n - 1) * gap`, so the answer is the
 * largest `n` that stays within the width. At least one, so a narrow strip
 * still shows a card rather than none.
 */
export function fitCount(
  visibleWidth: number,
  minCardWidth: number,
  gap: number,
): number {
  if (visibleWidth <= 0) return 1;

  const count = Math.floor((visibleWidth + gap) / (minCardWidth + gap));
  return Math.max(1, count);
}

/**
 * The exact width that makes `count` cards fill `visibleWidth`, gaps included.
 *
 * Setting each card to this -- rather than a fixed width -- is what guarantees
 * no half card is ever visible: `count` of them plus the gaps between them come
 * to precisely the visible width, so a whole number always fits.
 *
 * A fractional result is fine; browsers scroll by fractional pixels.
 */
export function cardWidthFor(
  visibleWidth: number,
  count: number,
  gap: number,
): number {
  if (count <= 1) return Math.max(0, visibleWidth);
  return Math.max(0, (visibleWidth - gap * (count - 1)) / count);
}

/**
 * The card width that fills `visibleWidth` with a whole number of cards.
 *
 * One call for the component: work out how many fit, then stretch them to fill
 * exactly. Before layout (`visibleWidth` is 0, as in SSR and jsdom) it falls
 * back to `minCardWidth`, so the cards still have a size to render with.
 */
export function fitCardWidth(
  visibleWidth: number,
  minCardWidth: number,
  gap: number,
): number {
  if (visibleWidth <= 0) return minCardWidth;
  const count = fitCount(visibleWidth, minCardWidth, gap);
  return cardWidthFor(visibleWidth, count, gap);
}

/**
 * How many cards to show at a given width.
 *
 * A fixed count per size band, rather than "as many as happen to fit", so the
 * strip shows the same whole number of cards at every window size -- five at
 * 900px, seven at 1100px, and so on. The card width is then derived to fill the
 * row exactly (`cardWidthFor`), which is what keeps a card from ever being cut
 * off at the edge.
 *
 * Ordered widest-first so the first match wins; the final entry is the floor.
 */
export const CARD_COUNT_STEPS: ReadonlyArray<{
  minWidth: number;
  count: number;
}> = [
  { minWidth: 1600, count: 15 },
  { minWidth: 1280, count: 12 },
  { minWidth: 1024, count: 9 },
  { minWidth: 768, count: 7 },
  { minWidth: 520, count: 5 },
  { minWidth: 0, count: 4 },
];

/** The card count for `visibleWidth`, from the breakpoint table. */
export function cardCountFor(visibleWidth: number): number {
  for (const step of CARD_COUNT_STEPS) {
    if (visibleWidth >= step.minWidth) return step.count;
  }
  // Unreachable: the table's last entry has `minWidth: 0`. Kept for the type
  // checker, which cannot know the array is non-empty and ordered.
  return CARD_COUNT_STEPS.at(-1)?.count ?? 1;
}

/**
 * The card width that fills `visibleWidth` with the breakpoint's card count.
 *
 * Before layout (`visibleWidth` is 0, as in SSR and jsdom) it falls back to
 * `minCardWidth`, so the cards still have a size to render with.
 */
export function breakpointCardWidth(
  visibleWidth: number,
  minCardWidth: number,
  gap: number,
): number {
  if (visibleWidth <= 0) return minCardWidth;
  return cardWidthFor(visibleWidth, cardCountFor(visibleWidth), gap);
}

/**
 * How many trending-rail slides to show at a given width.
 *
 * A rail slide is heavier than a day tab (rank badge plus a poster), so fewer
 * fit. Same idea as {@link CARD_COUNT_STEPS}: a fixed count per size band, with
 * the slide width derived so a whole number always fills the row.
 *
 * Ordered widest-first so the first match wins; the final entry is the floor.
 */
export const RAIL_COUNT_STEPS: ReadonlyArray<{
  minWidth: number;
  count: number;
}> = [
  { minWidth: 1600, count: 8 },
  { minWidth: 1280, count: 7 },
  { minWidth: 1024, count: 6 },
  { minWidth: 768, count: 4 },
  { minWidth: 0, count: 3 },
];

/** The rail slide count for `visibleWidth`, from the breakpoint table. */
export function railCardCountFor(visibleWidth: number): number {
  for (const step of RAIL_COUNT_STEPS) {
    if (visibleWidth >= step.minWidth) return step.count;
  }
  return RAIL_COUNT_STEPS.at(-1)?.count ?? 1;
}

/**
 * The slide width that fills `visibleWidth` with the rail's slide count.
 *
 * Before layout (`visibleWidth` is 0, as in SSR and jsdom) it falls back to
 * `minSlideWidth`, so the slides still have a size to render with.
 */
export function railSlideWidth(
  visibleWidth: number,
  minSlideWidth: number,
  gap: number,
): number {
  if (visibleWidth <= 0) return minSlideWidth;
  return cardWidthFor(visibleWidth, railCardCountFor(visibleWidth), gap);
}

/**
 * Ease-out cubic: quick to start, gentle to arrive.
 *
 * Used by the release glide, so the strip decelerates into the snapped card
 * rather than stopping dead.
 */
export function easeOutCubic(progress: number): number {
  const t = Math.min(Math.max(progress, 0), 1);
  return 1 - (1 - t) ** 3;
}

/**
 * Where a glide from `start` to `target` sits, `progress` being 0..1.
 *
 * Pure, so the animation's arithmetic is testable without a browser.
 */
export function glidePosition(
  start: number,
  target: number,
  progress: number,
): number {
  return start + (target - start) * easeOutCubic(progress);
}

/**
 * The nearest whole-stride offset to `scrollLeft`, clamped to the range.
 *
 * Used to settle a drag on a card boundary: the content may have stopped
 * anywhere, and this is the closest point that shows a whole card.
 */
export function snapTarget(
  scrollLeft: number,
  stride: number,
  maxScroll: number,
): number {
  const limit = Math.max(0, maxScroll);
  if (stride <= 0) return Math.min(Math.max(scrollLeft, 0), limit);

  const nearest = Math.round(scrollLeft / stride) * stride;
  return Math.min(Math.max(nearest, 0), limit);
}