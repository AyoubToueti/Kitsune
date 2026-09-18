<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { stripHtml } from "$lib/text";

  let { anime }: { anime: Anime } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");
  // Banner is the intended backdrop; a portrait cover is a poor substitute
  // but better than nothing.
  const backdrop = $derived(anime.bannerImage ?? anime.coverImage);
  const synopsis = $derived(stripHtml(anime.description));
</script>

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

      <div class="mt-2 flex flex-wrap items-center gap-3 text-sm">
        {#if anime.averageScore != null}
          <span class="font-semibold text-score">{anime.averageScore}</span>
        {/if}
        {#if anime.format}
          <span class="text-ink-muted">{anime.format}</span>
        {/if}
        {#if anime.seasonYear}
          <span class="text-ink-muted">{anime.seasonYear}</span>
        {/if}
      </div>

      {#if (anime.genres ?? []).length}
        <ul class="mt-3 flex flex-wrap gap-2">
          {#each (anime.genres ?? []).slice(0, 4) as genre (genre)}
            <li class="rounded-full bg-surface-hover px-2.5 py-0.5 text-xs text-ink-muted">
              {genre}
            </li>
          {/each}
        </ul>
      {/if}

      {#if synopsis}
        <p class="mt-3 line-clamp-3 max-w-2xl text-sm text-ink-muted">{synopsis}</p>
      {/if}

      <a
        href={`/anime/${anime.id}`}
        class="mt-4 inline-block rounded-full bg-accent px-5 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
      >
        View details
      </a>
    </div>
  </div>
</section>