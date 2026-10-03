// Load-as-you-scroll for the paged browse routes.
//
// Three routes need the same thing -- accumulate pages into one growing list as
// the reader approaches the bottom -- and each would otherwise carry its own
// copy of the same async state machine. That is where the bugs live, so it
// lives here once.
//
// The URL is deliberately NOT kept in step with the page count. An infinite
// list has no stable "page 5": the reader's position is a scroll offset, and
// writing that into the URL would produce a link that opens halfway down a list
// nobody else has loaded. The URL stays the entry point, and a refresh starts
// from the top.

import { untrack } from "svelte";

import { errorMessage } from "./api/anime";
import type { PageInfo } from "./types";

/** One page of a paged endpoint: the items plus where they sit in the set. */
export interface Paged<T> {
  items: T[];
  pageInfo: PageInfo;
}

/** How close to the bottom the sentinel triggers, in pixels. */
const PRELOAD_MARGIN = 600;

export interface InfiniteScroll<T> {
  /** Everything loaded so far, oldest first. */
  readonly items: T[];
  /** True until the FIRST page settles, so the page can show a spinner. */
  readonly loading: boolean;
  /** True while a later page is in flight, for a subtle footer spinner. */
  readonly loadingMore: boolean;
  readonly error: string | null;
  /** Whether another page exists to fetch. */
  readonly hasMore: boolean;
  /** Fetch the next page, if there is one and nothing is in flight. */
  loadMore: () => void;
  /** Abandon the failed request and try the same page again. */
  retry: () => void;
  /**
   * An action for the element that should trigger loading.
   *
   * Returns a no-op when `IntersectionObserver` is unavailable -- jsdom has no
   * layout engine, so tests drive `loadMore` directly rather than pretending to
   * scroll.
   */
  sentinel: (node: HTMLElement) => { destroy: () => void };
}

/**
 * Accumulate pages of a browse query as the reader scrolls.
 *
 * `key` identifies the current query. When it changes -- a new filter, a new
 * search term -- the list resets and refetches from page one. It must be a
 * function so the composable reads it reactively without the caller having to
 * know when to reset.
 *
 * `fetchPage` is 1-based, matching the provider.
 */
export function createInfiniteScroll<T>(
  key: () => string,
  fetchPage: (page: number) => Promise<Paged<T>>,
): InfiniteScroll<T> {
  let items = $state<T[]>([]);
  let loadedPage = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);
  /**
   * Whether the last page said another exists.
   *
   * Taken from `pageInfo.hasNextPage`, NOT derived from `lastPage`: AniList
   * reports `lastPage` inconsistently for some connections (the recommendations
   * connection among them), so `loadedPage < lastPage` could be false on page 1
   * and strand the list. The provider's own `hasNextPage` is authoritative.
   */
  let hasNextPage = $state(false);

  /** Guards against a second request starting while one is in flight. */
  let inFlight = false;

  /**
   * Which query the in-flight request belongs to.
   *
   * Bumped whenever the key changes. A response only applies if its generation
   * still matches, so a slow request for the previous filter cannot append its
   * results to the new list -- the stale-response guard every other route here
   * has, and the reason they cancel via a local `cancelled` flag.
   */
  let generation = 0;

  const hasMore = $derived(hasNextPage);

  async function fetchNext(): Promise<void> {
    const target = loadedPage + 1;
    const mine = generation;
    inFlight = true;

    if (target === 1) {
      loading = true;
    } else {
      loadingMore = true;
    }
    error = null;

    try {
      const page = await fetchPage(target);

      // The query changed while this was in flight, so these results belong to
      // a filter the reader has already moved on from.
      if (mine !== generation) return;

      // Append rather than replace: the previous pages are still on screen, and
      // discarding them would make the list jump.
      items = [...items, ...page.items];
      loadedPage = page.pageInfo.currentPage;
      hasNextPage = page.pageInfo.hasNextPage;
    } catch (err) {
      if (mine !== generation) return;
      // The backend renders the cause, so surface it: a rate limit should not
      // look like a bug.
      error = errorMessage(err);
    } finally {
      // Only the current generation clears the flags: a superseded request
      // finishing must not report "not loading" for the one that replaced it.
      if (mine === generation) {
        inFlight = false;
        loading = false;
        loadingMore = false;
      }
    }
  }

  function loadMore(): void {
    // `hasMore` already accounts for the loaded page, but a failed request
    // leaves `loadedPage` where it was, so without `error` this would retry
    // forever the moment the sentinel is visible.
    if (inFlight || error !== null || !hasMore) return;
    void fetchNext();
  }

  function retry(): void {
    if (inFlight) return;
    error = null;
    void fetchNext();
  }

  function sentinel(node: HTMLElement): { destroy: () => void } {
    // No observer in jsdom, and nothing to scroll in a headless environment.
    if (typeof IntersectionObserver === "undefined") {
      return { destroy: () => {} };
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) loadMore();
      },
      { rootMargin: `${PRELOAD_MARGIN}px` },
    );

    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }

  $effect(() => {
    // Read the key so a new query re-runs this effect, then untrack everything
    // else. This read is LOAD-BEARING, not leftover: it is the effect's only
    // reactive dependency. Removing it (as a "unused variable" tidy-up once
    // did) makes the effect run exactly once, so a filter or search change
    // never resets or refetches the list.
    void key();

    // Everything else is untracked: `fetchNext` writes `loadedPage` and
    // `items`, which would otherwise be read as dependencies of this same
    // effect and re-trigger it -- the loop that left ScheduleWidget stuck on
    // "Loading…".
    untrack(() => {
      // Bump BEFORE resetting: any request still in flight belongs to the
      // previous key and must not apply when it lands.
      generation += 1;

      items = [];
      loadedPage = 0;
      hasNextPage = false;
      error = null;
      loading = true;
      inFlight = false;
      void fetchNext();
    });
  });

  return {
    get items() {
      return items;
    },
    get loading() {
      return loading;
    },
    get loadingMore() {
      return loadingMore;
    },
    get error() {
      return error;
    },
    get hasMore() {
      return hasMore;
    },
    loadMore,
    retry,
    sentinel,
  };
}