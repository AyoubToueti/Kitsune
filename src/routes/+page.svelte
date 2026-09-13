<script lang="ts">
  import { getGenres, getList, getSchedule, getTrending } from "$lib/api/anime";
  import GenreGrid from "$lib/components/GenreGrid.svelte";
  import HeroCarousel from "$lib/components/HeroCarousel.svelte";
  import ScheduleWidget from "$lib/components/ScheduleWidget.svelte";
  import Shelf from "$lib/components/Shelf.svelte";
  import type { Anime, ScheduledEpisode } from "$lib/types";

  /** Enough for a scrollable shelf without over-fetching. */
  const SHELF_LIMIT = 20;
  /** The carousel wants a handful of strong titles, not a full row. */
  const HERO_LIMIT = 5;
  /** How far ahead the schedule widget looks. */
  const SCHEDULE_DAYS = 7;

  // --- hero ---------------------------------------------------------------
  // Loaded here rather than through `Shelf`, because the hero renders a
  // carousel instead of a poster row.

  let hero = $state<Anime[]>([]);

  async function loadHero() {
    try {
      // `onMount` in the component tree below means this runs in the browser;
      // awaiting here keeps the failure contained to the hero.
      hero = await getList("trending", HERO_LIMIT);
    } catch {
      // A failed hero should degrade to nothing rather than take the page
      // down; the shelves below still load independently.
      hero = [];
    }
  }

  loadHero();

  // --- secondary sections -------------------------------------------------
  // These feed single sections rather than shelves, so a failure leaves the
  // section absent rather than breaking the page.

  let genres = $state<string[]>([]);
  let schedule = $state<ScheduledEpisode[]>([]);

  // Computed once per page load, so the schedule window does not drift while
  // the user reads.
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

  // --- shelves ------------------------------------------------------------
  // Each loads through its own `Shelf`, so one rate-limited row cannot blank
  // the rest of the page.

  const trending = () => getTrending(SHELF_LIMIT);
  const topAiring = () => getList("topAiring", SHELF_LIMIT);
  const mostPopular = () => getList("mostPopular", SHELF_LIMIT);
  const topRated = () => getList("topRated", SHELF_LIMIT);
  const latestCompleted = () => getList("latestCompleted", SHELF_LIMIT);
  const upcoming = () => getList("upcoming", SHELF_LIMIT);
</script>

{#if hero.length}
  <HeroCarousel anime={hero} />
{/if}

<div class="space-y-2">
  <Shelf title="Trending now" load={trending} numbered />
  <Shelf title="Top airing" load={topAiring} />
  <Shelf title="Most popular" load={mostPopular} />
  <Shelf title="Top rated" load={topRated} />
  <Shelf title="Latest completed" load={latestCompleted} />
  <Shelf title="Upcoming" load={upcoming} />

  <GenreGrid {genres} />
  <ScheduleWidget entries={schedule} />
</div>