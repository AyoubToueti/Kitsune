<script lang="ts">
  import { onMount } from "svelte";

  import { browseAnime, errorMessage } from "$lib/api/anime";
  import type { Anime } from "$lib/types";
  import TopList from "./TopList.svelte";
  import TopListSkeleton from "./TopListSkeleton.svelte";

  /**
   * How many rows the preview shows.
   *
   * The full list is 100 at its own route; a landing page does not need that
   * much scrolling, and the reference shows the same short preview.
   */
  const PREVIEW = 10;

  let anime = $state<Anime[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      // Score order, which is what "top rated" means. A popularity floor is
      // deliberately NOT applied: it would hide well-rated short works, and the
      // point of a top list is the ranking rather than the recognition.
      const page = await browseAnime({ sort: "score" }, 1, PREVIEW);
      anime = page.items;
    } catch (err) {
      // Surface the backend's own cause so a rate limit does not look like a bug.
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  // One-shot: the ranking does not depend on anything on this page.
  onMount(load);
</script>

<section class="mt-8">
  <div class="mb-3 flex items-baseline justify-between gap-3">
    <h2 class="text-lg font-semibold tracking-tight">Top 100 Anime</h2>

    <a
      href="/top"
      class="shrink-0 text-xs text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      View All ›
    </a>
  </div>

  {#if loading}
    <div class="rounded-xl border border-border-subtle py-1">
      <TopListSkeleton count={PREVIEW} />
    </div>
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
  {:else if anime.length === 0}
    <p class="text-xs text-ink-faint">Nothing to rank right now.</p>
  {:else}
    <div class="rounded-xl border border-border-subtle py-1">
      <TopList {anime} />
    </div>
  {/if}
</section>