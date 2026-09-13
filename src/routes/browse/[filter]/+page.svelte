<script lang="ts">
  import { page } from "$app/state";

  import { errorMessage, getList } from "$lib/api/anime";
  import AnimeCard from "$lib/components/AnimeCard.svelte";
  import type { Anime, ListFilter } from "$lib/types";

  const LIMIT = 30;

  /**
   * The accepted filter values, as a runtime list.
   *
   * `ListFilter` is a type, so it cannot be checked at runtime; this array is
   * what lets an unknown URL segment be rejected instead of sent to the
   * backend, where it would fail deserialisation with a confusing message.
   */
  const FILTERS: ListFilter[] = [
    "trending",
    "topAiring",
    "mostPopular",
    "topRated",
    "latestCompleted",
    "upcoming",
  ];

  /** Human labels, so the heading is not the camelCase wire value. */
  const LABELS: Record<ListFilter, string> = {
    trending: "Trending",
    topAiring: "Top airing",
    mostPopular: "Most popular",
    topRated: "Top rated",
    latestCompleted: "Latest completed",
    upcoming: "Upcoming",
  };

  const raw = $derived(page.params.filter ?? "");
  const filter = $derived(
    FILTERS.includes(raw as ListFilter) ? (raw as ListFilter) : null,
  );

  let results = $state<Anime[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let loadedFor = $state<ListFilter | null>(null);

  $effect(() => {
    const current = filter;

    if (current === null) {
      results = [];
      loading = false;
      error = null;
      loadedFor = null;
      return;
    }

    // Guards against a stale response overwriting a newer one when the user
    // moves between filters quickly.
    let cancelled = false;
    loading = true;
    error = null;

    getList(current, LIMIT)
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

{#if filter === null}
  <p class="py-16 text-center text-ink-muted">Unknown list.</p>
{:else if loading}
  <p class="py-16 text-center text-ink-muted">Loading…</p>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load that list.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if results.length === 0}
  <p class="py-16 text-center text-ink-muted">
    Nothing in {loadedFor ? LABELS[loadedFor].toLowerCase() : "this list"} right now.
  </p>
{:else}
  <h1 class="mb-4 text-lg font-semibold tracking-tight">
    {loadedFor ? LABELS[loadedFor] : "List"}
  </h1>

  <ul class="flex flex-wrap gap-4">
    {#each results as item (item.id)}
      <li>
        <AnimeCard anime={item} />
      </li>
    {/each}
  </ul>
{/if}