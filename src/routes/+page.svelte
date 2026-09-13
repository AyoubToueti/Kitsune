<script lang="ts">
  import { getGenres, getList, getSchedule, getTrending } from "$lib/api/anime";
  import GenreGrid from "$lib/components/GenreGrid.svelte";
  import HeroCarousel from "$lib/components/HeroCarousel.svelte";
  import ListSection from "$lib/components/ListSection.svelte";
  import ScheduleWidget from "$lib/components/ScheduleWidget.svelte";
  import TrendingRail from "$lib/components/TrendingRail.svelte";
  import type { Anime, ScheduledEpisode } from "$lib/types";

  /** Enough to fill the rail without over-fetching. */
  const TRENDING_LIMIT = 20;
  /** How many of those the hero rotates. */
  const HERO_LIMIT = 5;
  /** Per list block. */
  const BLOCK_LIMIT = 5;
  /** How far ahead the schedule looks. */
  const SCHEDULE_DAYS = 7;

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

  // --- schedule and genres ------------------------------------------------

  let genres = $state<string[]>([]);
  let schedule = $state<ScheduledEpisode[]>([]);

  // Computed once per load, so the window cannot drift while the user reads.
  const now = Math.floor(Date.now() / 1000);
  const scheduleTo = now + SCHEDULE_DAYS * 24 * 60 * 60;

  async function loadSecondary() {
    const [genreResult, scheduleResult] = await Promise.allSettled([
      getGenres(),
      getSchedule(now, scheduleTo, 50),
    ]);

    if (genreResult.status === "fulfilled") genres = genreResult.value;
    if (scheduleResult.status === "fulfilled") schedule = scheduleResult.value;
  }

  loadSecondary();

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
<ScheduleWidget entries={schedule} />