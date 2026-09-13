<script lang="ts">
  import { getGenres, getList, getSchedule, getTrending } from "$lib/api/anime";
  import GenreGrid from "$lib/components/GenreGrid.svelte";
  import HeroCarousel from "$lib/components/HeroCarousel.svelte";
  import ScheduleWidget from "$lib/components/ScheduleWidget.svelte";
  import Shelf from "$lib/components/Shelf.svelte";
  import type { Anime, ScheduledEpisode } from "$lib/types";

  /** Enough for a scrollable shelf without over-fetching. */
  const SHELF_LIMIT = 20;
  /** How many of those the hero rotates. */
  const HERO_LIMIT = 5;
  /** How far ahead the schedule widget looks. */
  const SCHEDULE_DAYS = 7;

  // --- trending: shared by the hero and the first row ----------------------
  //
  // Both show the same list: the hero rotates the top few titles, the row ranks
  // the rest. Fetching them separately would hit AniList twice for identical
  // data and repeat the same titles in the carousel and the row beneath it.

  /**
   * The in-flight (or resolved) trending request.
   *
   * A failure clears it, so a retry issues a fresh call rather than replaying
   * the same settled rejection.
   */
  let trendingRequest: Promise<Anime[]> | null = null;

  function loadTrending(): Promise<Anime[]> {
    trendingRequest ??= getTrending(SHELF_LIMIT).catch((err) => {
      trendingRequest = null;
      throw err;
    });
    return trendingRequest;
  }

  let hero = $state<Anime[]>([]);

  // A failed hero degrades to nothing rather than taking the page down. The row
  // below surfaces the same failure with a retry, since it reads the same
  // request.
  loadTrending()
    .then((all) => {
      hero = all.slice(0, HERO_LIMIT);
    })
    .catch(() => {
      hero = [];
    });

  /** The rest of the trending list, which is what the row ranks. */
  const trendingRow = () => loadTrending().then((all) => all.slice(HERO_LIMIT));

  // --- secondary sections -------------------------------------------------
  // These feed single sections rather than shelves, so a failure leaves the
  // section absent rather than breaking the page.

  let genres = $state<string[]>([]);
  let schedule = $state<ScheduledEpisode[]>([]);

  // Computed once per load, so the schedule window cannot drift while the user
  // reads.
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

  // --- remaining shelves --------------------------------------------------
  // Each loads through its own `Shelf`, so one rate-limited row cannot blank
  // the rest of the page.

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
  <Shelf title="Trending now" load={trendingRow} numbered />
  <Shelf title="Top airing" load={topAiring} />
  <Shelf title="Most popular" load={mostPopular} />
  <Shelf title="Top rated" load={topRated} />
  <Shelf title="Latest completed" load={latestCompleted} />
  <Shelf title="Upcoming" load={upcoming} />

  <GenreGrid {genres} />
  <ScheduleWidget entries={schedule} />
</div>