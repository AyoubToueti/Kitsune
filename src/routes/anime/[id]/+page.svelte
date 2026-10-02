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
  import TrailerCard from "$lib/components/TrailerCard.svelte";

  // Reactive ID handling for SvelteKit page switches
  const id = $derived(Number(page.params.id));

  let anime = $state<Anime | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
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

  $effect(() => {
    const malId = anime?.idMal;

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

  const episodes = $derived(
    anime ? episodesFor(anime, episodeInfo ?? []) : [],
  );

  let modalEpisode = $state<number | undefined>(undefined);

  function watchEpisode(index: number): void {
    if (!anime) return;
    modalEpisode = index;
  }

  const requestedEpisode = $derived.by(() => {
    const raw = page.url.searchParams.get("ep");
    if (raw === null) return undefined;
    const value = Number(raw);
    return Number.isInteger(value) && value >= 0 ? value : undefined;
  });

  $effect(() => {
    const wanted = requestedEpisode;
    if (wanted === undefined || wanted >= episodes.length) return;
    modalEpisode = wanted;
  });

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
  <div class="my-12 flex flex-col items-center justify-center rounded-2xl border border-border-subtle bg-surface-raised/40 py-16 px-4 text-center">
    <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-accent/10 text-accent">
      <svg class="h-6 w-6 stroke-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
    </div>
    <h3 class="text-base font-bold text-ink">Could not load anime details</h3>
    <p class="mt-1 max-w-sm text-xs text-ink-faint">{error}</p>
  </div>
{:else if anime === null}
  <div class="my-12 flex flex-col items-center justify-center rounded-2xl border border-border-subtle bg-surface-raised/40 py-16 text-center">
    <p class="text-base font-bold text-ink">Anime not found.</p>
    <p class="mt-1 text-xs text-ink-muted">The requested title does not exist or has been removed.</p>
  </div>
{:else}
  <!-- Hero banner -->
  <section class="group/hero relative overflow-hidden rounded-2xl border border-border-subtle/80 bg-surface shadow-lg">
    <!-- Backdrop Background Image Layer -->
    <div class="pointer-events-none absolute inset-0">
      {#if backdrop}
        <img
          src={backdrop}
          alt=""
          class="h-full w-full object-cover opacity-25 filter blur-xs scale-105 transition-transform duration-700 group-hover/hero:scale-100"
        />
      {/if}
      <!-- Gradient Overlays for smooth readability -->
      <div class="absolute inset-0 bg-gradient-to-t from-surface via-surface/80 to-surface/30"></div>
      <div class="absolute inset-0 bg-gradient-to-r from-surface via-surface/85 to-transparent"></div>
    </div>

    <!-- Content Area -->
    <div class="relative flex flex-col gap-6 p-6 sm:p-8 md:flex-row md:items-start">
      <!-- Poster Image -->
      {#if anime.coverImage}
        <div class="relative shrink-0 self-center md:self-start">
          <img
            src={anime.coverImage}
            alt={title ?? "Anime poster"}
            class="w-40 sm:w-48 rounded-xl object-cover shadow-2xl ring-1 ring-white/10 transition-transform duration-300 hover:scale-[1.02]"
          />
        </div>
      {/if}

      <!-- Main Info Block -->
      <div class="flex min-w-0 flex-1 flex-col">
        <!-- Title & Badges -->
        <div>
          <h1 class="text-2xl font-black tracking-tight text-ink sm:text-3xl lg:text-4xl">
            {title}
          </h1>

          <!-- Genres list -->
          {#if (anime.genres ?? []).length}
            <ul class="mt-3 flex flex-wrap gap-1.5">
              {#each (anime.genres ?? []).slice(0, 6) as genre (genre)}
                <li>
                  <a
                    href={genreHref(genre)}
                    class="inline-flex items-center rounded-lg border border-border-subtle/60 bg-surface-raised/60 px-2.5 py-1 text-xs font-medium text-ink-muted backdrop-blur-md transition-all hover:border-accent/40 hover:bg-accent/10 hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  >
                    {genre}
                  </a>
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <!-- Action Bar: Add/Edit List Status -->
        <div class="mt-4 flex flex-wrap items-center gap-3 border-y border-border-subtle/50 py-3">
          <ListStatusMenu mediaId={anime.id} />
        </div>

        <!-- Metadata Strip -->
        <div class="mt-3">
          <MetadataStrip {anime} />
        </div>

        <!-- Synopsis Container -->
        <div class="mt-4 text-xs sm:text-sm text-ink-muted">
          <Synopsis text={anime.description} />
        </div>
      </div>
    </div>
  </section>

  <!-- Main Content Grid -->
  <div class="mt-8 grid gap-8 lg:grid-cols-[1fr_20rem]">
    <!-- Left Column: Episodes List -->
    <div class="min-w-0">
      {#if episodeInfo === null}
        <!-- Loading Skeleton for Episodes -->
        <div class="space-y-3">
          <Skeleton class="h-6 w-32 rounded-md" />
          <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            {#each Array(8) as _, i (i)}
              <Skeleton class="aspect-video w-full rounded-xl" />
            {/each}
          </div>
        </div>
      {:else if episodes.length > 0}
        <EpisodeList
          {episodes}
          onSelect={watchEpisode}
          airedCount={episodes.length}
          totalCount={anime.episodeCount}
        />
      {:else}
        <!-- Empty Episode State -->
        <div class="flex flex-col items-center justify-center rounded-xl border border-border-subtle/60 bg-surface-raised/30 py-12 text-center">
          <svg class="h-8 w-8 text-ink-faint mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
          </svg>
          <p class="text-sm font-semibold text-ink">Episode info unavailable</p>
          <p class="text-xs text-ink-faint mt-0.5">No streaming or catalogue listings found for this title.</p>
        </div>
      {/if}
    </div>

    <!-- Right Sidebar: Sticky Container for Trailer and Relations -->
    <aside class="space-y-6 lg:sticky lg:top-6 lg:self-start">
      <TrailerCard trailer={anime.trailer} />
      <RelatedAnimeList relations={anime.relations ?? []} />
    </aside>
  </div>

  <!-- Recommendations Carousel Section -->
  <div class="mt-12 border-t border-border-subtle/60 pt-8">
    <Recommendations recommendations={anime.recommendations ?? []} workId={anime.id} />
  </div>

  <!-- In-place Episode Watch Modal -->
  <EpisodeWatchModal
    open={modalEpisode !== undefined}
    {anime}
    {episodes}
    episodeIndex={modalEpisode}
    onClose={() => (modalEpisode = undefined)}
  />
{/if}