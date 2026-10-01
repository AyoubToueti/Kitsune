// Searching the indexers for a work's releases, and probing them for swarm
// health — one state machine, shared by the watch page and the episode modal.
//
// Two things happen here that always happen together: a search keyed on the
// work's titles and the wanted episode, and a probe of whatever that search
// returned. A caller that did only one of them would show either an unranked
// list or no list at all.
//
// A `.svelte.ts` module so it can own runes; a component that calls
// `createReleaseSearch` gets the effects, and a test can wrap it in
// `$effect.root` (see `src/test/release-search-harness.svelte.ts`).

import { searchReleases, onProbeResult, probeReleases } from "./api/releases";
import { errorMessage } from "./api/anime";
import { titleForms, type ProbeOutcome, type Release } from "./types";

/** What to search for. `null` means "nothing to search yet". */
export interface ReleaseRequest {
  /** Every title form the work has, best first. */
  titles: string[];
  /** Episode number relative to the cour, when one is selected. */
  episode?: number;
  /** Work-wide episode number, when it differs from `episode`. */
  absoluteEpisode?: number;
}

/** The reactive surface a caller renders. */
export interface ReleaseSearch {
  readonly releases: Release[];
  readonly searching: boolean;
  readonly error: string | null;
  readonly probeOutcomes: (ProbeOutcome | undefined)[];
  readonly probing: boolean;
  /** The badge for the release at `index`, or undefined while unprobed. */
  badgeFor(index: number): ProbeOutcome | undefined;
}

/**
 * Run a search and a probe whenever the request key changes.
 *
 * `getRequest` is a function rather than a value so the composable reads it
 * reactively: a component can derive the request from its own state and this
 * re-runs when it changes, without the caller having to know when to reset.
 */
export function createReleaseSearch(
  getRequest: () => ReleaseRequest | null,
): ReleaseSearch {
  let releases = $state<Release[]>([]);
  let searching = $state(false);
  let error = $state<string | null>(null);
  // Index-aligned with `releases` rather than keyed by info hash: a release is
  // not guaranteed to carry a hash, and an index cannot go missing.
  let probeOutcomes = $state<(ProbeOutcome | undefined)[]>([]);
  let probing = $state(false);

  // --- the search ----------------------------------------------------------
  //
  // The titles and the episode number are the whole key. A search in flight is
  // abandoned when the key changes, so a slow response for episode 1 cannot
  // overwrite the results for episode 2.
  $effect(() => {
    const request = getRequest();

    if (request === null || request.titles.length === 0) {
      releases = [];
      return;
    }

    let cancelled = false;
    searching = true;
    error = null;

    searchReleases(request.titles, request.episode, request.absoluteEpisode)
      .then((found) => {
        if (cancelled) return;
        releases = found;
      })
      .catch((err) => {
        if (cancelled) return;
        releases = [];
        error = errorMessage(err);
      })
      .finally(() => {
        if (cancelled) return;
        searching = false;
      });

    return () => {
      cancelled = true;
    };
  });

  // --- the probe -----------------------------------------------------------
  //
  // Keyed on the release list itself, so it runs when a search produces a new
  // set, not when unrelated state changes. Verdicts arrive as events and are
  // written in by position; the promise's return value is ignored because the
  // events already delivered it.
  //
  // A failure is swallowed: probing enhances the ranking, so an unreachable DHT
  // should leave the static order alone rather than surface an error the reader
  // cannot act on.
  $effect(() => {
    const found = releases;
    if (found.length === 0) return;

    let cancelled = false;
    probing = true;
    // Reset to the list's length so an index always lands in bounds, even for
    // an event that arrives before this effect finishes setting up.
    probeOutcomes = new Array(found.length).fill(undefined);

    const unlisten = onProbeResult((outcome) => {
      if (cancelled) return;
      if (outcome.index < 0 || outcome.index >= probeOutcomes.length) return;
      // Replace the array rather than mutating a slot: Svelte tracks the
      // binding, and an in-place write would not re-run a derived ranking.
      const next = probeOutcomes.slice();
      next[outcome.index] = outcome;
      probeOutcomes = next;
    });

    probeReleases(found)
      .catch(() => {
        // Already handled by the events; nothing to surface.
      })
      .finally(() => {
        if (cancelled) return;
        probing = false;
      });

    return () => {
      cancelled = true;
      unlisten.then((fn) => fn());
    };
  });

  return {
    get releases() {
      return releases;
    },
    get searching() {
      return searching;
    },
    get error() {
      return error;
    },
    get probeOutcomes() {
      return probeOutcomes;
    },
    get probing() {
      return probing;
    },
    badgeFor(index: number) {
      return probeOutcomes[index];
    },
  };
}

/**
 * Build the request key for a search from a work's title and episode numbers.
 *
 * A free function rather than inline in the caller so the "titles empty means
 * no search" rule lives in one place. `absoluteEpisode` is only meaningful when
 * it differs from `episode`; the caller is expected to pass `undefined` when
 * they agree, but this stays lenient.
 */
export function releaseRequest(
  title: { romaji?: string; english?: string; native?: string; userPreferred?: string } | undefined,
  episode?: number,
  absoluteEpisode?: number,
): ReleaseRequest | null {
  if (!title) return null;
  const titles = titleForms(title);
  if (titles.length === 0) return null;
  return { titles, episode, absoluteEpisode };
}