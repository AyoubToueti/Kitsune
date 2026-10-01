<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { createHoverPreview } from "$lib/hover-preview.svelte";
  import HoverPreview from "./HoverPreview.svelte";

  let { anime, rank }: { anime: Anime; rank: number } = $props();

  /**
   * The title, trimmed to a fixed budget so the vertical line never runs too
   * long. Computed inside the `$derived` so it tracks the `anime` prop.
   */
  const title = $derived.by(() => {
    const full = displayTitle(anime.title) ?? "Untitled";
    return full.length > 23 ? `${full.slice(0, 20)}...` : full;
  });

  /** Zero-padded, so the number column stays aligned past nine. */
  const label = $derived(String(rank).padStart(2, "0"));

  /** The preview anchors to the info circle, which is its trigger. */
  let trigger = $state<HTMLElement | null>(null);
  const hover = createHoverPreview(() => trigger);
</script>

<!-- One trending item: the vertical title runs up the left edge, the poster
     sits beside it with the rank number over its bottom-left corner. This is
     deliberately NOT `AnimeCard`: that one renders a title under the artwork,
     which is a different shape from the trending row. -->
<div class="flex items-stretch gap-1.5">
  <!-- The outer span has NO intrinsic height (its only child is absolutely
       positioned), so it stretches to the poster's height instead of growing
       with the title. `overflow-hidden` then trims a long title. This is what
       keeps every card the same size no matter how long the name is. -->
  <span class="relative w-[20px] shrink-0 overflow-hidden">
    <!-- `writing-mode` plus a 180 turn makes the text run bottom-to-top. -->
    <span
      class="absolute bottom-0 left-0 whitespace-nowrap pb-1 text-base font-semibold text-ink"
      style="writing-mode: vertical-rl; transform: rotate(180deg);"
    >
      {title}
    </span>
  </span>

  <!-- Wrapper so the info trigger can sit over the poster's top-left as a
       sibling of the link (a button inside an anchor is invalid). -->
  <div class="group relative min-w-0 flex-1">
    <a
      href={`/anime/${anime.id}`}
      aria-label={title}
      draggable="false"
      class="block focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      <div
        data-testid="trending-poster"
        class="relative aspect-[2/3] overflow-hidden rounded-md bg-surface-hover"
      >
        {#if anime.coverImage}
          <img
            src={anime.coverImage}
            alt=""
            loading="lazy"
            draggable="false"
            class="h-full w-full object-cover transition-transform duration-200 group-hover:scale-105"
          />
        {:else}
          <div class="flex h-full w-full items-center justify-center text-xs text-ink-faint">
            No cover
          </div>
        {/if}

        <!-- The rank number, set large and hung off the poster's bottom-left
             corner. Decorative: the link carries the accessible name, and the
             number is repeated in the item's own text for a screen reader. -->
        <span
          aria-hidden="true"
          data-testid="rank-badge"
          class="pointer-events-none absolute bottom-0 left-0 px-2 py-1 text-4xl font-bold leading-none text-white [text-shadow:0_2px_10px_rgba(0,0,0,0.85)]"
        >
          {label}
        </span>

        {#if anime.averageScore != null}
          <span
            class="absolute top-2 right-2 rounded-full bg-surface/85 px-2 py-0.5 text-xs font-semibold text-score"
          >
            {anime.averageScore}
          </span>
        {/if}
      </div>
    </a>

    <!-- The preview trigger: a small circle at the poster's top-left, shown on
         hover/focus of the card. It is the ONLY thing that opens the preview. -->
    <button
      bind:this={trigger}
      type="button"
      data-testid="info-trigger"
      aria-label={`Preview ${title}`}
      class="absolute top-2 left-2 flex h-6 w-6 items-center justify-center rounded-full border border-border-subtle bg-surface/80 text-[11px] font-bold text-ink backdrop-blur-sm transition-colors hover:bg-accent hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onmouseenter={hover.show}
      onmouseleave={hover.scheduleClose}
      onfocusin={hover.show}
      onfocusout={hover.scheduleClose}
    >
      i
    </button>
  </div>
</div>

{#if hover.open}
  <!-- Sibling of the link, not a child: nesting links inside a link is invalid,
       and the preview has its own genre links. -->
  <HoverPreview
    {anime}
    placement={hover.placement}
    measure={hover.measure}
    onenter={hover.enterPreview}
    onleave={hover.leavePreview}
  />
{/if}