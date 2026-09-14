<script lang="ts">
  import { onMount } from "svelte";

  import { browseAnime, errorMessage } from "$lib/api/anime";
  import { formatSeason, nextSeason, seasonHref, seasonQuery } from "$lib/season";
  import type { Anime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";

  /** Six columns at the widest, so two rows. */
  const LIMIT = 12;

  // Computed once: the season cannot change while the page is open, and
  // recomputing it on a re-render would be a wasted allocation.
  const season = nextSeason();
  const label = formatSeason(season.season, season.year);

  let anime = $state<Anime[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      const page = await browseAnime(seasonQuery(season), 1, LIMIT);
      anime = page.items;
    } catch (err) {
      // The backend already renders the cause, so surface it: a rate limit
      // should not look like a bug.
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  // One-shot. The season is fixed for the life of the page, so there is nothing
  // to react to.
  onMount(load);
</script>

<section class="mt-8">
  <div class="mb-3 flex items-baseline justify-between gap-3">
    <h2 class="text-lg font-semibold tracking-tight">Upcoming next season</h2>

    <a
      href={seasonHref(season)}
      class="shrink-0 text-xs text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      View all{label ? ` · ${label}` : ""} ›
    </a>
  </div>

  {#if loading}
    <p class="text-xs text-ink-faint">Loading…</p>
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
    <p class="text-xs text-ink-faint">
      Nothing announced for {label ?? "next season"} yet.
    </p>
  {:else}
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
      {#each anime as item (item.id)}
        <AnimeCard anime={item} />
      {/each}
    </div>
  {/if}
</section>