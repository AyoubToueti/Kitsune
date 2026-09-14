<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime, getGenres, getTags } from "$lib/api/anime";
  import ActiveFilters from "$lib/components/ActiveFilters.svelte";
  import AnimeGrid from "$lib/components/AnimeGrid.svelte";
  import AnimeGridSkeleton from "$lib/components/AnimeGridSkeleton.svelte";
  import FilterBar from "$lib/components/FilterBar.svelte";
  import { parseBrowseQuery } from "$lib/filter";
  import { createInfiniteScroll } from "$lib/infinite-scroll.svelte";
  import type { MediaTag } from "$lib/types";

  const PER_PAGE = 30;

  /**
   * The filters, read from the URL.
   *
   * The URL is the single source of truth: a filtered view is shareable, the
   * back button works, and the page needs no state store.
   */
  const params = $derived(page.url.searchParams);
  const query = $derived(parseBrowseQuery(params));

  let genres = $state<string[]>([]);
  let tags = $state<MediaTag[]>([]);

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

  /**
   * Results, grown a page at a time as the reader scrolls.
   *
   * The key is the filter query serialised. Changing any filter produces a new
   * key, which resets the list and refetches from page one; re-parsing the same
   * URL does not, because the serialisation is identical. The page number is
   * deliberately not part of the key: the list grows by scrolling, so a refresh
   * starts from the top rather than restoring a position nobody else has loaded.
   */
  const scroll = createInfiniteScroll(
    () => JSON.stringify(query),
    (target) => browseAnime(query, target, PER_PAGE),
  );

  const { sentinel } = scroll;
</script>

<h1 class="mb-4 text-lg font-semibold tracking-tight">Filter anime</h1>

<FilterBar {params} {genres} {tags} current={query} />

<div class="mt-4">
  <ActiveFilters {params} current={query} />
</div>

<div class="mt-8">
  {#if scroll.loading}
    <AnimeGridSkeleton count={PER_PAGE} />
  {:else if scroll.items.length === 0 && scroll.error}
    <div class="py-16 text-center">
      <p class="text-ink">Could not load results.</p>
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
    <p class="py-16 text-center text-ink-muted">No titles match those filters.</p>
  {:else}
    <AnimeGrid anime={scroll.items} />

    {#if scroll.loadingMore}
      <div class="mt-4">
        <AnimeGridSkeleton count={6} />
      </div>
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