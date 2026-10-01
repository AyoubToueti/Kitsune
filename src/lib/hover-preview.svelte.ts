// Shared hover-preview behaviour.
//
// Three call sites need it -- the poster card, the compact list row and the
// trending rail -- and each needs the same three things: a delay before
// closing so the pointer can cross the gap, a position recomputed as the page
// moves, and a timer that does not outlive the component.

import { isOffscreen, positionPreview, PREVIEW_SIZE, type Placement } from "./hover";

/**
 * How long the preview survives after the pointer leaves.
 *
 * The preview is a separate element from the card, so travelling between them
 * crosses a gap. Without a delay it would vanish mid-journey.
 */
export const CLOSE_DELAY_MS = 120;

const INITIAL: Placement = { x: 0, y: 0, side: "bottom", caretX: 0 };

/**
 * Wire up hover behaviour for one card.
 *
 * `getAnchor` is a getter rather than an element because the element does not
 * exist when this runs; it is bound during the component's first render.
 *
 * Must be called during component initialisation so the teardown effect can
 * attach.
 */
export function createHoverPreview(getAnchor: () => HTMLElement | null) {
  let open = $state(false);
  let placement = $state<Placement>(INITIAL);
  let timer: ReturnType<typeof setTimeout> | null = null;

  /**
   * The rendered preview, once it exists.
   *
   * Kept so the panel can be measured rather than assumed. `PREVIEW_SIZE` is
   * only an estimate; using it for the vertical offset left a visible gap
   * between the caret and the card, because the real panel is shorter.
   */
  let previewEl: HTMLElement | null = null;

  function cancelClose() {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  /**
   * Re-read both boxes and recompute the placement.
   *
   * Called on show, again once the preview has mounted, and on every scroll or
   * resize while open -- that last part is what makes the preview travel with
   * the card rather than staying where it first appeared.
   */
  function reposition() {
    const anchor = getAnchor();
    if (!anchor) return;

    const measured = previewEl
      ? { width: previewEl.offsetWidth, height: previewEl.offsetHeight }
      : null;

    // Fall back to the estimate when the panel cannot be measured: offsetWidth
    // is 0 before layout and in jsdom, and positioning against a zero-sized box
    // would be worse than a slightly-off estimate.
    const size =
      measured && measured.width > 0 && measured.height > 0
        ? measured
        : PREVIEW_SIZE;

    placement = positionPreview(anchor.getBoundingClientRect(), size, {
      width: window.innerWidth,
      height: window.innerHeight,
    });
  }

  function onViewportChange() {
    const anchor = getAnchor();
    if (!anchor) return;

    // Scrolling can carry the card clean off screen while the preview -- which
    // follows it -- slides under a stationary pointer. The pointer never left
    // the preview, so the hover that normally keeps it open is still live and
    // the preview would hang there over nothing. Once the card is gone the
    // preview has nothing to anchor to, so it goes too.
    const offscreen = isOffscreen(anchor.getBoundingClientRect(), {
      width: window.innerWidth,
      height: window.innerHeight,
    });

    if (offscreen) {
      cancelClose();
      open = false;
      detach();
      return;
    }

    reposition();
  }

  function attach() {
    // `capture: true` so a scroll inside a rail -- not just the page -- also
    // repositions. Passive because the handler never prevents default.
    window.addEventListener("scroll", onViewportChange, {
      capture: true,
      passive: true,
    });
    window.addEventListener("resize", onViewportChange);
  }

  function detach() {
    window.removeEventListener("scroll", onViewportChange, { capture: true });
    window.removeEventListener("resize", onViewportChange);
  }

  function show() {
    cancelClose();
    // Position with the estimate first so the panel is never painted at the
    // origin, then the mount action below re-measures and corrects it.
    reposition();
    open = true;
    attach();
  }

  /** Hide, after a grace period so the pointer can reach the preview. */
  function scheduleClose() {
    cancelClose();
    timer = setTimeout(() => {
      open = false;
      timer = null;
      detach();
    }, CLOSE_DELAY_MS);
  }

  /**
   * Measure the preview once it is in the DOM.
   *
   * Applied as an action on the panel. Actions run after the element mounts but
   * before paint, so the corrected position is what the user sees -- there is
   * no frame at the estimated position.
   */
  function measure(node: HTMLElement) {
    previewEl = node;
    reposition();

    return {
      destroy() {
        if (previewEl === node) previewEl = null;
      },
    };
  }

  // Teardown only: nothing may outlive the component.
  $effect(() => () => {
    cancelClose();
    detach();
  });

  return {
    get open() {
      return open;
    },
    get placement() {
      return placement;
    },
    measure,
    show,
    scheduleClose,
    cancelClose,
  };
}