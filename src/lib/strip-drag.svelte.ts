// Pointer-drag scrolling for a horizontally scrolling strip.
//
// The strip hides its scrollbar, so dragging is the primary way to move it by
// hand; the arrows are the precise way. The element owns the geometry, this
// owns the gesture: when a press becomes a drag, how far the content follows,
// and swallowing the click that would otherwise land on whatever sat under the
// pointer when it was released.

import { clampScrollLeft, glidePosition, snapTarget } from "./scroll-strip";

/**
 * How far the pointer must travel before a press counts as a drag.
 *
 * Without this, the tiny movement in an ordinary click would be treated as a
 * drag and the click on the tab under the pointer would be swallowed.
 */
export const DRAG_THRESHOLD_PX = 5;

/**
 * How long the release glide takes, in milliseconds.
 *
 * Deliberately slower than an arrow press: the pointer has just been moving, so
 * a longer deceleration reads as the strip settling rather than snapping. Raise
 * it for an even softer landing.
 */
export const SETTLE_DURATION_MS = 700;

/** Wiring for {@link createStripDrag}. */
export interface StripDragOptions {
  /** Distance from one card's left edge to the next, in pixels. */
  getStride: () => number;
  /** Override the release-glide duration. */
  settleDurationMs?: number;
}

/**
 * Wire pointer-drag scrolling for one strip.
 *
 * `getEl` is a getter because the element does not exist when this runs.
 *
 * Must be called during component initialisation so the teardown effect can
 * attach.
 */
export function createStripDrag(
  getEl: () => HTMLElement | null,
  options: StripDragOptions,
) {
  /** Whether a press is down. Drives the cursor and text selection. */
  let dragging = $state(false);

  /**
   * Whether an animated glide is running.
   *
   * The component mirrors this onto the element as a class, which turns scroll
   * snapping OFF for the duration. A `mandatory` snap resolves instantly, so
   * leaving it on would both hard-snap the moment the pointer is released and
   * fight every frame of the glide.
   */
  let settling = $state(false);

  /** The in-flight glide's animation frame, if any. */
  let raf: number | null = null;

  /**
   * Whether the press has travelled far enough to be a drag rather than a
   * click.
   *
   * Deliberately NOT `$state`: it is read only in event handlers, never in the
   * template, so making it reactive would be wasted work.
   */
  let moved = false;

  /** Where the pointer went down, and the scroll offset at that moment. */
  let startX = 0;
  let startScrollLeft = 0;

  function onMove(event: PointerEvent) {
    const el = getEl();
    if (!el || !dragging) return;

    const dx = event.clientX - startX;

    // Below the threshold, stay a potential click: do not scroll, and do not
    // mark the gesture as a drag yet.
    if (!moved && Math.abs(dx) < DRAG_THRESHOLD_PX) return;
    moved = true;

    // The content follows the pointer, so dragging right (a positive dx) moves
    // the view left. Hence the subtraction.
    el.scrollLeft = clampScrollLeft(
      startScrollLeft - dx,
      el.clientWidth,
      el.scrollWidth,
    );
  }

  function stopListening() {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", finish);
    window.removeEventListener("pointercancel", finish);
  }

  /** Cancel a running glide without touching `settling`. */
  function stopGlide() {
    if (raf !== null) {
      cancelAnimationFrame(raf);
      raf = null;
    }
  }

  /**
   * Animate the strip to `target` with an ease-out curve.
   *
   * Driven by `requestAnimationFrame` rather than `scrollTo({ behavior:
   * "smooth" })`, because a native smooth scroll is resolved instantly under
   * `scroll-snap-type: mandatory` -- it would land with a jolt instead of
   * easing. Snap is off for the whole glide, and restored once it lands on a
   * boundary, where re-enabling it cannot move the strip.
   */
  function glideTo(target: number) {
    const el = getEl();
    if (!el) return;

    stopGlide();

    const start = el.scrollLeft;
    if (Math.abs(target - start) < 0.5) {
      settling = false;
      return;
    }

    settling = true;
    const duration = options.settleDurationMs ?? SETTLE_DURATION_MS;
    const t0 = performance.now();

    function frame(now: number) {
      const current = getEl();
      // The element can vanish mid-flight (navigated away). Stop quietly.
      if (!current) {
        raf = null;
        settling = false;
        return;
      }

      const progress = (now - t0) / duration;
      current.scrollLeft = glidePosition(start, target, progress);

      if (progress < 1) {
        raf = requestAnimationFrame(frame);
      } else {
        raf = null;
        settling = false;
      }
    }

    raf = requestAnimationFrame(frame);
  }

  function finish() {
    dragging = false;
    stopListening();

    const el = getEl();
    if (el) {
      // Settle on the nearest whole card. `maxScroll` is the element's own
      // travel, so the glide can never overshoot the ends.
      const maxScroll = el.scrollWidth - el.clientWidth;
      glideTo(snapTarget(el.scrollLeft, options.getStride(), maxScroll));
    }
  }

  function onPointerDown(event: PointerEvent) {
    // Primary button only: a right-click should open a menu, not start a drag.
    if (event.button !== 0) return;

    const el = getEl();
    if (!el) return;

    // A press takes over from any glide still running, so grabbing mid-flight
    // does not fight the animation.
    stopGlide();
    settling = false;

    dragging = true;
    moved = false;
    startX = event.clientX;
    startScrollLeft = el.scrollLeft;

    // Deliberately NO `setPointerCapture`. Capturing retargets the `click` that
    // follows the release to the capturing element, so a plain click on a day
    // tab would never reach the tab -- it would land on the strip instead. The
    // window listeners below already track a drag that leaves the strip, so
    // capture buys nothing here.
    //
    // Listening on the window, not the element, so a fast drag that outruns the
    // strip still tracks. `pointerup` on the window also catches a release that
    // happens outside it.
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", finish);
  }

  /**
   * Swallow the click that follows a drag.
   *
   * A release over a day tab fires a click on that tab; without this, finishing
   * a swipe would also select whatever the pointer happened to be over. Capture
   * phase, because the click targets the button beneath the pointer and would
   * otherwise reach it first.
   */
  function onclickcapture(event: MouseEvent) {
    if (!moved) return;

    moved = false;
    event.preventDefault();
    event.stopPropagation();
  }

  /**
   * Cancel the browser's own drag gesture inside the strip.
   *
   * Images and links are draggable by default, so grabbing a poster would start
   * a native drag (a ghost image following the pointer) instead of scrolling
   * the strip. `preventDefault` on `dragstart` stops it at the source, for any
   * content, without needing `draggable="false"` on every child.
   */
  function ondragstart(event: DragEvent) {
    event.preventDefault();
  }

  // Teardown only: a listener added on a press, or a glide's animation frame,
  // must not outlive the component.
  $effect(() => () => {
    stopGlide();
    dragging = false;
    settling = false;
    stopListening();
  });

  return {
    get dragging() {
      return dragging;
    },
    /**
     * Whether the strip is gliding to a snapped position.
     *
     * The component mirrors this onto the element, which suspends scroll
     * snapping for the duration. Without it the mandatory snap fires the
     * instant the pointer is released and the glide never gets to run.
     */
    get settling() {
      return settling;
    },
    /**
     * Animate the strip to a whole-card offset.
     *
     * Exposed so the arrow buttons use the same easing as a drag release, rather
     * than one path animating and the other jumping.
     */
    glideTo,
    onpointerdown: onPointerDown,
    onclickcapture,
    ondragstart,
  };
}