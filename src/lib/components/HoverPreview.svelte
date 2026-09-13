<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { stripHtml } from "$lib/text";

  let {
    anime,
    x,
    y,
    onenter,
    onleave,
  }: {
    anime: Anime;
    x: number;
    y: number;
    /** Called when the pointer enters, so the card can cancel its close. */
    onenter: () => void;
    /** Called when the pointer leaves, so the card can schedule a close. */
    onleave: () => void;
  } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");
  const synopsis = $derived(stripHtml(anime.description));

  /** Short facts for the badge row, mirroring the reference layout. */
  const facts = $derived(
    [
      anime.episodeCount != null ? `${anime.episodeCount} eps` : null,
      anime.durationMinutes != null ? `${anime.durationMinutes}m` : null,
    ].filter((f): f is string => f !== null),
  );

  function genreHref(genre: string): string {
    return `/genre/${encodeURIComponent(genre)}`;
  }
</script>

<!-- Fixed rather than absolute: the rows scroll horizontally, so an in-flow
     popup would be clipped by their `overflow-x-auto`. -->
<div
  role="tooltip"
  data-testid="hover-preview"
  style="left: {x}px; top: {y}px;"
  class="fixed z-50 w-72 rounded-lg border border-border-subtle bg-surface-raised p-3 shadow-xl"
  onmouseenter={onenter}
  onmouseleave={onleave}
>
  <p class="text-sm font-semibold text-ink">{title}</p>

  <div class="mt-1.5 flex flex-wrap items-center gap-2 text-xs">
    {#if anime.averageScore != null}
      <span class="flex items-center gap-1 font-semibold text-score">
        <span aria-hidden="true">★</span>{anime.averageScore}
      </span>
    {/if}

    {#if anime.format}
      <span class="rounded bg-surface-hover px-1.5 py-0.5 text-ink-muted">
        {anime.format}
      </span>
    {/if}

    {#each facts as fact (fact)}
      <span class="rounded bg-surface-hover px-1.5 py-0.5 text-ink-muted">
        {fact}
      </span>
    {/each}
  </div>

  {#if synopsis}
    <p class="mt-2 line-clamp-3 text-xs text-ink-muted">{synopsis}</p>
  {/if}

  <dl class="mt-2 space-y-0.5 text-xs">
    {#if anime.title.native}
      <div class="flex gap-2">
        <dt class="shrink-0 text-ink-faint">Japanese:</dt>
        <dd class="min-w-0 truncate text-ink-muted">{anime.title.native}</dd>
      </div>
    {/if}

    {#if anime.seasonYear != null}
      <div class="flex gap-2">
        <dt class="shrink-0 text-ink-faint">Aired:</dt>
        <dd class="text-ink-muted">{anime.seasonYear}</dd>
      </div>
    {/if}

    {#if anime.status}
      <div class="flex gap-2">
        <dt class="shrink-0 text-ink-faint">Status:</dt>
        <dd class="text-ink-muted">{anime.status}</dd>
      </div>
    {/if}
  </dl>

  {#if anime.genres.length}
    <p class="mt-2 text-xs">
      <span class="text-ink-faint">Genres:</span>
      {#each anime.genres.slice(0, 5) as genre, i (genre)}<a
          href={genreHref(genre)}
          class="text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >{genre}</a
        >{#if i < Math.min(anime.genres.length, 5) - 1}<span class="text-ink-faint"
          >, </span
        >{/if}{/each}
    </p>
  {/if}

  <a
    href={`/anime/${anime.id}`}
    class="mt-3 block rounded-full bg-accent px-4 py-1.5 text-center text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  >
    View details
  </a>
</div>