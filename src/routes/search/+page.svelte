<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime, getGenres, getTags } from "$lib/api/anime";
  import ActiveFilters from "$lib/components/ActiveFilters.svelte";
  import AnimeGrid from "$lib/components/AnimeGrid.svelte";
  import FilterBar from "$lib/components/FilterBar.svelte";
  import { parseBrowseQuery } from "$lib/filter";
  import { createInfiniteScroll } from "$lib/infinite-scroll.svelte";
  import type { AnimePage, MediaTag } from "$lib/types";

  /** AniList caps perPage at 50; the backend clamps anything larger. */
  const PER_PAGE = 30;

  const params = $derived(page.url.searchParams);

  /**
   * The search term.
   *
   * `q`, not `search`: the navbar's search box owns this parameter, and the
   * filter panel reads `search`. Keeping them distinct means a filtered search
   * cannot accidentally overwrite the term the user typed.
   */
  const term = $derived((params.get("q") ?? "").trim());

  /** The filters applied on top of the search, read from the same URL. */
  const filters = $derived(parseBrowseQuery(params));

  let genres = $state<string[]>([]);
  let tags = $state<MediaTag[]>([]);

  /** An empty result, used when there is no term to search for. */
  function emptyPage(): AnimePage {
    return {
      items: [],
      pageInfo: { total: 0, currentPage: 1, lastPage: 1, hasNextPage: false },
    };
  }

  /**
   * Results, grown a page at a time as the reader scrolls.
   *
   * The key folds in the term and the filters, so changing either resets the
   * list and refetches from page one. An empty term fetches nothing: the page
   * shows its prompt instead, and `browseAnime` is never called without a term.
   *
   * Relevance first, always: a text search should return the closest matches,
   * not the most popular titles that happen to contain the words. The panel
   * therefore does not offer a sort control here.
   */
  const scroll = createInfiniteScroll(
    () => JSON.stringify({ term, filters }),
    (target) =>
      term === ""
        ? Promise.resolve(emptyPage())
        : browseAnime({ ...filters, search: term, sort: "searchMatch" }, target, PER_PAGE),
  );

  const { sentinel } = scroll;

  getGenres()
    .then((found) => {
      genres = found;
    })
    .catch(() => {
      // Same reasoning as the filter page: a missing list is a smaller panel,
      // not a broken page.
      genres = [];
    });

  getTags()
    .then((found) => {
      tags = found;
    })
    .catch(() => {
      tags = [];
    });

</script>

<!-- Echoes the term only when there are results to head. The empty state names
     the term itself, and two elements carrying it would read as a stutter. -->
<h1 class="mb-4 text-lg font-semibold tracking-tight">
  {#if scroll.items.length > 0}
    Results for “{term}”
  {:else}
    Search anime
  {/if}
</h1>

<!-- The term is not shown as a field: the navbar's search box already owns it.
     It rides along as a hidden parameter so changing a filter does not wipe it. -->
<FilterBar
  {params}
  {genres}
  {tags}
  current={filters}
  base="/search"
  showSearch={false}
  showSort={false}
  extraParams={{ q: term }}
/>

<div class="mt-4">
  <ActiveFilters {params} current={filters} base="/search" />
</div>

<div class="mt-8">
  {#if term === ""}
    <p class="py-16 text-center text-ink-muted">
      Type something in the search box above.
    </p>
  {:else if scroll.loading}
    <p class="py-16 text-center text-ink-muted">Searching…</p>
  {:else if scroll.items.length === 0 && scroll.error}
    <div class="py-16 text-center">
      <p class="text-ink">Could not run that search.</p>
      <p class="mt-2 text-sm text-ink-faint">{scroll.error}</p>
      <button
        type="button"
        onclick={scroll.retry}
        class="mt-4 rounded-full bg-surface-hover px-4 py-2 text-sm text-ink hover:text-ink-muted"
      >
        Try again
      </button>
    </div>
  {:else if scroll.items.length === 0}
    <p class="py-16 text-center text-ink-muted">No results for “{term}”.</p>
  {:else}
    <AnimeGrid anime={scroll.items} />

    {#if scroll.loadingMore}
      <p class="py-6 text-center text-sm text-ink-faint">Loading more…</p>
    {/if}

    {#if scroll.error}
      <div class="py-6 text-center">
        <p class="text-sm text-ink-faint">{scroll.error}</p>
        <button
          type="button"
          onclick={scroll.retry}
          class="mt-3 rounded-full bg-surface-hover px-4 py-2 text-sm text-ink hover:text-ink-muted"
        >
          Try again
        </button>
      </div>
    {:else if scroll.hasMore}
      <!-- Triggers the next page as it scrolls into view. A hairline so it does
           not shift the layout when items are appended above it. -->
      <div use:sentinel data-testid="scroll-sentinel" class="h-px"></div>
    {/if}
  {/if}
</div>