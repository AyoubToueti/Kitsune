<script lang="ts">
  import { onMount } from "svelte";

  import { errorMessage } from "$lib/api/anime";
  import type { Anime, ListFilter } from "$lib/types";
  import ListBlock from "./ListBlock.svelte";
  import AnimeCardSkeleton from "./AnimeCardSkeleton.svelte";

  let {
    title,
    filter,
    load,
  }: {
    title: string;
    filter: ListFilter;
    load: () => Promise<Anime[]>;
  } = $props();

  let anime = $state<Anime[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function run() {
    loading = true;
    error = null;
    try {
      anime = await load();
    } catch (err) {
      // The backend already renders the cause, so surface it rather than a
      // generic message. A rate limit then looks different from a bug.
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  // One-shot, like `Shelf`: `load` is a fresh closure per parent render, so an
  // effect would treat it as a changed dependency and refetch in a loop.
  onMount(run);
</script>

{#if loading}
  <!-- The heading is kept during loading so the grid does not reflow as the
       four blocks arrive at different times. -->
  <section class="rounded-xl border border-border-subtle p-3">
    <h2 class="mb-2 text-sm font-semibold tracking-tight text-accent">
      {title}
    </h2>
    <div role="status" aria-busy="true" class="flex gap-3 overflow-hidden">
      <span class="sr-only">Loading…</span>
      {#each Array(4) as _, i (i)}
        <div class="w-24 shrink-0">
          <AnimeCardSkeleton />
        </div>
      {/each}
    </div>
  </section>
{:else if error}
  <section class="rounded-xl border border-border-subtle p-3">
    <h2 class="mb-2 text-sm font-semibold tracking-tight text-accent">
      {title}
    </h2>
    <p class="text-xs text-ink-faint">
      Could not load. <span class="text-ink-muted">{error}</span>
    </p>
    <button
      type="button"
      onclick={run}
      class="mt-2 rounded-full border border-border-subtle bg-surface-hover px-3 py-1 text-xs text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      Try again
    </button>
  </section>
{:else}
  <ListBlock {title} {anime} {filter} />
{/if}