import { describe, it, expect } from "vitest";

import { chooseMenuAlign, MENU_WIDTH } from "./menu-position";

describe("chooseMenuAlign", () => {
  it("opens rightward when there is room", () => {
    // A trigger at the left of a wide viewport: plenty of space to the right.
    expect(chooseMenuAlign(0, 1280)).toBe("start");
  });

  /// A trigger far enough right that the menu would overrun the edge must flip.
  it("flips to right-aligned at the right edge", () => {
    // 1200 + 176 = 1376 > 1280, so it cannot open rightward.
    expect(chooseMenuAlign(1200, 1280)).toBe("end");
  });

  /// Exactly fitting is still fine: the menu ends flush with the edge.
  it("opens rightward when it ends exactly at the edge", () => {
    expect(chooseMenuAlign(1280 - MENU_WIDTH, 1280)).toBe("start");
  });

  /// One pixel past the edge is where it flips.
  it("flips one pixel past a flush fit", () => {
    expect(chooseMenuAlign(1280 - MENU_WIDTH + 1, 1280)).toBe("end");
  });

  /// The width is a parameter so the arithmetic can be exercised with a value
  /// that is not the real menu, keeping the test readable.
  it("honours an explicit menu width", () => {
    expect(chooseMenuAlign(900, 1000, 200)).toBe("end");
    expect(chooseMenuAlign(700, 1000, 200)).toBe("start");
  });
});