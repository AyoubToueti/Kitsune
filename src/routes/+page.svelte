<script lang="ts">
  import { onMount } from "svelte";

  import { errorMessage, getTrending } from "$lib/api/anime";
  import HeroBanner from "$lib/components/HeroBanner.svelte";
  import PosterRow from "$lib/components/PosterRow.svelte";
  import type { Anime } from "$lib/types";

  // Enough for a hero plus a scrollable row, without asking AniList for
  // more than a screenful.
  const LIMIT = 24;

  let anime = $state<Anime[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      anime = await getTrending(LIMIT);
    } catch (err) {
      // The backend already renders the cause, so surface it rather than a
      // generic message. A rate limit then looks different from a bug.
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  // ssr is disabled, so this runs once in the browser.
  onMount(load);

  const featured = $derived(anime[0]);
  // The hero already shows the first title, so the row holds the rest.
  const rest = $derived(anime.slice(1));
</script>

{#if loading}
  <p class="py-16 text-center text-ink-muted">Loading…</p>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load trending titles.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
    <button
      type="button"
      onclick={load}
      class="mt-4 rounded-full bg-accent px-5 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
    >
      Try again
    </button>
  </div>
{:else if anime.length === 0}
  <p class="py-16 text-center text-ink-muted">No titles available right now.</p>
{:else}
  {#if featured}
    <HeroBanner anime={featured} />
  {/if}

  <PosterRow title="Trending now" anime={rest} />
{/if}