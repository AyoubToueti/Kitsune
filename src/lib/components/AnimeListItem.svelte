<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { createHoverPreview } from "$lib/hover-preview.svelte";
  import HoverPreview from "./HoverPreview.svelte";

  let { anime }: { anime: Anime } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");

  // Anchored to the thumbnail rather than the whole row: the preview overlaps
  // its anchor, and covering the title would hide what the pointer is on.
  let thumb = $state<HTMLElement | null>(null);
  const hover = createHoverPreview(() => thumb);
</script>

<!-- The compact row used by the home page's list blocks: a small portrait
     thumbnail beside the title and a row of facts. -->
<a
  href={`/anime/${anime.id}`}
  aria-label={title}
  class="group flex gap-3 rounded-lg p-2 transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  onmouseenter={hover.show}
  onmouseleave={hover.scheduleClose}
  onfocusin={hover.show}
  onfocusout={hover.scheduleClose}
>
  <div
    bind:this={thumb}
    class="relative aspect-[2/3] w-16 shrink-0 overflow-hidden rounded bg-surface-hover"
  >
    {#if anime.coverImage}
      <img
        src={anime.coverImage}
        alt=""
        loading="lazy"
        class="h-full w-full object-cover"
      />
    {:else}
      <div class="flex h-full w-full items-center justify-center text-[10px] text-ink-faint">
        No cover
      </div>
    {/if}
  </div>

  <div class="min-w-0 flex-1">
    <h3
      class="line-clamp-2 text-sm font-medium text-ink transition-colors group-hover:text-accent"
    >
      {title}
    </h3>

    <!-- Chips mirror the reference layout. The provider has no sub/dub
         counts, so these carry the facts it does report rather than
         fabricating figures. -->
    <div class="mt-1.5 flex flex-wrap items-center gap-x-1.5 gap-y-1 text-xs">
      {#if anime.averageScore != null}
        <span
          data-testid="score-chip"
          class="rounded bg-surface-hover px-1.5 py-0.5 font-semibold text-score"
        >
          {anime.averageScore}
        </span>
      {/if}

      {#if anime.episodeCount != null}
        <span
          data-testid="episodes-chip"
          class="rounded bg-surface-hover px-1.5 py-0.5 text-ink-muted"
        >
          {anime.episodeCount}
        </span>
      {/if}

      {#if anime.format}
        <span class="text-ink-faint" aria-hidden="true">•</span>
        <span class="text-ink-faint">{anime.format}</span>
      {/if}
    </div>
  </div>
</a>

{#if hover.open}
  <!-- Sibling of the anchor, not a child: nesting links inside a link is
       invalid, and the preview has its own genre links. -->
  <HoverPreview
    {anime}
    placement={hover.placement}
    measure={hover.measure}
    onenter={hover.cancelClose}
    onleave={hover.scheduleClose}
  />
{/if}