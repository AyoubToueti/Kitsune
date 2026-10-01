<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { createHoverPreview } from "$lib/hover-preview.svelte";
  import HoverPreview from "./HoverPreview.svelte";

  /**
   * `fluid` lets the card fill its parent instead of the fixed rail width, so a
   * grid cell sizes it. Every rail keeps the default.
   */
  let { anime, fluid = false }: { anime: Anime; fluid?: boolean } = $props();

  const title = $derived.by(() => {
    const full = displayTitle(anime.title) ?? "Untitled";
    return full.length > 23 ? `${full.slice(0, 20)}...` : full;
  });

  /** The preview anchors to the info trigger button inside the quick bar. */
  let trigger = $state<HTMLElement | null>(null);
  const hover = createHoverPreview(() => trigger);
</script>

<div
  data-testid="anime-card"
  class="group relative flex flex-col overflow-hidden rounded-xl border border-border-subtle bg-surface-raised shadow-md transition-all duration-300 hover:-translate-y-1 hover:border-accent/60 hover:shadow-xl {fluid
    ? 'w-full min-w-0'
    : 'w-40 shrink-0'}"
>
  <!-- Artwork Container -->
  <div
    data-testid="poster"
    class="relative aspect-[2/3] w-full overflow-hidden bg-surface-hover"
  >
    <a
      href={`/anime/${anime.id}`}
      aria-label={title}
      draggable="false"
      class="block h-full w-full focus:outline-none"
    >
      {#if anime.coverImage}
        <img
          src={anime.coverImage}
          alt=""
          loading="lazy"
          draggable="false"
          class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105 group-hover:brightness-90"
        />
      {:else}
        <div
          class="flex h-full w-full items-center justify-center text-xs text-ink-faint"
        >
          No cover
        </div>
      {/if}
    </a>

    <!-- Average Score Badge -->
    {#if anime.averageScore != null}
      <span
        class="absolute top-2 right-2 rounded-md border border-border-subtle/50 bg-surface/80 px-1.5 py-0.5 text-[11px] font-bold text-score backdrop-blur-md shadow-sm"
      >
        ★ {anime.averageScore}
      </span>
    {/if}

    <!-- Hover Quick Bar (Slides up on card hover or focus-within) -->
    <div
      class="absolute inset-x-0 bottom-0 z-10 flex translate-y-full items-center gap-1.5 bg-gradient-to-t from-black/90 via-black/70 to-transparent p-2.5 pt-6 transition-transform duration-200 group-hover:translate-y-0 focus-within:translate-y-0"
    >
      <!-- Watch / Details Link -->
      <a
        href={`/anime/${anime.id}`}
        class="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-accent py-1.5 text-xs font-semibold text-white shadow-md transition-all hover:bg-accent-hover hover:shadow-accent/30 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <svg class="h-3.5 w-3.5 fill-current" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M8 5v14l11-7z" />
        </svg>
        <span>Watch</span>
      </a>

      <!-- Info Trigger Button -->
      <button
        bind:this={trigger}
        type="button"
        data-testid="info-trigger"
        aria-label={`Preview ${title}`}
        class="flex h-7 w-7 items-center justify-center rounded-lg border border-white/10 bg-white/15 text-xs font-bold text-white backdrop-blur-sm transition-all hover:bg-white/30 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        onmouseenter={hover.show}
        onmouseleave={hover.scheduleClose}
        onfocusin={hover.show}
        onfocusout={hover.scheduleClose}
      >
        i
      </button>
    </div>
  </div>

  <!-- Title section. Not a second link: the poster already links here, and
       two anchors with the same accessible name read as duplicates. -->
  <div class="flex flex-1 flex-col justify-between p-2.5">
    <p
      class="line-clamp-2 text-xs font-semibold leading-snug text-ink transition-colors group-hover:text-accent"
      title={title}
    >
      {title}
    </p>
  </div>
</div>

{#if hover.open}
  <!-- Sibling of the card, triggered by the quick bar info button -->
  <HoverPreview
    {anime}
    placement={hover.placement}
    measure={hover.measure}
    onenter={hover.enterPreview}
    onleave={hover.leavePreview}
  />
{/if}