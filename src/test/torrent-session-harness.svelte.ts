// A component-free harness for `createTorrentSession`.
//
// The composable uses runes, so exercising it needs a reactive context. A
// `.svelte.ts` module has one and `$effect.root` supplies the rest, which keeps
// the tests free of a throwaway component.
//
// The teardown MUST be called by each test: an effect root outlives the test
// that made it, and a leaked one keeps its subscriptions (and its torrents)
// alive into the next.

import { flushSync } from "svelte";

import {
  createTorrentSession,
  type TorrentSession,
} from "$lib/torrent-session.svelte";

export interface Harness {
  readonly session: TorrentSession;
  setEpisode(next: number | undefined): void;
  setOffset(next: number): void;
  destroy(): void;
}

export function harness(options: {
  episode?: number;
  offset?: number;
  launched?: () => void;
  /**
   * Whether the session should prompt for a player, mirroring the setting.
   *
   * Defaults to `true` (prompt), so tests that do not care get the picker.
   */
  askEveryTime?: boolean;
} = {}): Harness {
  let episode = $state<number | undefined>(options.episode);
  let offset = $state(options.offset ?? 0);
  let session!: TorrentSession;

  const destroy = $effect.root(() => {
    session = createTorrentSession({
      getId: () => 42,
      getEpisode: () => episode,
      getEpisodeOffset: () => offset,
      onLaunched: options.launched,
      getAskEveryTime: () => options.askEveryTime ?? true,
    });
  });

  flushSync();

  return {
    get session() {
      return session;
    },
    setEpisode(next) {
      episode = next;
      flushSync();
    },
    setOffset(next) {
      offset = next;
      flushSync();
    },
    destroy,
  };
}

/**
 * Let pending promises settle, then flush what they triggered.
 *
 * Several microtask hops so a chain of awaits (getStreamUrl -> poll ->
 * getTorrentStats -> launch) all resolve before anything asserts. `flushSync`
 * between hops runs the effects those writes scheduled.
 */
export async function settle(rounds = 6): Promise<void> {
  for (let i = 0; i < rounds; i += 1) {
    await Promise.resolve();
    flushSync();
  }
}