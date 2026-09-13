// Test-only stand-in for SvelteKit's `$app/state`.
//
// That module is virtual: the sveltekit() Vite plugin provides it, and
// vitest.config.js deliberately leaves that plugin out so unit runs do not
// carry the whole framework. Aliasing it here lets components import
// `page` normally while specs control the URL directly.
//
// Not used by the app build — vite.config.js still resolves the real one.
//
// Note: unlike the real `page`, this is not reactive. Each test renders
// fresh, so a static value per render is sufficient.

export const page = {
  url: new URL("http://localhost/"),
};