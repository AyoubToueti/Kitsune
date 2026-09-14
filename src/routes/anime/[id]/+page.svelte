<script lang="ts">
  import { page } from "$app/state";

  import { errorMessage, getAnime } from "$lib/api/anime";
  import { displayTitle, type Anime } from "$lib/types";
  import MetadataStrip from "$lib/components/MetadataStrip.svelte";
  import Synopsis from "$lib/components/Synopsis.svelte";
  import EpisodeGrid from "$lib/components/EpisodeGrid.svelte";
  import EpisodeList from "$lib/components/EpisodeList.svelte";
  import RelatedAnimeList from "$lib/components/RelatedAnimeList.svelte";
  import StreamingLinks from "$lib/components/StreamingLinks.svelte";

  // Reactive: navigating from /anime/1 to /anime/2 does NOT remount the
  // component in SvelteKit, so reading the param via $derived catches the
  // change and $effect re-fetches.
  const id = $derived(Number(page.params.id));

  let anime = $state<Anime | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

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

  const title = $derived(
    anime ? (displayTitle(anime.title) ?? "Untitled") : null,
  );
  const backdrop = $derived(
    anime ? (anime.bannerImage ?? anime.coverImage) : null,
  );
</script>

{#if loading}
  <p class="py-16 text-center text-ink-muted">Loading…</p>
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
  <section class="relative overflow-hidden rounded-xl border border-border-subtle">
    {#if backdrop}
      <img
        src={backdrop}
        alt=""
        class="absolute inset-0 h-full w-full object-cover opacity-40"
      />
    {/if}
    <div
      class="absolute inset-0 bg-gradient-to-r from-surface via-surface/85 to-surface/40"
    ></div>

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

        {#if anime.genres.length}
          <ul class="mt-2 flex flex-wrap gap-2">
            {#each anime.genres.slice(0, 6) as genre (genre)}
              <li class="rounded-full bg-surface-hover px-2.5 py-0.5 text-xs text-ink-muted">
                {genre}
              </li>
            {/each}
          </ul>
        {/if}

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
      {#if anime.streamingEpisodes.length > 0}
        <!-- Prefer the thumbnail grid: it is the richer view and the data is
             already there. -->
        <EpisodeList episodes={anime.streamingEpisodes} />
      {:else}
        <!-- No streaming data, so fall back to the inert numbered grid the page
             used before, which at least shows the episode count. -->
        <EpisodeGrid count={anime.episodeCount} />
      {/if}
    </div>

    <aside>
      <RelatedAnimeList relations={anime.relations} />
    </aside>
  </div>

  <!-- Where to watch -->
  <div class="mt-8">
    <StreamingLinks episodes={anime.streamingEpisodes} />
  </div>
{/if}