// Shared hover-preview behaviour.
//
// Three call sites need it -- the poster card, the compact list row and the
// trending rail -- and each needs the same three things: a delay before
// closing so the pointer can cross the gap, a position recomputed as the page
// moves, and a timer that does not outlive the component.

import {
  positionPreview,
  PREVIEW_SIZE,
  type Placement,
} from "./hover";

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

  function cancelClose() {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  /**
   * Re-read the card's position.
   *
   * Called on show and again on every scroll or resize while open, which is
   * what makes the preview travel with the card rather than staying where it
   * first appeared.
   */
  function reposition() {
    const el = getAnchor();
    if (!el) return;

    placement = positionPreview(el.getBoundingClientRect(), PREVIEW_SIZE, {
      width: window.innerWidth,
      height: window.innerHeight,
    });
  }

  function onViewportChange() {
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
    show,
    scheduleClose,
    cancelClose,
  };
}