// Which way a dropdown opens, when the trigger sits near a viewport edge.
//
// A trigger in the last column would push a right-opening menu off-screen; one
// in the first column would push a left-opening menu off the other side. The
// rule is a single measurement, so it lives here as a pure function rather than
// inline in the component -- `getBoundingClientRect` returns zeros in jsdom, so
// testing the raw measurement would be vacuous, but testing THIS is not.

/**
 * The menu's own width, in pixels.
 *
 * Must match the `w-44` class on the menu (44 * 4 = 176). Kept as a named
 * constant so the arithmetic and the class cannot drift silently.
 */
export const MENU_WIDTH = 176;

/** `"start"` = `left-0` (opens rightward); `"end"` = `right-0` (opens leftward). */
export type MenuAlign = "start" | "end";

/**
 * Pick the alignment that keeps the menu on screen.
 *
 * Prefers opening rightward from the trigger's left edge, which reads as
 * "unfolding from the button". Flips to right-aligned only when the menu would
 * overrun the viewport's right edge.
 *
 * `triggerLeft` and `viewportWidth` are in viewport coordinates, as
 * `getBoundingClientRect()` and `window.innerWidth` report them.
 */
export function chooseMenuAlign(
  triggerLeft: number,
  viewportWidth: number,
  menuWidth: number = MENU_WIDTH,
): MenuAlign {
  // Rightward fits when the menu spans [triggerLeft, triggerLeft + menuWidth]
  // without passing the right edge.
  return triggerLeft + menuWidth <= viewportWidth ? "start" : "end";
}