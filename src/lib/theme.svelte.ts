// Applying the reader's chosen appearance.
//
// `"light"`, `"dark"` or `"system"`. The choice drives a `data-theme`
// attribute on `<html>`; the CSS in `app.css` selects its token set from that.
// `"system"` resolves through `prefers-color-scheme` and keeps listening, so a
// reader who flips their OS theme mid-session sees the app follow without a
// reload.
//
// A `.svelte.ts` module because it owns a rune and a subscription; callers use
// it as a singleton (`applyTheme`, `initTheme`), not a per-component instance.

import { browser } from "$app/environment";

/** The three choices the UI offers. */
export type ThemeChoice = "light" | "dark" | "system";

/** The two concrete themes a choice can resolve to. */
export type ResolvedTheme = "light" | "dark";

const MEDIA = "(prefers-color-scheme: dark)";

/** The chosen theme, or `"system"` before anything is loaded. */
let choice = $state<ThemeChoice>("system");
/** The theme actually applied, after resolving `"system"`. */
let resolved = $state<ResolvedTheme>("dark");

/** The reader's current choice, for a binding. */
export function themeChoice(): ThemeChoice {
  return choice;
}

/** The theme currently applied to the document. */
export function resolvedTheme(): ResolvedTheme {
  return resolved;
}

/** Resolve a choice to the concrete theme for the current OS setting. */
function resolve(next: ThemeChoice): ResolvedTheme {
  if (next === "system") {
    return browser && window.matchMedia(MEDIA).matches ? "dark" : "light";
  }
  return next;
}

/** Write the concrete theme onto `<html>` and the rune. */
function paint(): void {
  resolved = resolve(choice);
  if (browser) {
    document.documentElement.dataset.theme = resolved;
  }
}

/**
 * Set the theme and paint it.
 *
 * The caller persists the choice separately (it is part of `Settings`); this
 * only applies it, so toggling feels instant and a failed save does not leave
 * the UI unstyled.
 */
export function applyTheme(next: ThemeChoice): void {
  choice = next;
  paint();
}

/** The theme the backend says was chosen, applied without persisting. */
export function setThemeFromSettings(stored: string): void {
  applyTheme(stored === "light" || stored === "dark" ? stored : "system");
}

/**
 * Apply the theme at startup and keep `"system"` in step with the OS.
 *
 * Returns a teardown that removes the OS listener. Idempotent: called once from
 * the layout, so a second call would add a second listener.
 */
export function initTheme(initial: ThemeChoice): () => void {
  applyTheme(initial);
  if (!browser) return () => {};

  const media = window.matchMedia(MEDIA);
  const onChange = () => {
    // Only `"system"` follows the OS; an explicit light/dark choice is a
    // deliberate override and must not be clobbered.
    if (choice === "system") paint();
  };
  media.addEventListener("change", onChange);

  return () => media.removeEventListener("change", onChange);
}