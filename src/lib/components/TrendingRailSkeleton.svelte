<script lang="ts">
  import Skeleton from "./Skeleton.svelte";

  /** The default card slide width matching MIN_SLIDE_WIDTH in TrendingRail */
  const SLIDE_WIDTH = 180;
</script>

<section class="mt-8">
  <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
    Trending
  </h2>

  <div
    role="status"
    aria-busy="true"
    class="relative pr-15"
    data-testid="trending-skeleton"
  >
    <span class="sr-only">Loading trending anime…</span>

    <!-- Matches the <ul> structure and classes of TrendingRail (overflow-hidden to prevent skeleton scrolling) -->
    <ul
      role="presentation"
      class="no-scrollbar snap-strip flex gap-4 overflow-hidden pb-2"
    >
      {#each Array(6) as _, i (i)}
        <li class="shrink-0" style="width: {SLIDE_WIDTH}px;">
          <div class="flex h-full gap-2">
            <!-- Vertical Title Skeleton (Left side) -->
            <div class="flex w-6 shrink-0 items-center justify-center py-2">
              <Skeleton class="h-3/4 w-3 rounded" />
            </div>

            <!-- Poster Artwork & Rank Number Skeleton (Right side) -->
            <div class="relative flex-1 min-w-0">
              <Skeleton class="aspect-2/3 w-full rounded-xl" />
              <!-- Rank number overlay matching bottom-left corner placement -->
              <Skeleton class="absolute bottom-2 left-2 h-6 w-6 rounded-md" />
            </div>
          </div>
        </li>
      {/each}
    </ul>

    <!-- Nav Buttons Placeholder (Matching the exact absolute positioning layout of TrendingRail) -->
    <div class="absolute top-0 right-0 bottom-2 w-10">
      <Skeleton
        class="absolute top-0 right-0 left-0 h-[48%] w-full rounded-[10px]"
      />
      <Skeleton
        class="absolute right-0 bottom-0 left-0 h-[48%] w-full rounded-[10px]"
      />
    </div>
  </div>
</section>
