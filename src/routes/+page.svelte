<script lang="ts">
  import { getGenres, getList, getTrending } from "$lib/api/anime";
  import GenreGrid from "$lib/components/GenreGrid.svelte";
  import HeroCarousel from "$lib/components/HeroCarousel.svelte";
  import ListSection from "$lib/components/ListSection.svelte";
  import ScheduleWidget from "$lib/components/ScheduleWidget.svelte";
  import TrendingRail from "$lib/components/TrendingRail.svelte";
  import type { Anime } from "$lib/types";

  /** Enough to fill the rail without over-fetching. */
  const TRENDING_LIMIT = 20;
  /** How many of those the hero rotates. */
  const HERO_LIMIT = 5;
  /** Per list block. */
  const BLOCK_LIMIT = 5;


  // --- trending: shared by the hero and the rail --------------------------
  //
  // Both draw on the same list: the hero rotates the top few, the rail ranks
  // the rest. Two calls would hit AniList twice for identical data and repeat
  // the same titles side by side.

  let hero = $state<Anime[]>([]);
  let rail = $state<Anime[]>([]);

  // One call, split between the two consumers. Fetching the list twice would
  // hit AniList for identical data and show the same titles in both the
  // carousel and the rail. A failure degrades both to nothing rather than
  // taking the page down; the blocks below load independently.
  getTrending(TRENDING_LIMIT)
    .then((all) => {
      hero = all.slice(0, HERO_LIMIT);
      rail = all.slice(HERO_LIMIT);
    })
    .catch(() => {
      hero = [];
      rail = [];
    });

  // --- genres -------------------------------------------------------------
  // The schedule is not fetched here: the widget owns its own day-on-demand
  // loading, so the page does not need to know its window.

  let genres = $state<string[]>([]);

  getGenres()
    .then((found) => {
      genres = found;
    })
    .catch(() => {
      // A missing genre grid is a missing widget, not a broken page.
      genres = [];
    });

  // --- the four list blocks -----------------------------------------------
  // Each loads through its own `ListSection`, so one rate-limited block cannot
  // blank the grid.

  const topAiring = () => getList("topAiring", BLOCK_LIMIT);
  const mostPopular = () => getList("mostPopular", BLOCK_LIMIT);
  const topRated = () => getList("topRated", BLOCK_LIMIT);
  const latestCompleted = () => getList("latestCompleted", BLOCK_LIMIT);
</script>

{#if hero.length}
  <HeroCarousel anime={hero} />
{/if}

<TrendingRail anime={rail} />

<div class="mt-8 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
  <ListSection title="Top airing" filter="topAiring" load={topAiring} />
  <ListSection title="Most popular" filter="mostPopular" load={mostPopular} />
  <ListSection title="Top rated" filter="topRated" load={topRated} />
  <ListSection
    title="Latest completed"
    filter="latestCompleted"
    load={latestCompleted}
  />
</div>

<GenreGrid {genres} />
<ScheduleWidget />