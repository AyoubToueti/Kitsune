import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";

import { SETTLE_DURATION_MS } from "./strip-drag.svelte";
import { harness, setGeometry, type StripDragHarness } from "../test/strip-drag-harness.svelte";

// A 1000px-wide strip showing 400px, so 600px of travel.
const WIDTH = 400;
const TOTAL = 1000;

let active: StripDragHarness | null = null;

function start(): StripDragHarness {
  const h = harness();
  setGeometry(h.el, WIDTH, TOTAL);
  active = h;
  return h;
}

/** The event the template wires on the strip itself. */
function pointerDown(h: StripDragHarness, clientX: number, button = 0): void {
  h.onpointerdown(
    new PointerEvent("pointerdown", { clientX, button, pointerId: 1, bubbles: true }),
  );
}

/** A move on the window, as the composable listens. */
function pointerMove(clientX: number): void {
  window.dispatchEvent(
    new PointerEvent("pointermove", { clientX, pointerId: 1, bubbles: true }),
  );
}

function pointerUp(): void {
  window.dispatchEvent(
    new PointerEvent("pointerup", { clientX: 0, pointerId: 1, bubbles: true }),
  );
}

afterEach(() => {
  active?.destroy();
  active = null;
});

describe("createStripDrag", () => {
  it("starts not dragging", () => {
    expect(start().dragging()).toBe(false);
  });

  it("reports a drag while the button is down", () => {
    const h = start();

    pointerDown(h, 200);

    expect(h.dragging()).toBe(true);
  });

  it("stops dragging on release", () => {
    const h = start();

    pointerDown(h, 200);
    pointerUp();

    expect(h.dragging()).toBe(false);
  });

  it("scrolls the content with the pointer", () => {
    const h = start();
    h.el.scrollLeft = 300;

    pointerDown(h, 200);
    // Move left by 100: the view should advance to the right by the same.
    pointerMove(100);

    expect(h.el.scrollLeft).toBe(400);
  });

  it("scrolls back when the pointer moves right", () => {
    const h = start();
    h.el.scrollLeft = 300;

    pointerDown(h, 100);
    pointerMove(200);

    expect(h.el.scrollLeft).toBe(200);
  });

  it("ignores movement below the drag threshold", () => {
    const h = start();
    h.el.scrollLeft = 300;

    pointerDown(h, 200);
    // 3px is inside the threshold, so this is still a potential click.
    pointerMove(197);

    expect(h.el.scrollLeft).toBe(300);
  });

  it("starts dragging once the threshold is passed", () => {
    const h = start();

    pointerDown(h, 200);
    pointerMove(200 - 20);

    expect(h.dragging()).toBe(true);
  });

  it("clamps at the end of the strip", () => {
    const h = start();
    h.el.scrollLeft = TOTAL - WIDTH - 5; // 5px from the end

    pointerDown(h, 200);
    pointerMove(0); // would travel 200px

    expect(h.el.scrollLeft).toBe(TOTAL - WIDTH);
  });

  it("clamps at the start of the strip", () => {
    const h = start();
    h.el.scrollLeft = 10;

    pointerDown(h, 0);
    pointerMove(200);

    expect(h.el.scrollLeft).toBe(0);
  });

  it("ignores a right-button press", () => {
    const h = start();

    pointerDown(h, 200, 2);

    expect(h.dragging()).toBe(false);
  });

  it("never captures the pointer", () => {
    const h = start();

    // Regression guard: capturing retargets the click that follows the release
    // to the strip, so a plain click on a day tab would never reach the tab.
    // The window listeners already cover a drag that leaves the strip.
    const capture = vi.fn();
    h.el.setPointerCapture = capture;

    pointerDown(h, 200);
    pointerMove(100);

    expect(capture).not.toHaveBeenCalled();
  });

  it("cancels the browser's native drag gesture", () => {
    const h = start();

    // Without this, grabbing a poster starts a native drag (a ghost image
    // following the pointer) instead of scrolling the strip.
    const event = new Event("dragstart", { cancelable: true });
    h.ondragstart(event as DragEvent);

    expect(event.defaultPrevented).toBe(true);
  });

  it("ignores movement when no press is active", () => {
    const h = start();
    h.el.scrollLeft = 300;

    pointerMove(0);

    expect(h.el.scrollLeft).toBe(300);
  });

  it("stops tracking after release", () => {
    const h = start();
    h.el.scrollLeft = 300;

    pointerDown(h, 200);
    pointerUp();
    pointerMove(0);

    // A move after release must not scroll: the window listener is gone.
    expect(h.el.scrollLeft).toBe(300);
  });

  it("swallows the click that follows a drag", () => {
    const h = start();

    pointerDown(h, 200);
    pointerMove(100);

    const click = new MouseEvent("click", { cancelable: true, bubbles: true });
    h.onclickcapture(click);

    expect(click.defaultPrevented).toBe(true);
  });

  it("lets a plain click through", () => {
    const h = start();

    pointerDown(h, 200);
    // No movement past the threshold: the click is a real click.
    pointerUp();

    const click = new MouseEvent("click", { cancelable: true, bubbles: true });
    h.onclickcapture(click);

    expect(click.defaultPrevented).toBe(false);
  });
});

