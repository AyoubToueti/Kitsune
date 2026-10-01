<script lang="ts">
  import { page } from "$app/state";

  import { errorMessage, getAnime, getEpisodes } from "$lib/api/anime";
  import { displayTitle, type Anime, type EpisodeInfo } from "$lib/types";
  import MetadataStrip from "$lib/components/MetadataStrip.svelte";
  import DetailPageSkeleton from "$lib/components/DetailPageSkeleton.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import Synopsis from "$lib/components/Synopsis.svelte";
  import { episodesFor } from "$lib/episodes";
  import { genreHref } from "$lib/filter";
  import {
    consumeOpenRequest,
    nowPlaying,
    openRequested,
  } from "$lib/now-playing.svelte";
  import EpisodeList from "$lib/components/EpisodeList.svelte";
  import EpisodeWatchModal from "$lib/components/EpisodeWatchModal.svelte";
  import ListStatusMenu from "$lib/components/ListStatusMenu.svelte";
  import Recommendations from "$lib/components/Recommendations.svelte";
  import RelatedAnimeList from "$lib/components/RelatedAnimeList.svelte";
  import StreamingLinks from "$lib/components/StreamingLinks.svelte";
  import TrailerCard from "$lib/components/TrailerCard.svelte";

  // Reactive: navigating from /anime/1 to /anime/2 does NOT remount the
  // component in SvelteKit, so reading the param via $derived catches the
  // change and $effect re-fetches.
  const id = $derived(Number(page.params.id));

  let anime = $state<Anime | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  /**
   * The episode catalogue, or `null` while it is being fetched.
   *
   * The two empty-ish states need different UI: `null` means "not known yet" and
   * renders a skeleton, while `[]` means "known, and there is nothing" and falls
   * back to the synthesised list. A plain empty array would conflate them.
   */
  let episodeInfo = $state<EpisodeInfo[] | null>(null);

  $effect(() => {
    const currentId = id;

    let cancelled = false;
    loading = true;
    error = null;
    anime = null;

    getAnime(currentId)
      .then((result) => {
        if (cancelled) return;
        anime = result;
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

  /**
   * Fetch the episode catalogue once the work is known.
   *
   * Keyed on `idMal`, so it fires when the title changes rather than on every
   * render. This is the episode list's source, not a bonus: a failure still falls
   * back to the synthesised list rather than showing an error, because a work with
   * no reachable catalogue is exactly the case synthesis exists for.
   */
  $effect(() => {
    const malId = anime?.idMal;

    // No MAL link means there is nothing to ask for, so fall straight through to
    // synthesis instead of waiting on a request that will never be made.
    if (malId === undefined) {
      episodeInfo = [];
      return;
    }

    let cancelled = false;
    episodeInfo = null;

    getEpisodes(malId)
      .then((result) => {
        if (cancelled) return;
        episodeInfo = result;
      })
      .catch(() => {
        if (cancelled) return;
        episodeInfo = [];
      });

    return () => {
      cancelled = true;
    };
  });

  const title = $derived(
    anime ? (displayTitle(anime.title) ?? "Untitled") : null,
  );
  const backdrop = $derived(
    anime ? (anime.bannerImage ?? anime.coverImage) : null,
  );

  /**
   * The episodes to show: the Jikan catalogue when it has entries, otherwise a
   * synthesised 1..episodeCount stand-in using the cover as artwork, so a work
   * with no MAL link is still navigable.
   */
  const episodes = $derived(
    anime ? episodesFor(anime, episodeInfo ?? []) : [],
  );

  /**
   * The episode whose watch modal is open, by list index, or `undefined`.
   *
   * Opening the modal instead of navigating means the reader keeps the page
   * they were browsing while they pick a release and start playback.
   */
  let modalEpisode = $state<number | undefined>(undefined);

  /**
   * Open the watch modal for an episode.
   *
   * A named function rather than an inline arrow: the template narrows `anime`
   * to non-null, but that narrowing does not survive into a callback, so the
   * guard has to live inside the body.
   */
  function watchEpisode(index: number): void {
    if (!anime) return;
    modalEpisode = index;
  }

  /**
   * The episode index requested by `?ep=`, when present and sane.
   *
   * Resume links (the Continue Watching row and the floating disc) point here
   * with the SAME 0-based index the watch page uses, so a click lands on the
   * detail page with the modal already open on the right episode. A nonsense
   * value is treated as "no request" rather than clamped to an arbitrary one.
   */
  const requestedEpisode = $derived.by(() => {
    const raw = page.url.searchParams.get("ep");
    if (raw === null) return undefined;
    const value = Number(raw);
    return Number.isInteger(value) && value >= 0 ? value : undefined;
  });

  // Open the modal for a requested episode once the list is known, so an
  // out-of-range value is ignored rather than shown as a phantom selection.
  $effect(() => {
    const wanted = requestedEpisode;
    if (wanted === undefined || wanted >= episodes.length) return;
    modalEpisode = wanted;
  });

  // Open the modal when the now-playing disc asks. Separate from the `?ep=`
  // effect above because a request must work even when the URL does not change
  // -- a click from this very page, where `?ep=` is already set. The request is
  // only consumed once this page's work matches and the list is known, so one
  // that arrives before load is not lost.
  $effect(() => {
    openRequested();
    const playing = nowPlaying();
    if (!anime || !playing || playing.anime.id !== anime.id) return;
    if (!consumeOpenRequest()) return;
    modalEpisode = playing.episodeIndex;
  });
</script>

{#if loading}
  <DetailPageSkeleton />
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load details.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if anime === null}
  <div class="py-16 text-center">
    <p class="text-ink">Anime not found.</p>
  </div>
{:else}
  <!-- Hero section: banner backdrop + cover + title + metadata + synopsis -->
  <!-- `overflow-hidden` lives on the backdrop layer, NOT the section. On the
       section it would also clip the list-status dropdown, which is an
       absolutely-positioned child; on an inner layer it still clips the image
       to the rounded corners and leaves the menu free. -->
  <section class="relative rounded-xl border border-border-subtle">
    <div class="pointer-events-none absolute inset-0 overflow-hidden rounded-xl">
      {#if backdrop}
        <img
          src={backdrop}
          alt=""
          class="absolute inset-0 h-full w-full object-cover opacity-40"
        />
      {/if}
      <div
        class="absolute inset-0 bg-linear-to-r from-surface via-surface/85 to-surface/40"
      ></div>
    </div>

    <div class="relative flex gap-5 p-6">
      {#if anime.coverImage}
        <img
          src={anime.coverImage}
          alt=""
          class="hidden w-32 shrink-0 rounded-lg object-cover sm:block"
        />
      {/if}

      <div class="min-w-0">
        <h1 class="text-2xl font-semibold tracking-tight sm:text-3xl">{title}</h1>

        {#if (anime.genres ?? []).length}
          <ul class="mt-2 flex flex-wrap gap-2">
            {#each (anime.genres ?? []).slice(0, 6) as genre (genre)}
              <!-- A link, not a label: a genre is a filter, and the pills are
                   the shortest path to the rest of it. -->
              <li>
                <a
                  href={genreHref(genre)}
                  class="inline-block rounded-full bg-surface-hover px-2.5 py-0.5 text-xs text-ink-muted transition-colors hover:bg-surface-raised hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {genre}
                </a>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="mt-3">
          <ListStatusMenu mediaId={anime.id} />
        </div>

        <div class="mt-3">
          <MetadataStrip {anime} />
        </div>

        <div class="mt-3">
          <Synopsis text={anime.description} />
        </div>
      </div>
    </div>
  </section>

  <!-- Episodes and relations share one row: the episode grid is the main
       column, the relations a narrower sidebar beside it. They stack on narrow
       screens, where a sidebar beside a grid would be too cramped. -->
  <div class="mt-8 grid gap-8 lg:grid-cols-[1fr_20rem]">
    <div>
      {#if episodeInfo === null}
        <!-- Waiting on the catalogue. A skeleton rather than the synthesised
             list, so the grid does not visibly reshuffle when the real titles
             land underneath it. -->
        <Skeleton class="mb-3 h-6 w-24" />
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
          {#each Array(8) as _, i (i)}
            <Skeleton class="aspect-video w-full rounded-lg" />
          {/each}
        </div>
      {:else if episodes.length > 0}
        <EpisodeList
          {episodes}
          onSelect={watchEpisode}
          airedCount={episodes.length}
          totalCount={anime.episodeCount}
        />
      {:else}
        <!-- Neither a catalogue nor a count, so say so rather than render an
             empty grid that reads as broken. -->
        <p class="text-sm text-ink-faint">Episode information unavailable.</p>
      {/if}
    </div>

      <aside>
      <TrailerCard trailer={anime.trailer} />
      <RelatedAnimeList relations={anime.relations ?? []} />
    </aside>
  </div>


  <!-- The in-place watch modal. Clicking an episode opens it rather than
       navigating away, so the reader keeps their place on the page while they
       pick a release and start playback. -->
  <EpisodeWatchModal
    open={modalEpisode !== undefined}
    {anime}
    {episodes}
    episodeIndex={modalEpisode}
    onClose={() => (modalEpisode = undefined)}
  />
  <!-- Where to watch -->
  <!-- <div class="mt-8">
    <StreamingLinks episodes={anime.streamingEpisodes} />
  </div> -->

  <!-- Recommendations, full width below the grid: a poster row needs the room,
       and it reads as "more like this" rather than part of the sidebar. -->
  <Recommendations recommendations={anime.recommendations ?? []} />
{/if}