// A component-free harness for `createStripDrag`.
//
// The composable uses runes, so exercising it needs a reactive context. A
// `.svelte.ts` module has one and `$effect.root` supplies the rest, which keeps
// the tests free of a throwaway component -- the same shape as
// `infinite-scroll-harness.svelte.ts`.
//
// The element is a plain, detached div: the composable only reads its geometry
// and calls `setPointerCapture`, and the tests drive the window listeners
// directly. Nothing here needs layout or a component tree.

import { flushSync } from "svelte";

import { createStripDrag } from "$lib/strip-drag.svelte";

export interface StripDragHarness {
  /** The strip the composable is bound to. */
  el: HTMLElement;
  /** Whether a drag is in progress. */
  dragging(): boolean;
  /** Whether a release glide is running. */
  settling(): boolean;
  /** Animate to an offset, as the arrows do. */
  glideTo(target: number): void;
  /** The handlers the component would wire in its template. */
  onpointerdown(event: PointerEvent): void;
  onclickcapture(event: MouseEvent): void;
  ondragstart(event: DragEvent): void;
  destroy(): void;
}

/**
 * Give a jsdom element scroll geometry.
 *
 * jsdom lays nothing out, so `clientWidth` and `scrollWidth` are always 0 and
 * the composable would clamp every move to 0. These are the values a real
 * layout would have produced.
 */
export function setGeometry(
  el: HTMLElement,
  clientWidth: number,
  scrollWidth: number,
): void {
  Object.defineProperty(el, "clientWidth", {
    value: clientWidth,
    configurable: true,
  });
  Object.defineProperty(el, "scrollWidth", {
    value: scrollWidth,
    configurable: true,
  });
}

/**
 * Mount one composable over a fresh div. Call `destroy` when done.
 *
 * `stride` is what the release glide snaps to; default to one card so the
 * existing drag tests keep a predictable target.
 */
export function harness(stride = 100): StripDragHarness {
  const el = document.createElement("div");

  let drag!: ReturnType<typeof createStripDrag>;

  const destroy = $effect.root(() => {
    drag = createStripDrag(() => el, { getStride: () => stride });
  });

  // `$effect` is scheduled, not run inline, so flush to match a real mount.
  flushSync();

  return {
    el,
    dragging: () => drag.dragging,
    settling: () => drag.settling,
    glideTo: (target) => drag.glideTo(target),
    onpointerdown: (event) => drag.onpointerdown(event),
    onclickcapture: (event) => drag.onclickcapture(event),
    ondragstart: (event) => drag.ondragstart(event),
    destroy,
  };
}