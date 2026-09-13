<script lang="ts">
  import { CARET_SIZE, type Placement } from "$lib/hover";
  import { displayTitle, type Anime } from "$lib/types";
  import { stripHtml } from "$lib/text";

  let {
    anime,
    placement,
    measure,
    onenter,
    onleave,
  }: {
    anime: Anime;
    placement: Placement;
    /**
     * Action that lets the controller measure this panel.
     *
     * The height drives the vertical offset, and an estimate left a visible
     * gap between the caret and the card.
     */
    measure: (node: HTMLElement) => { destroy: () => void };
    /** Called when the pointer enters, so the card can cancel its close. */
    onenter: () => void;
    /** Called when the pointer leaves, so the card can schedule a close. */
    onleave: () => void;
  } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");
  const synopsis = $derived(stripHtml(anime.description));

  /** Short facts for the badge row. */
  const facts = $derived(
    [
      anime.episodeCount != null ? `${anime.episodeCount} eps` : null,
      anime.durationMinutes != null ? `${anime.durationMinutes}m` : null,
    ].filter((f): f is string => f !== null),
  );

  /** Only the first few genres fit; more would wrap awkwardly. */
  const shownGenres = $derived(anime.genres.slice(0, 5));

  function genreHref(genre: string): string {
    return `/genre/${encodeURIComponent(genre)}`;
  }
</script>

<!-- Fixed rather than absolute: the rails scroll horizontally, so an in-flow
     popup would be clipped by their `overflow-x-auto`. -->
<div
  use:measure
  role="tooltip"
  data-testid="hover-preview"
  data-side={placement.side}
  style="left: {placement.x}px; top: {placement.y}px;"
  class="fixed z-50 w-72 rounded-lg border border-border-subtle bg-surface-raised p-3 shadow-xl"
  onmouseenter={onenter}
  onmouseleave={onleave}
>
  <!-- A rotated square reads as a caret. Which edge it sits on follows the
       placement, so it always points back at the card. It sits flush against
       the panel edge, overlapping the card, so there is no dead zone to cross
       on the way to the preview. -->
  <span
    aria-hidden="true"
    data-testid="hover-caret"
    class="absolute h-3 w-3 rotate-45 border-border-subtle bg-surface-raised {placement.side ===
    'bottom'
      ? '-bottom-1.5 border-b border-r'
      : '-top-1.5 border-t border-l'}"
    style="left: {placement.caretX - CARET_SIZE / 2}px"
  ></span>

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

  {#if shownGenres.length}
    <p class="mt-2 text-xs">
      <span class="text-ink-faint">Genres:</span>
      {#each shownGenres as genre, i (genre)}<a
          href={genreHref(genre)}
          class="text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >{genre}</a
        >{#if i < shownGenres.length - 1}<span class="text-ink-faint">, </span>{/if}{/each}
    </p>
  {/if}

  <a
    href={`/anime/${anime.id}`}
    class="mt-3 block rounded-full bg-accent px-4 py-1.5 text-center text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  >
    View details
  </a>
</div>