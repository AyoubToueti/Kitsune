<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime, errorMessage, getGenres, getTags } from "$lib/api/anime";
  import ActiveFilters from "$lib/components/ActiveFilters.svelte";
  import AnimeGrid from "$lib/components/AnimeGrid.svelte";
  import FilterBar from "$lib/components/FilterBar.svelte";
  import Pagination from "$lib/components/Pagination.svelte";
  import { filterHref, parseBrowseQuery } from "$lib/filter";
  import { clampPage } from "$lib/pagination";
  import type { AnimePage, MediaTag } from "$lib/types";

  const PER_PAGE = 30;

  /**
   * The filters, read from the URL.
   *
   * The URL is the single source of truth: a filtered view is shareable, the
   * back button works, and the page needs no state store.
   */
  const params = $derived(page.url.searchParams);
  const query = $derived(parseBrowseQuery(params));
  const requestedPage = $derived(
    Math.max(1, Math.floor(Number(params.get("page") ?? "1")) || 1),
  );

  let result = $state<AnimePage | null>(null);
  let genres = $state<string[]>([]);
  let tags = $state<MediaTag[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Fetched once: neither list changes with the filters.
  getGenres()
    .then((found) => {
      genres = found;
    })
    .catch(() => {
      // A missing genre list means no checkboxes, not a broken page. The other
      // filters still work.
      genres = [];
    });

  getTags()
    .then((found) => {
      tags = found;
    })
    .catch(() => {
      // Same reasoning as genres: a missing tag list is a smaller panel, not
      // a broken page.
      tags = [];
    });

  $effect(() => {
    const current = query;
    const requested = requestedPage;

    // Guards against a stale response overwriting a newer one when the user
    // changes filters quickly.
    let cancelled = false;
    loading = true;
    error = null;

    browseAnime(current, requested, PER_PAGE)
      .then((found) => {
        if (cancelled) return;
        result = found;
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

  /** Page links preserve the filters, since losing them would reset the view. */
  function hrefFor(target: number): string {
    const last = result?.pageInfo.lastPage ?? 1;
    return filterHref(params, clampPage(target, last));
  }
</script>

<h1 class="mb-4 text-lg font-semibold tracking-tight">Filter anime</h1>

<FilterBar {params} {genres} {tags} current={query} />

<div class="mt-4">
  <ActiveFilters {params} current={query} />
</div>

<div class="mt-8">
  {#if loading}
    <p class="py-16 text-center text-ink-muted">Loading…</p>
  {:else if error}
    <div class="py-16 text-center">
      <p class="text-ink">Could not load results.</p>
      <p class="mt-2 text-sm text-ink-faint">{error}</p>
    </div>
  {:else if result && result.items.length === 0}
    <p class="py-16 text-center text-ink-muted">No titles match those filters.</p>
  {:else if result}
    <AnimeGrid anime={result.items} />
    <Pagination
      current={result.pageInfo.currentPage}
      last={result.pageInfo.lastPage}
      {hrefFor}
    />
  {/if}
</div>