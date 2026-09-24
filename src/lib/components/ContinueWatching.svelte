<script lang="ts">
  import { onDestroy, onMount } from "svelte";

  import { errorMessage } from "$lib/api/anime";
  import { getContinueWatching, onListChanged } from "$lib/api/auth";
  import { resumeIndex } from "$lib/resume";
  import type { ContinueWatchingItem } from "$lib/types";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import AnimeCard from "./AnimeCard.svelte";
  import AnimeGridSkeleton from "./AnimeGridSkeleton.svelte";

  /** Six columns at the widest, so two rows. */
  const LIMIT = 12;

  let items = $state<ContinueWatchingItem[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  /**
   * Whether the reader has anything to continue.
   *
   * Nothing is rendered until the answer is known: the backend answers an
   * empty list when signed out, so a skeleton shown during loading would flash
   * a "Continue Watching" heading and then vanish for every signed-out reader
   * -- which is most home-page loads.
   */
  const visible = $derived(!loading && (error !== null || items.length > 0));

  async function load() {
    loading = true;
    error = null;
    try {
      items = await getContinueWatching(LIMIT);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  let unlisten: UnlistenFn | null = null;

  onMount(() => {
    let cancelled = false;
    load();

    // A write from the watch page (recording an episode) or the My List page
    // (a status change, a removal) re-reads this row, so a work just watched
    // appears without a reload.
    onListChanged(() => {
      void load();
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // The row still loads on mount; a missing listener only means it will
        // not refresh while visible.
      });

    return () => {
      cancelled = true;
    };
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

{#if visible}
  <section class="mt-8">
    <div class="mb-3 flex items-baseline justify-between gap-3">
      <h2 class="text-lg font-semibold tracking-tight">Continue Watching</h2>

      <a
        href="/list"
        class="shrink-0 text-xs text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        View all ›
      </a>
    </div>

    {#if loading}
      <AnimeGridSkeleton count={LIMIT} />
    {:else if error}
      <p class="text-xs text-ink-faint">
        Could not load. <span class="text-ink-muted">{error}</span>
      </p>
      <button
        type="button"
        onclick={load}
        class="mt-2 rounded-full border border-border-subtle bg-surface-hover px-3 py-1 text-xs text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        Try again
      </button>
    {:else}
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
        {#each items as entry (entry.anime.id)}
          <div class="relative">
            <AnimeCard anime={entry.anime} />
            <!-- A sibling of the card's link, not a child: nesting an anchor
                 inside an anchor is invalid HTML. Positioned over the card's
                 bottom-right corner, which is the "resume" affordance. -->
            <a
              href={`/watch/${entry.anime.id}?ep=${resumeIndex(entry.progress)}`}
              data-testid="resume"
              aria-label={`Resume ${entry.anime.title.romaji ?? "this work"}`}
              title={entry.progress > 0
                ? `Resume at episode ${entry.progress}`
                : "Start watching"}
              class="absolute right-1 bottom-1 flex h-8 w-8 items-center justify-center rounded-full bg-accent text-sm text-white shadow-lg transition-transform hover:scale-110 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              <span aria-hidden="true">▶</span>
            </a>
          </div>
        {/each}
      </div>
    {/if}
  </section>
{/if}