<script lang="ts">
  import { onMount } from "svelte";

  import {
    canScrollLeft,
    canScrollRight,
    railSlideWidth,
  } from "$lib/scroll-strip";
  import { createStripDrag } from "$lib/strip-drag.svelte";
  import type { Anime } from "$lib/types";
  import TrendingCard from "./TrendingCard.svelte";

  let { anime }: { anime: Anime[] } = $props();

  /** The rail's `gap-4`, in pixels. Used to divide the width into slides. */
  const GAP_PX = 16;

  /** The narrowest a slide may be, so the rank and poster stay legible. */
  const MIN_SLIDE_WIDTH = 180;

  let rail = $state<HTMLElement | null>(null);

  /**
   * The exact width each slide is given.
   *
   * Set so a whole number fills the row, leaving no half slide at the edge.
   * `null` until measured; the fallback keeps slides rendering before layout.
   */
  let slideWidth = $state<number | null>(null);

  // Drag-to-scroll with a release glide, exactly as the day strip uses. The
  // stride getter reads `slideWidth`, which is assigned before any gesture can
  // end, so the closure always sees a real value.
  const drag = createStripDrag(() => rail, {
    getStride: () => (slideWidth ?? MIN_SLIDE_WIDTH) + GAP_PX,
  });

  /** Whether a nav button has anywhere to go. */
  let atStart = $state(true);
  let atEnd = $state(true);

  /** Re-read the rail's geometry: resize the slides, then update the buttons. */
  function sync() {
    if (!rail) return;

    slideWidth = railSlideWidth(rail.clientWidth, MIN_SLIDE_WIDTH, GAP_PX);

    atStart = !canScrollLeft(rail.scrollLeft);
    atEnd = !canScrollRight(rail.scrollLeft, rail.clientWidth, rail.scrollWidth);
  }

  /** Move the rail one whole slide, easing like a drag release. */
  function scrollStep(direction: 1 | -1) {
    if (!rail) return;
    const stride = (slideWidth ?? MIN_SLIDE_WIDTH) + GAP_PX;
    const maxScroll = Math.max(0, rail.scrollWidth - rail.clientWidth);
    const target = Math.min(
      Math.max(rail.scrollLeft + stride * direction, 0),
      maxScroll,
    );
    drag.glideTo(target);
  }

  // Read the geometry once the rail is bound, and again whenever the window
  // resizes (the visible width, and so the slide count, changes with it).
  onMount(() => {
    sync();
    window.addEventListener("resize", sync);
    return () => window.removeEventListener("resize", sync);
  });

</script>

{#if anime.length}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
      Trending
    </h2>

    <!-- The body reserves a 60px gutter on the right for the nav buttons, so
         they never overlap the cards. -->
    <div class="relative pr-15">
      <!-- `no-scrollbar` + `snap-strip`: the rail hides its scrollbar and rests
           on whole slides, so a slide is never left half cut off. -->
      <ul
        bind:this={rail}
        data-testid="trending-rail"
        role="presentation"
        class="no-scrollbar snap-strip flex gap-4 overflow-x-auto pb-2 {drag.dragging
          ? 'strip-dragging select-none cursor-grabbing'
          : drag.settling
            ? 'strip-settling'
            : ''}"
        onpointerdown={drag.onpointerdown}
        onclickcapture={drag.onclickcapture}
        ondragstart={drag.ondragstart}
        onscroll={sync}
      >
        {#each anime as item, i (item.id)}
          <li
            class="shrink-0"
            style="width: {slideWidth ?? MIN_SLIDE_WIDTH}px;"
          >
            <!-- A dedicated card: vertical title on the left, rank number over
                 the poster's bottom-left corner. `AnimeCard` renders its title
                 under the artwork, which is a different shape. -->
            <TrendingCard anime={item} rank={i + 1} />
          </li>
        {/each}
      </ul>

      <!-- Stacked nav in the gutter, spanning the slides' height. Each button
           is its own rounded pill; `next` sits on top, `prev` below. -->
      <div class="absolute top-0 right-0 bottom-2 w-10">
        <button
          type="button"
          data-testid="trending-next"
          aria-label="Scroll to later titles"
          disabled={atEnd}
          onclick={() => scrollStep(1)}
          class="absolute top-0 right-0 left-0 flex h-[48%] items-center justify-center rounded-[10px] bg-white/10 text-ink transition-colors hover:bg-accent hover:text-[#111] focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-default disabled:opacity-30"
        >
          <span aria-hidden="true" class="text-lg leading-none">›</span>
        </button>

        <button
          type="button"
          data-testid="trending-prev"
          aria-label="Scroll to earlier titles"
          disabled={atStart}
          onclick={() => scrollStep(-1)}
          class="absolute right-0 bottom-0 left-0 flex h-[48%] items-center justify-center rounded-[10px] bg-white/10 text-ink transition-colors hover:bg-accent hover:text-[#111] focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-default disabled:opacity-30"
        >
          <span aria-hidden="true" class="text-lg leading-none">‹</span>
        </button>
      </div>
    </div>
  </section>
{/if}