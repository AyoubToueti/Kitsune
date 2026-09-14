// A small TTL cache for the API wrappers.
//
// Every request the UI makes funnels through `$lib/api/anime`, so caching here
// covers every caller without each one needing to know about it. The cache is
// in-memory and dies with the page: AniList data is not precious, and a
// persistent store would need versioning and eviction for a restart that
// happens rarely.
//
// Two behaviours beyond a plain TTL:
//   - In-flight de-duplication: two callers asking for the same key at once
//     share a single request rather than racing two.
//   - LRU eviction at a fixed cap, so unbounded browse keys cannot leak.

/** How long each kind of result stays fresh, in milliseconds. */
export const API_TTL = {
  /** The genre and tag catalogue is effectively static. */
  catalogue: 60 * 60 * 1000,
  /** Metadata for a single title drifts slowly. */
  detail: 30 * 60 * 1000,
  /** Rankings and browse results shift hour to hour. */
  list: 5 * 60 * 1000,
  /** Airing times move, so the schedule is the least safe to hold. */
  schedule: 60 * 1000,
} as const;

/**
 * Upper bound on stored entries.
 *
 * A browse key encodes the whole query, so an unbounded map would grow with
 * every filter combination the reader tries. The cap keeps memory flat; the
 * LRU order means the entries evicted are the ones least recently read.
 */
const MAX_ENTRIES = 200;

interface Entry {
  value: unknown;
  /** Epoch milliseconds after which the entry is stale. */
  expires: number;
}

/**
 * Insertion order is access order: a read re-inserts its key, so the first key
 * is always the least recently used.
 */
const store = new Map<string, Entry>();

/** Promises for keys currently being fetched, so racers can share one. */
const inFlight = new Map<string, Promise<unknown>>();

/** Drop least-recently-used entries until the store is within its cap. */
function evictOverflow(): void {
  while (store.size > MAX_ENTRIES) {
    const oldest = store.keys().next().value;
    if (oldest === undefined) return;
    store.delete(oldest);
  }
}

/**
 * Resolve `key` from the cache, or run `load` and remember the result.
 *
 * A rejection is deliberately NOT cached: a rate limit or a transient network
 * failure should be retried on the next call, not stick for the whole TTL.
 */
export async function cached<T>(
  key: string,
  ttlMs: number,
  load: () => Promise<T>,
): Promise<T> {
  const hit = store.get(key);
  if (hit !== undefined) {
    if (hit.expires > Date.now()) {
      // Refresh recency so a colder key is the one evicted.
      store.delete(key);
      store.set(key, hit);
      return hit.value as T;
    }
    // Stale: drop it and fall through to a fresh load.
    store.delete(key);
  }

  const pending = inFlight.get(key);
  if (pending !== undefined) {
    return pending as Promise<T>;
  }

  // `load` is invoked before the in-flight entry is recorded, but that is safe:
  // JavaScript is single-threaded, so a second caller cannot interleave between
  // the call and the `set` on the next line.
  const promise = load()
    .then((value) => {
      store.set(key, { value, expires: Date.now() + ttlMs });
      evictOverflow();
      return value;
    })
    .finally(() => {
      inFlight.delete(key);
    });

  inFlight.set(key, promise);
  return promise;
}

/**
 * Empty the cache.
 *
 * A test seam: without it, a value cached by one test would leak into the next
 * and make the suite order-dependent.
 */
export function clearApiCache(): void {
  store.clear();
  inFlight.clear();
}