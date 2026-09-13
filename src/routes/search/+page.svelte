<script lang="ts">
  import { page } from "$app/state";

  import { errorMessage, searchAnime } from "$lib/api/anime";
  import AnimeCard from "$lib/components/AnimeCard.svelte";
  import type { Anime } from "$lib/types";

  const LIMIT = 30;

  // Reactive: the navbar submits a GET to /search, so the query string
  // changes without the component unmounting.
  const query = $derived((page.url.searchParams.get("q") ?? "").trim());

  let results = $state<Anime[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  // Distinguishes "nothing yet" from "searched and found nothing", which
  // need different copy.
  let searched = $state("");

  $effect(() => {
    const q = query;

    if (q === "") {
      results = [];
      error = null;
      loading = false;
      searched = "";
      return;
    }

    // Guards against a stale response overwriting a newer one when the
    // user searches again quickly.
    let cancelled = false;
    loading = true;
    error = null;

    searchAnime(q, LIMIT)
      .then((found) => {
        if (cancelled) return;
        results = found;
        searched = q;
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

{#if query === ""}
  <p class="py-16 text-center text-ink-muted">Type something in the search box above.</p>
{:else if loading}
  <p class="py-16 text-center text-ink-muted">Searching…</p>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not run that search.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if results.length === 0}
  <p class="py-16 text-center text-ink-muted">
    No results for “{searched}”.
  </p>
{:else}
  <h1 class="mb-4 text-lg font-semibold tracking-tight">
    Results for “{searched}”
  </h1>

  <ul class="flex flex-wrap gap-4">
    {#each results as item (item.id)}
      <li>
        <AnimeCard anime={item} />
      </li>
    {/each}
  </ul>
{/if}