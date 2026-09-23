<script lang="ts">
  import { onMount } from "svelte";

  import { errorMessage } from "$lib/api/anime";
  import { getContinueWatching } from "$lib/api/auth";
  import type { Anime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";
  import AnimeGridSkeleton from "./AnimeGridSkeleton.svelte";

  /** Six columns at the widest, so two rows. */
  const LIMIT = 12;

  let anime = $state<Anime[]>([]);
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
  const visible = $derived(!loading && (error !== null || anime.length > 0));

  async function load() {
    loading = true;
    error = null;
    try {
      anime = await getContinueWatching(LIMIT);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  onMount(load);
</script>

{#if visible}
  <section class="mt-8">
    <div class="mb-3 flex items-baseline justify-between gap-3">
      <h2 class="text-lg font-semibold tracking-tight">Continue Watching</h2>
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
        {#each anime as item (item.id)}
          <AnimeCard anime={item} />
        {/each}
      </div>
    {/if}
  </section>
{/if}