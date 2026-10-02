// App-wide per-work watch progress.
//
// A grid of cards needs to know, for each work, how far the reader got -- so a
// Watch button can resume instead of always starting at episode one. Asking the
// backend per card would be one command per tile; instead the whole list is
// read once and answered from memory.
//
// The list is small (the reader's own entries) and changes only through this
// app, so a single read plus a refresh on the `list-changed` event is enough.
// Same shape as `now-playing.svelte.ts`: a lazy singleton under an
// `$effect.root`, so the subscription outlives any one card.

import { getUserList, onListChanged } from "./api/auth";
import type { UnlistenFn } from "@tauri-apps/api/event";

/**
 * Progress by media id, in episodes watched (1-based, matching AniList).
 *
 * A plain object rather than a `Map` so a `$state` proxy tracks it; the getter
 * reads it reactively.
 */
let progress = $state<Record<number, number>>({});

/** Whether the first read has completed. */
let loaded = $state(false);

/** The list subscription, once it resolves. */
let unlisten: UnlistenFn | null = null;

/** A refresh in flight, so overlapping events do not stack reads. */
let refreshing = false;

/** Read the whole list and rebuild the progress table. */
async function refresh(): Promise<void> {
  if (refreshing) return;
  refreshing = true;
  try {
    const entries = await getUserList();
    const next: Record<number, number> = {};
    for (const entry of entries) {
      // Only keep works actually started; a `planning` entry at 0 is the same
      // as not being on the list for a resume decision.
      if (entry.progress > 0) next[entry.anime.id] = entry.progress;
    }
    progress = next;
  } catch {
    // Signed out, or the read failed: treat as "nothing watched". The button
    // then falls back to episode one, which is always a safe action.
    progress = {};
  } finally {
    loaded = true;
    refreshing = false;
  }
}

/**
 * Start the subscription once, on first use.
 *
 * Lazy rather than at import time so importing this module does not reach
 * Tauri (every test that touches the store would otherwise need the runtime).
 */
let started = false;

function ensureStarted(): void {
  if (started) return;
  started = true;

  $effect.root(() => {
    void refresh();

    onListChanged(() => {
      void refresh();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // No listener means the table will not refresh while the app is open;
        // the initial read still populated it.
      });
  });
}

/**
 * Episodes watched for a work, `0` when none.
 *
 * Reading this is what starts the store, so a component that renders a card
 * gets a live answer without any explicit setup.
 */
export function progressFor(animeId: number): number {
  ensureStarted();
  return progress[animeId] ?? 0;
}

/** Whether the initial read has completed, for callers that care about a flash. */
export function progressLoaded(): boolean {
  ensureStarted();
  return loaded;
}

/** Tear the subscription down. Tests only; the app lives until the process ends. */
export function stopWatchProgress(): void {
  unlisten?.();
  unlisten = null;
  started = false;
  progress = {};
  loaded = false;
}