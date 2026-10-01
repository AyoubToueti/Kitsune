import { describe, it, expect } from "vitest";

import {
  STEP_FRACTION,
  breakpointCardWidth,
  canScrollLeft,
  canScrollRight,
  cardCountFor,
  cardWidthFor,
  clampScrollLeft,
  easeOutCubic,
  fitCardWidth,
  fitCount,
  glidePosition,
  railCardCountFor,
  railSlideWidth,
  snapTarget,
  stepByStride,
  stepScrollLeft,
} from "./scroll-strip";

// A 1000px-wide strip showing 400px, so 600px of travel.
const WIDTH = 400;
const TOTAL = 1000;
const MAX = TOTAL - WIDTH;

describe("clampScrollLeft", () => {
  it("keeps a value inside the scrollable range", () => {
    expect(clampScrollLeft(100, WIDTH, TOTAL)).toBe(100);
  });

  it("never goes below zero", () => {
    expect(clampScrollLeft(-50, WIDTH, TOTAL)).toBe(0);
  });

  it("never goes past the end", () => {
    expect(clampScrollLeft(9999, WIDTH, TOTAL)).toBe(MAX);
  });

  it("is zero when everything fits", () => {
    // scrollWidth <= clientWidth: nothing to scroll, so any value clamps to 0.
    expect(clampScrollLeft(100, WIDTH, WIDTH)).toBe(0);
  });
});

describe("canScrollLeft / canScrollRight", () => {
  it("cannot scroll left from the start", () => {
    expect(canScrollLeft(0)).toBe(false);
  });

  it("can scroll left once moved", () => {
    expect(canScrollLeft(100)).toBe(true);
  });

  it("tolerates a sub-pixel offset at the start", () => {
    // Browsers report fractional scrollLeft; an exact check would flicker.
    expect(canScrollLeft(0.5)).toBe(false);
  });

  it("can scroll right until the end", () => {
    expect(canScrollRight(0, WIDTH, TOTAL)).toBe(true);
  });

  it("cannot scroll right once the end is reached", () => {
    expect(canScrollRight(MAX, WIDTH, TOTAL)).toBe(false);
  });

  it("tolerates a sub-pixel offset at the end", () => {
    expect(canScrollRight(MAX - 0.5, WIDTH, TOTAL)).toBe(false);
  });
});

describe("stepScrollLeft", () => {
  it("advances by most of a viewport to the right", () => {
    expect(stepScrollLeft(0, WIDTH, TOTAL, 1)).toBe(
      Math.round(WIDTH * STEP_FRACTION),
    );
  });

  it("steps back to the left", () => {
    expect(stepScrollLeft(320, WIDTH, TOTAL, -1)).toBe(0);
  });

  it("clamps at the end rather than overshooting", () => {
    // From near the end, a full step would pass the maximum.
    expect(stepScrollLeft(MAX - 10, WIDTH, TOTAL, 1)).toBe(MAX);
  });

  it("clamps at the start rather than going negative", () => {
    expect(stepScrollLeft(10, WIDTH, TOTAL, -1)).toBe(0);
  });

  it("travels less than the full viewport, so items are not skipped", () => {
    expect(Math.round(WIDTH * STEP_FRACTION)).toBeLessThan(WIDTH);
  });
});

describe("stepByStride", () => {
  it("advances by exactly one stride", () => {
    expect(stepByStride(0, WIDTH, TOTAL, 81.6, 1)).toBeCloseTo(81.6, 5);
  });

  it("steps back by one stride", () => {
    expect(stepByStride(200, WIDTH, TOTAL, 81.6, -1)).toBeCloseTo(118.4, 5);
  });

  it("clamps at the end", () => {
    expect(stepByStride(MAX - 10, WIDTH, TOTAL, 81.6, 1)).toBe(MAX);
  });

  it("clamps at the start", () => {
    expect(stepByStride(10, WIDTH, TOTAL, 81.6, -1)).toBe(0);
  });
});

describe("fitCount", () => {
  it("counts how many cards of the minimum width fit", () => {
    // 5 * 72 + 4 * 8 = 392 <= 400; 6 * 72 + 5 * 8 = 472 > 400.
    expect(fitCount(400, 72, 8)).toBe(5);
  });

  it("never returns zero, even for a strip narrower than one card", () => {
    expect(fitCount(40, 72, 8)).toBe(1);
  });

  it("returns one before layout, when the width is zero", () => {
    expect(fitCount(0, 72, 8)).toBe(1);
  });

  it("fits an exact number without a spare", () => {
    // 5 * 72 + 4 * 8 = 392 is the most that fits, so a 392 strip still fits 5.
    expect(fitCount(392, 72, 8)).toBe(5);
  });
});

