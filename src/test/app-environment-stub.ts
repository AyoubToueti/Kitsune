// Test-only stand-in for SvelteKit's `$app/environment`.
//
// Like `$app/state` and `$app/navigation`, that module is virtual: the
// sveltekit() plugin provides it and vitest.config.js leaves that plugin out.
// Aliasing it here lets `theme.svelte.ts` import `browser` normally.
//
// The unit suite runs in jsdom, so `browser` is true -- matching what a real
// component sees in the app, and letting the theme module touch `document`
// and `window.matchMedia` (the latter stubbed in setup.js).

/** True in a browser; jsdom counts as one. */
export const browser = true;

/** No SSR build here, so nothing is ever "building". */
export const building = false;

/** No SSR build here. */
export const dev = true;

/** No SSR build here. */
export const version = "test";