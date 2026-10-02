<script lang="ts">
  import { genreHref } from "$lib/filter";
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
    measure: (node: HTMLElement) => { destroy: () => void };
    onenter: () => void;
    onleave: () => void;
  } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");
  const synopsis = $derived(stripHtml(anime.description));

  /** Short facts for the badge row. */
  const facts = $derived(
    [
      anime.episodeCount != null ? `${anime.episodeCount} ${anime.episodeCount === 1 ? 'ep' : 'eps'}` : null,
      anime.durationMinutes != null ? `${anime.durationMinutes}m` : null,
    ].filter((f): f is string => f !== null),
  );

  /** Display up to 4 genres cleanly as interactive pills */
  const shownGenres = $derived(anime.genres.slice(0, 4));
</script>

<div
  use:measure
  role="tooltip"
  data-testid="hover-preview"
  data-side={placement.side}
  style="left: {placement.x}px; top: {placement.y}px;"
  class="fixed z-50 w-80 rounded-2xl border border-border-subtle/80 bg-surface-raised/95 p-4 shadow-2xl backdrop-blur-md transition-all motion-reduce:animate-none {placement.side === 'bottom' ? 'animate-preview-in-bottom' : 'animate-preview-in-top'}"
  onmouseenter={onenter}
  onmouseleave={onleave}
>
  <!-- Caret Pointer -->
  <span
    aria-hidden="true"
    data-testid="hover-caret"
    class="absolute h-3 w-3 rotate-45 border-border-subtle/80 bg-surface-raised/95 {placement.side === 'bottom'
      ? '-bottom-1.5 border-b border-r shadow-sm'
      : '-top-1.5 border-t border-l shadow-sm'}"
    style="left: {placement.caretX - CARET_SIZE / 2}px"
  ></span>

  <!-- Title & Score Header -->
  <div class="flex items-start justify-between gap-2">
    <p class="text-sm font-bold leading-tight text-ink line-clamp-2">
      {title}
    </p>

    {#if anime.averageScore != null}
      <span class="flex shrink-0 items-center gap-0.5 rounded-md border border-border-subtle/40 bg-surface/85 px-1.5 py-0.5 text-[10px] font-extrabold text-score shadow-sm">
        <span class="text-amber-400">★</span> {anime.averageScore}
      </span>
    {/if}
  </div>

  <!-- Format & Fact Badges -->
  <div class="mt-2 flex flex-wrap items-center gap-1.5 text-[11px] font-medium text-ink-muted">
    {#if anime.format}
      <span class="rounded-md border border-border-subtle/50 bg-surface-hover/80 px-2 py-0.5 font-bold uppercase text-ink">
        {anime.format}
      </span>
    {/if}

    {#each facts as fact (fact)}
      <span class="rounded-md border border-border-subtle/40 bg-surface-hover/50 px-2 py-0.5">
        {fact}
      </span>
    {/each}

    {#if anime.status}
      <span class="rounded-md border border-border-subtle/40 bg-surface-hover/50 px-2 py-0.5 capitalize">
        {anime.status.toLowerCase()}
      </span>
    {/if}
  </div>

  <!-- Synopsis -->
  {#if synopsis}
    <p class="mt-2.5 line-clamp-3 text-xs leading-relaxed text-ink-muted">
      {synopsis}
    </p>
  {/if}

  <!-- Metadata Grid -->
  {#if anime.title.native || anime.seasonYear != null}
    <div class="mt-3 border-t border-border-subtle/40 pt-2.5 text-[11px]">
      {#if anime.title.native}
        <div class="flex items-center justify-between gap-2">
          <span class="shrink-0 font-medium text-ink-faint">Japanese</span>
          <span class="truncate font-semibold text-ink-muted">{anime.title.native}</span>
        </div>
      {/if}

      {#if anime.seasonYear != null}
        <div class="mt-1 flex items-center justify-between gap-2">
          <span class="shrink-0 font-medium text-ink-faint">Aired</span>
          <span class="font-semibold text-ink-muted">{anime.seasonYear}</span>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Genre Pills -->
  {#if shownGenres.length}
    <div class="mt-3 flex flex-wrap gap-1.5">
      {#each shownGenres as genre (genre)}
        <a
          href={genreHref(genre)}
          class="rounded-lg border border-border-subtle/40 bg-surface-hover/40 px-2 py-0.5 text-[10px] font-semibold text-ink-muted transition-all hover:border-accent/40 hover:bg-accent/10 hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          {genre}
        </a>
      {/each}
    </div>
  {/if}

  <!-- CTA Action Button -->
  <a
    href={`/anime/${anime.id}`}
    class="group/btn mt-3.5 flex items-center justify-center gap-1.5 w-full rounded-xl bg-accent py-2 text-xs font-bold text-white shadow-md transition-all hover:bg-accent-hover hover:shadow-accent/30 active:scale-98 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  >
    <span>View Details</span>
    <svg class="h-3.5 w-3.5 stroke-current fill-none stroke-2 transition-transform group-hover/btn:translate-x-0.5" viewBox="0 0 24 24" aria-hidden="true">
      <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
    </svg>
  </a>
</div>