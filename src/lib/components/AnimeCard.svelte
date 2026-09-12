<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";

  let { anime }: { anime: Anime } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");
</script>

<a
  href={`/anime/${anime.id}`}
  aria-label={title}
  class="group block w-40 shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
>
  <div class="relative aspect-[2/3] overflow-hidden rounded-lg bg-surface-hover">
    {#if anime.coverImage}
      <img
        src={anime.coverImage}
        alt=""
        loading="lazy"
        class="h-full w-full object-cover transition-transform duration-200 group-hover:scale-105"
      />
    {:else}
      <div class="flex h-full w-full items-center justify-center text-xs text-ink-faint">
        No cover
      </div>
    {/if}

    {#if anime.averageScore != null}
      <span
        class="absolute top-2 right-2 rounded-full bg-surface/85 px-2 py-0.5 text-xs font-semibold text-score"
      >
        {anime.averageScore}
      </span>
    {/if}
  </div>

  <p class="mt-2 line-clamp-2 text-sm font-medium text-ink group-hover:text-accent">
    {title}
  </p>
</a>