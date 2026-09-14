// Test-only stand-in for SvelteKit's `$app/navigation`.
//
// Like `$app/state`, that module is virtual: the sveltekit() plugin provides
// it and vitest.config.js leaves that plugin out. Aliasing it here lets a
// component import `goto` normally while a spec inspects where it went.
//
// Not used by the app build — vite.config.js still resolves the real one.

/** Every path `goto` has been called with since the last reset. */
export const navigations: string[] = [];

/**
 * Record a navigation instead of performing one.
 *
 * Navigation is a side effect the unit suite has no router for, so it is
 * captured rather than executed. Specs assert on `navigations` and clear it
 * between tests.
 */
export function goto(url: string): Promise<void> {
  navigations.push(url);
  return Promise.resolve();
}

/** Forget recorded navigations. Called from a spec's `beforeEach`. */
export function clearNavigations(): void {
  navigations.length = 0;
}