describe("cardWidthFor", () => {
  it("divides the width evenly, gaps included", () => {
    // 5 cards + 4 gaps must equal the width exactly.
    const width = cardWidthFor(400, 5, 8);
    expect(width * 5 + 8 * 4).toBeCloseTo(400, 5);
  });

  it("gives a lone card the whole width", () => {
    expect(cardWidthFor(400, 1, 8)).toBe(400);
  });

  it("never returns a negative width", () => {
    expect(cardWidthFor(0, 5, 8)).toBe(0);
  });
});

describe("fitCardWidth", () => {
  it("returns a width that fills the strip with whole cards", () => {
    const width = fitCardWidth(400, 72, 8);
    const count = fitCount(400, 72, 8);

    expect(width * count + 8 * (count - 1)).toBeCloseTo(400, 5);
  });

  it("falls back to the minimum before layout", () => {
    // clientWidth is 0 in SSR and jsdom; the cards still need a width.
    expect(fitCardWidth(0, 72, 8)).toBe(72);
  });

  it("never makes a card narrower than intended to fill the row", () => {
    // The chosen count is the largest that fits at the minimum, so stretching
    // them can only widen them, never shrink below the minimum.
    expect(fitCardWidth(400, 72, 8)).toBeGreaterThanOrEqual(72);
  });
});

describe("cardCountFor", () => {
  it("returns the count for the widest band that fits", () => {
    // 1024 is the boundary for 9 cards; 1280 would be 12.
    expect(cardCountFor(1024)).toBe(9);
    expect(cardCountFor(1279)).toBe(9);
    expect(cardCountFor(1280)).toBe(12);
  });

  it("falls back to the smallest band below the first breakpoint", () => {
    expect(cardCountFor(100)).toBe(4);
    expect(cardCountFor(0)).toBe(4);
  });
});

describe("breakpointCardWidth", () => {
  it("fills the row with exactly the breakpoint's count", () => {
    // At 1024 the table says 9 cards.
    const width = breakpointCardWidth(1024, 72, 8);
    expect(width * 9 + 8 * 8).toBeCloseTo(1024, 5);
  });

  it("falls back to the minimum before layout", () => {
    expect(breakpointCardWidth(0, 72, 8)).toBe(72);
  });
});

describe("easeOutCubic", () => {
  it("starts at 0 and ends at 1", () => {
    expect(easeOutCubic(0)).toBe(0);
    expect(easeOutCubic(1)).toBe(1);
  });

  it("clamps progress outside 0..1", () => {
    expect(easeOutCubic(-1)).toBe(0);
    expect(easeOutCubic(2)).toBe(1);
  });

  it("moves more than half way by the midpoint (decelerating)", () => {
    // An ease-out front-loads its movement: at t=0.5 it has travelled >50%.
    expect(easeOutCubic(0.5)).toBeGreaterThan(0.5);
  });
});

describe("glidePosition", () => {
  it("starts at the start and ends at the target", () => {
    expect(glidePosition(0, 100, 0)).toBe(0);
    expect(glidePosition(0, 100, 1)).toBe(100);
  });

  it("interpolates between the two", () => {
    const mid = glidePosition(0, 100, 0.5);
    expect(mid).toBeGreaterThan(0);
    expect(mid).toBeLessThan(100);
  });

  it("handles a backwards glide", () => {
    expect(glidePosition(100, 0, 1)).toBe(0);
  });
});

describe("snapTarget", () => {
  it("rounds to the nearest whole stride", () => {
    expect(snapTarget(140, 100, 1000)).toBe(100);
    expect(snapTarget(160, 100, 1000)).toBe(200);
  });

  it("clamps to the end of the scroll range", () => {
    expect(snapTarget(980, 100, 1000)).toBe(1000);
  });

  it("clamps to zero at the start", () => {
    expect(snapTarget(10, 100, 1000)).toBe(0);
  });

  it("returns the offset unchanged when the stride is zero", () => {
    // Guards against a divide-by-zero while the strip has no measured card.
    expect(snapTarget(123, 0, 1000)).toBe(123);
  });
});

describe("railCardCountFor", () => {
  it("returns the count for the widest band that fits", () => {
    expect(railCardCountFor(1280)).toBe(7);
    expect(railCardCountFor(1279)).toBe(6);
    expect(railCardCountFor(1600)).toBe(8);
  });

  it("falls back to the smallest band below the first breakpoint", () => {
    expect(railCardCountFor(100)).toBe(3);
    expect(railCardCountFor(0)).toBe(3);
  });
});

describe("railSlideWidth", () => {
  it("fills the row with exactly the breakpoint's count", () => {
    // At 1280 the table says 7 slides, gap 16.
    const width = railSlideWidth(1280, 180, 16);
    expect(width * 7 + 16 * 6).toBeCloseTo(1280, 5);
  });

  it("falls back to the minimum before layout", () => {
    expect(railSlideWidth(0, 180, 16)).toBe(180);
  });

  it("never returns a negative width for a tiny viewport", () => {
    expect(railSlideWidth(1, 180, 16)).toBeGreaterThanOrEqual(0);
  });
});