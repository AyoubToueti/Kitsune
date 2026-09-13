// Page-number windowing for the pagination control.
//
// Pure so the fiddly parts -- clamping a deep link, collapsing the middle into
// a gap -- are testable without rendering anything.

/** A page number, or a collapsed run of them. */
export type PageEntry = number | "gap";

/** How many pages either side of the current one the window keeps. */
const DEFAULT_SPAN = 2;

/**
 * The page numbers to show, in order.
 *
 * Always includes the first and last page so the extent of the result set is
 * visible, plus a window around the current page. A long run between the window
 * and either end collapses to a single "gap" marker, which keeps a 300-page
 * result from rendering 300 buttons.
 *
 * `current` is clamped into range: a hand-edited `?page=999` should show the
 * last page rather than render a control with nothing selected.
 */
export function pageWindow(
  current: number,
  last: number,
  span: number = DEFAULT_SPAN,
): PageEntry[] {
  // A result set always has at least one page, even when empty, so the control
  // renders "1" rather than nothing at all.
  const total = Math.max(1, Math.floor(last));
  const active = Math.min(Math.max(Math.floor(current), 1), total);

  // When every page fits, show every page. Windowing a short result would hide
  // a page behind a gap that saves no space -- `1 2 3 ... 5` is strictly worse
  // than `1 2 3 4 5`.
  const fitsWithoutGaps = 2 * span + 3;
  if (total <= fitsWithoutGaps) {
    return Array.from({ length: total }, (_, i) => i + 1);
  }

  const keep = new Set<number>([1, total]);
  for (let page = active - span; page <= active + span; page++) {
    if (page >= 1 && page <= total) keep.add(page);
  }

  const sorted = [...keep].sort((a, b) => a - b);
  const entries: PageEntry[] = [];

  let previous = 0;
  for (const page of sorted) {
    // Any skipped page needs a marker: a jump of two hides exactly one number,
    // and rendering `6 8` would read as contiguous. The "all fit" case above is
    // what stops a short result from being broken up by ellipses it does not
    // need.
    if (previous !== 0 && page - previous > 1) entries.push("gap");
    entries.push(page);
    previous = page;
  }

  return entries;
}

/**
 * Clamp a page number read from a URL.
 *
 * `last` may be 0 before the first response arrives, in which case page 1 is
 * the only sensible answer.
 */
export function clampPage(requested: number, last: number): number {
  // Checked first because `Number(undefined)` is NaN, which is exactly what a
  // missing query parameter produces.
  if (!Number.isFinite(requested)) return 1;

  const total = Math.max(1, Math.floor(last));
  return Math.min(Math.max(Math.floor(requested), 1), total);
}