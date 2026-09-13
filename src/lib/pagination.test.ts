import { describe, it, expect } from "vitest";

import { clampPage, pageWindow } from "./pagination";

describe("pageWindow", () => {
  it("lists every page when they all fit", () => {
    expect(pageWindow(1, 5)).toEqual([1, 2, 3, 4, 5]);
  });

  it("always includes the first and last page", () => {
    const entries = pageWindow(50, 100);

    expect(entries[0]).toBe(1);
    expect(entries[entries.length - 1]).toBe(100);
  });

  it("collapses the middle into a gap", () => {
    const entries = pageWindow(50, 100);

    // The long runs either side of the window become gaps, so a 100-page
    // result does not render 100 buttons.
    expect(entries.filter((e) => e === "gap")).toHaveLength(2);
  });

  it("centres the window on the current page", () => {
    const entries = pageWindow(50, 100, 2);

    expect(entries).toContain(48);
    expect(entries).toContain(49);
    expect(entries).toContain(50);
    expect(entries).toContain(51);
    expect(entries).toContain(52);
  });

  it("renders no gap when every page fits", () => {
    // Pages 1..7 with a window at 4: everything fits, so a gap would be a
    // stray ellipsis with nothing hidden.
    expect(pageWindow(4, 7)).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });

  it("marks a gap even when it hides only one page", () => {
    // 8 pages with a window at 4 keeps 1,2,3,4,5,6 and the last page 8, hiding
    // only 7. Without a marker the control would render `6 8`, which reads as
    // contiguous and quietly skips a page.
    const entries = pageWindow(4, 8);

    expect(entries).toContain("gap");
    expect(entries).toContain(8);
    expect(entries).not.toContain(7);
  });

  it("has no gap at the very start", () => {
    expect(pageWindow(1, 100)[0]).toBe(1);
    expect(pageWindow(1, 100)[1]).toBe(2);
  });

  it("has no gap at the very end", () => {
    const entries = pageWindow(100, 100);

    expect(entries[entries.length - 2]).toBe(99);
    expect(entries[entries.length - 1]).toBe(100);
  });

  it("handles a single page", () => {
    expect(pageWindow(1, 1)).toEqual([1]);
  });

  it("clamps a current page past the end", () => {
    // A hand-edited ?page=999 must not produce an empty control.
    const entries = pageWindow(999, 10);

    expect(entries).toContain(10);
    expect(entries.filter((e) => typeof e === "number").every((p) => (p as number) <= 10)).toBe(
      true,
    );
  });

  it("clamps a current page below the start", () => {
    expect(pageWindow(0, 10)).toContain(1);
  });

  it("treats a zero page count as a single page", () => {
    // Before the first response arrives there is no known last page, and the
    // control should still render something sensible.
    expect(pageWindow(1, 0)).toEqual([1]);
  });

  it("honours a wider span", () => {
    const entries = pageWindow(50, 100, 5);

    expect(entries).toContain(45);
    expect(entries).toContain(55);
  });

  it("returns ascending numbers", () => {
    const numbers = pageWindow(50, 100).filter(
      (e): e is number => typeof e === "number",
    );

    expect([...numbers].sort((a, b) => a - b)).toEqual(numbers);
  });
});

describe("clampPage", () => {
  it("passes a valid page through", () => {
    expect(clampPage(3, 10)).toBe(3);
  });

  it("caps at the last page", () => {
    expect(clampPage(999, 10)).toBe(10);
  });

  it("floors at one", () => {
    expect(clampPage(0, 10)).toBe(1);
    expect(clampPage(-5, 10)).toBe(1);
  });

  it("returns one when the last page is not yet known", () => {
    expect(clampPage(4, 0)).toBe(1);
  });

  it("returns one for a non-finite page", () => {
    // `Number(undefined)` is NaN, which is exactly what a missing query
    // parameter produces.
    expect(clampPage(Number.NaN, 10)).toBe(1);
  });

  it("truncates a fractional page", () => {
    expect(clampPage(2.7, 10)).toBe(2);
  });
});