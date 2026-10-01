// A component-free harness for `createReleaseSearch`.
//
// The composable uses runes, so exercising it needs a reactive context. A
// `.svelte.ts` module has one and `$effect.root` supplies the rest, which keeps
// the tests free of a throwaway component.
//
// The teardown MUST be called by each test: an effect root outlives the test
// that made it, and a leaked one keeps its subscriptions alive into the next.

import { flushSync } from "svelte";

import {
  createReleaseSearch,
  type ReleaseRequest,
  type ReleaseSearch,
} from "$lib/release-search.svelte";

export interface Harness {
  readonly search: ReleaseSearch;
  /** Swap the request, as a new episode selection would. */
  setRequest(next: ReleaseRequest | null): void;
  destroy(): void;
}

export function harness(initial: ReleaseRequest | null = null): Harness {
  let request = $state<ReleaseRequest | null>(initial);
  let search!: ReleaseSearch;

  const destroy = $effect.root(() => {
    search = createReleaseSearch(() => request);
  });

  flushSync();

  return {
    get search() {
      return search;
    },
    setRequest(next) {
      request = next;
      flushSync();
    },
    destroy,
  };
}

/**
 * Let pending promises settle, then flush what they triggered.
 *
 * Two microtask hops: the first resolves the fetch, the second lets the
 * `finally` block run before anything asserts on `searching`.
 */
export async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
}