describe("createStripDrag release glide", () => {
  // A single stride, so "nearest whole card" is a round number.
  const STRIDE = 100;

  function start(): StripDragHarness {
    const h = harness(STRIDE);
    setGeometry(h.el, WIDTH, TOTAL);
    active = h;
    return h;
  }

  /**
   * Drive the glide to completion.
   *
   * The animation runs on `requestAnimationFrame`, which fake timers also
   * control, so advancing by the full duration runs every frame and lands the
   * strip. Deterministic, unlike polling a wall-clock animation -- which
   * flaked under the load of a full-suite run.
   */
  function runGlide(): void {
    vi.advanceTimersByTime(SETTLE_DURATION_MS + 32);
  }

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("is not settling before any gesture", () => {
    expect(start().settling()).toBe(false);
  });

  it("starts settling on release", () => {
    const h = start();

    pointerDown(h, 200);
    // Drag to scrollLeft 30: off the stride-100 grid, so a glide is needed.
    pointerMove(170);
    pointerUp();

    // The glide begins immediately, so the strip can suspend snapping.
    expect(h.settling()).toBe(true);
  });

  it("eases to the nearest whole card on release", () => {
    const h = start();

    pointerDown(h, 200);
    // Drag left by 30: scrollLeft becomes 30, nearest stride point is 0.
    pointerMove(170);
    pointerUp();

    runGlide();
    expect(h.settling()).toBe(false);
    expect(h.el.scrollLeft).toBeCloseTo(0, 0);
  });

  it("settles onto the far card when past the halfway point", () => {
    const h = start();

    pointerDown(h, 200);
    // Drag left by 70: scrollLeft becomes 70, nearest point is 100.
    pointerMove(130);
    pointerUp();

    runGlide();
    expect(h.settling()).toBe(false);
    expect(h.el.scrollLeft).toBeCloseTo(100, 0);
  });

  it("never settles past the end of the strip", () => {
    const h = start();

    pointerDown(h, 0);
    // Drag far past the end; maxScroll is 600.
    pointerMove(-5000);
    pointerUp();

    runGlide();
    expect(h.settling()).toBe(false);
    expect(h.el.scrollLeft).toBeLessThanOrEqual(TOTAL - WIDTH);
  });

  it("lets a new press cancel an in-flight glide", () => {
    const h = start();

    pointerDown(h, 200);
    // Land off the grid, so a glide actually starts.
    pointerMove(170);
    pointerUp();
    expect(h.settling()).toBe(true);

    // Grabbing again takes over from the animation.
    pointerDown(h, 100);
    expect(h.settling()).toBe(false);
  });

  it("glides to a given offset on demand", () => {
    const h = start();

    h.glideTo(300);

    expect(h.settling()).toBe(true);
    runGlide();
    expect(h.settling()).toBe(false);
    expect(h.el.scrollLeft).toBeCloseTo(300, 0);
  });

  it("does not start a glide for a zero-distance target", () => {
    const h = start();
    h.el.scrollLeft = 100;

    h.glideTo(100);

    expect(h.settling()).toBe(false);
  });
});