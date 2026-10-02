<script lang="ts">
  import { page } from "$app/state";

  import { errorMessage, getAnime, getRecommendations } from "$lib/api/anime";
  import AnimeGridSkeleton from "$lib/components/AnimeGridSkeleton.svelte";
  import RecommendationGrid from "$lib/components/RecommendationGrid.svelte";
  import { createInfiniteScroll } from "$lib/infinite-scroll.svelte";
  import { displayTitle, type Anime } from "$lib/types";

  const PER_PAGE = 24;

  /** The work whose recommendations these are, from the route param. */
  const id = $derived(Number(page.params.id));

  /** The work's title, for the heading. Fetched once; a failure just drops it. */
  let anime = $state<Anime | null>(null);

  $effect(() => {
    const currentId = id;
    let cancelled = false;

    getAnime(currentId)
      .then((result) => {
        if (cancelled) return;
        anime = result;
      })
      .catch(() => {
        if (cancelled) return;
        // The heading is a nicety; the recommendations still render without it.
        anime = null;
      });

    return () => {
      cancelled = true;
    };
  });

  const title = $derived(anime ? displayTitle(anime.title) : null);

  /**
   * Recommendations, grown a page at a time as the reader scrolls.
   *
   * Keyed by the id so navigating to another work's page resets the list and
   * refetches from page one.
   */
  const scroll = createInfiniteScroll(
    () => `recommendations:${id}`,
    (target) => getRecommendations(id, target, PER_PAGE),
  );

  const { sentinel } = scroll;
</script>

<svelte:head>
  <title>{title ? `Recommendations for ${title}` : "Recommendations"}</title>
</svelte:head>

<a
  href={`/anime/${id}`}
  class="mb-2 inline-flex items-center gap-1.5 text-sm font-medium text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
>
  <svg class="h-4 w-4 stroke-current fill-none stroke-2" viewBox="0 0 24 24" aria-hidden="true">
    <path stroke-linecap="round" stroke-linejoin="round" d="M15 19l-7-7 7-7" />
  </svg>
  Back
</a>

<h1 class="mb-6 text-lg font-semibold tracking-tight">
  {#if title}
    Recommended for {title}
  {:else}
    Recommendations
  {/if}
</h1>

{#if scroll.loading}
  <AnimeGridSkeleton count={PER_PAGE} />
{:else if scroll.items.length === 0 && scroll.error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load recommendations.</p>
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
  <p class="py-16 text-center text-ink-muted">
    No recommendations for this title yet.
  </p>
{:else}
  <RecommendationGrid recommendations={scroll.items} votable mediaId={id} />

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
    <div use:sentinel data-testid="scroll-sentinel" class="h-px"></div>
  {/if}
{/if}