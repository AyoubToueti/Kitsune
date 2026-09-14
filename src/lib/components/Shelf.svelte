<script lang="ts">
  import { onMount } from "svelte";

  import { errorMessage } from "$lib/api/anime";
  import type { Anime } from "$lib/types";
  import PosterRow from "./PosterRow.svelte";
  import AnimeCardSkeleton from "./AnimeCardSkeleton.svelte";

  let {
    title,
    load,
    numbered = false,
  }: {
    title: string;
    load: () => Promise<Anime[]>;
    numbered?: boolean;
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

  // Deliberately `onMount`, not `$effect`. These shelves are one-shot: there is
  // nothing to react to, and an effect would instead re-fetch whenever the
  // `load` prop changed identity. That is easy to trigger by accident -- an
  // inline arrow passed from the parent is a new function every render, which
  // would turn each parent update into a refetch.
  onMount(run);
</script>

{#if loading}
  <!-- The heading is rendered here rather than left to PosterRow so the
       section does not pop into existence and shift the page as it loads. -->
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">{title}</h2>
    <div role="status" aria-busy="true" class="flex gap-4 overflow-hidden">
      <span class="sr-only">Loading…</span>
      {#each Array(6) as _, i (i)}
        <AnimeCardSkeleton />
      {/each}
    </div>
  </section>
{:else if error}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">{title}</h2>
    <p class="text-sm text-ink-faint">
      Could not load this row. <span class="text-ink-muted">{error}</span>
    </p>
    <button
      type="button"
      onclick={run}
      class="mt-2 rounded-full border border-border-subtle bg-surface-hover px-4 py-1 text-sm text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      Try again
    </button>
  </section>
{:else}
  <!-- PosterRow renders nothing at all for an empty list, which is the right
       outcome here: an empty shelf is noise. -->
  <PosterRow {title} {anime} {numbered} />
{/if}