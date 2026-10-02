<script lang="ts">
  import type { RecommendedAnime } from "$lib/types";
  import RecommendationCard from "./RecommendationCard.svelte";

  let {
    recommendations = [],
    workId,
  }: {
    recommendations?: RecommendedAnime[];
    /**
     * The work these recommendations belong to, so "View more" can link to its
     * full recommendations page. Omitted only by tests that render the row in
     * isolation; without it the link is not shown.
     */
    workId?: number;
  } = $props();

  /**
   * How many recommendations the row may ever show.
   *
   * Six is the ceiling. The row reveals fewer on narrower windows via the
   * per-slot classes below, so the count tracks the viewport instead of
   * overflowing into a horizontal scroller.
   */
  const MAX_VISIBLE = 6;

  /** The recommendations to render, never more than the ceiling. */
  const visible = $derived(recommendations.slice(0, MAX_VISIBLE));

  /**
   * Responsive reveal, indexed by position.
   *
   * Slots 0-1 always show (two cards fit any window); each later slot appears
   * at a wider breakpoint, so the row grows 2 -> 3 -> 4 -> 5 -> 6 as the window
   * widens and never needs a scroll. `hidden` wins below the breakpoint, so the
   * extra slots take no space rather than wrapping to a second line.
   */
  const slotClasses = [
    "",
    "",
    "hidden sm:block",
    "hidden md:block",
    "hidden lg:block",
    "hidden xl:block",
  ] as const;
</script>

{#if recommendations.length > 0}
  <section class="mt-8">
    <div class="mb-3 flex items-center justify-between gap-3">
      <h2 class="text-lg font-semibold tracking-tight">Recommended</h2>
      <!-- The detail lookup only carries the first handful of recommendations;
           the full set lives on its own page, paged as the reader scrolls. -->
      {#if workId !== undefined}
        <a
          href={`/anime/${workId}/recommendations`}
          data-testid="view-more-recommendations"
          class="shrink-0 text-sm font-medium text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          View more ›
        </a>
      {/if}
    </div>
    <!-- A grid rather than a fixed-width flex row: the cards are FLUID, so the
         visible ones always fill the width instead of the last one wrapping to
         a second line and leaving a gap. The column count matches the slots
         revealed above, so 2-6 cards share the row evenly at every width. -->
    <ul class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
      {#each visible as item, i (item.anime.id)}
        <li class={slotClasses[i] ?? "hidden"}>
          <!-- Voting is available here too, keyed by the base work. Without a
               work id (a test rendering the row in isolation) it stays off. -->
          <RecommendationCard
            recommendation={item}
            fluid
            votable={workId !== undefined}
            mediaId={workId}
          />
        </li>
      {/each}
    </ul>
  </section>
{/if}