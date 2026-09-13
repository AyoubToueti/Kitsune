<script lang="ts">
  import { page } from "$app/state";

  import { errorMessage, getByGenre } from "$lib/api/anime";
  import AnimeCard from "$lib/components/AnimeCard.svelte";
  import type { Anime } from "$lib/types";

  const LIMIT = 30;

  // SvelteKit decodes route params, so a "Slice%20of%20Life" path arrives here
  // already readable. Decoding again could corrupt a name containing a literal
  // percent, so it is used as-is.
  const genre = $derived(page.params.name ?? "");

  let results = $state<Anime[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  // Distinguishes "nothing yet" from "loaded and found nothing", which need
  // different copy.
  let loadedFor = $state("");

  $effect(() => {
    const current = genre;

    if (current === "") {
      results = [];
      error = null;
      loading = false;
      loadedFor = "";
      return;
    }

    // Guards against a stale response overwriting a newer one when the user
    // navigates between genres quickly.
    let cancelled = false;
    loading = true;
    error = null;

    getByGenre(current, LIMIT)
      .then((found) => {
        if (cancelled) return;
        results = found;
        loadedFor = current;
      })
      .catch((err) => {
        if (cancelled) return;
        error = errorMessage(err);
      })
      .finally(() => {
        if (cancelled) return;
        loading = false;
      });

    return () => {
      cancelled = true;
    };
  });
</script>

{#if genre === ""}
  <p class="py-16 text-center text-ink-muted">No genre selected.</p>
{:else if loading}
  <p class="py-16 text-center text-ink-muted">Loading…</p>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load that genre.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if results.length === 0}
  <p class="py-16 text-center text-ink-muted">
    Nothing found in “{loadedFor}”.
  </p>
{:else}
  <h1 class="mb-4 text-lg font-semibold tracking-tight">{loadedFor}</h1>

  <ul class="flex flex-wrap gap-4">
    {#each results as item (item.id)}
      <li>
        <AnimeCard anime={item} />
      </li>
    {/each}
  </ul>
{/if}