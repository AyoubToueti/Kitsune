// A component-free harness for `createInfiniteScroll`.
//
// The composable uses runes, so exercising it needs a reactive context. A
// `.svelte.ts` module has one and `$effect.root` supplies the rest, which keeps
// the tests free of a throwaway component.
//
// The teardown MUST be called by each test: an effect root outlives the test
// that made it, and a leaked one keeps its subscriptions alive into the next.

import { flushSync } from "svelte";

import { createInfiniteScroll, type InfiniteScroll } from "$lib/infinite-scroll.svelte";
import type { Anime, AnimePage } from "$lib/types";

export interface Harness {
  readonly scroll: InfiniteScroll<Anime>;
  /** Swap the query key, as a new filter or search term would. */
  setKey(next: string): void;
  destroy(): void;
}

export function harness(
  fetchPage: (page: number) => Promise<AnimePage>,
): Harness {
  let key = $state("initial");
  let scroll!: InfiniteScroll<Anime>;

  const destroy = $effect.root(() => {
    scroll = createInfiniteScroll(() => key, fetchPage);
  });

  // `$effect` is scheduled, not run inline when the root is created, so the
  // initial fetch would otherwise wait for the first `setKey`. Flushing here
  // runs it now, as mounting a real component would.
  flushSync();

  return {
    get scroll() {
      return scroll;
    },
    setKey(next: string) {
      key = next;
      flushSync();
    },
    destroy,
  };
}

/**
 * Let pending promises settle, then flush what they triggered.
 *
 * Two microtask hops: the first resolves the fetch, the second lets the
 * `finally` block run before anything asserts on `loading`.
 */
export async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
}