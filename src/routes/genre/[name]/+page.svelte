<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime } from "$lib/api/anime";
  import AnimeGrid from "$lib/components/AnimeGrid.svelte";
  import AnimeGridSkeleton from "$lib/components/AnimeGridSkeleton.svelte";
  import { createInfiniteScroll } from "$lib/infinite-scroll.svelte";
  import type { AnimePage } from "$lib/types";

  const PER_PAGE = 30;

  // SvelteKit decodes route params, so a "Slice%20of%20Life" path arrives here
  // already readable. Decoding again could corrupt a name containing a literal
  // percent, so it is used as-is.
  const genre = $derived(page.params.name ?? "");

  /** An empty result, used when there is no genre to browse. */
  function emptyPage(): AnimePage {
    return {
      items: [],
      pageInfo: { total: 0, currentPage: 1, lastPage: 1, hasNextPage: false },
    };
  }

  /**
   * Results, grown a page at a time as the reader scrolls.
   *
   * The key is the genre name: navigating to another genre resets the list and
   * refetches from page one. An empty name fetches nothing -- the fetcher
   * resolves an empty page itself -- so the backend is never asked to browse
   * for a blank genre.
   */
  const scroll = createInfiniteScroll(
    () => genre,
    (target) =>
      genre === ""
        ? Promise.resolve(emptyPage())
        : browseAnime({ genres: [genre], sort: "popularity" }, target, PER_PAGE),
  );

  const { sentinel } = scroll;
</script>

{#if genre === ""}
  <p class="py-16 text-center text-ink-muted">No genre selected.</p>
{:else if scroll.loading}
  <AnimeGridSkeleton count={PER_PAGE} />
{:else if scroll.items.length === 0 && scroll.error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load that genre.</p>
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
  <p class="py-16 text-center text-ink-muted">Nothing found in “{genre}”.</p>
{:else}
  <h1 class="mb-4 text-lg font-semibold tracking-tight">{genre}</h1>

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