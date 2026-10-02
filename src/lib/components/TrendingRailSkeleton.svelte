<script lang="ts">
  import Skeleton from "./Skeleton.svelte";

  /** The default card slide width matching MIN_SLIDE_WIDTH in TrendingRail */
  const SLIDE_WIDTH = 180;
</script>

<section class="mt-8">
  <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
    Trending
  </h2>

  <!-- Container reserves the same 60px right gutter (`pr-15`) as TrendingRail -->
  <div
    role="status"
    aria-busy="true"
    class="relative pr-15"
    data-testid="trending-skeleton"
  >
    <span class="sr-only">Loading trending anime…</span>

    <div class="no-scrollbar flex gap-4 overflow-hidden pb-2">
      {#each Array(6) as _, i (i)}
        <div
          class="shrink-0 rounded-2xl border border-border-subtle/40 bg-surface-raised/40 p-2 shadow-sm"
          style="width: {SLIDE_WIDTH}px;"
        >
          <div class="flex items-center gap-2">
            <!-- Rank Number Column -->
            <div class="flex w-7 shrink-0 items-center justify-center">
              <Skeleton class="h-8 w-5 rounded-md" />
            </div>

            <!-- Poster Artwork & Title Skeleton -->
            <div class="flex-1 min-w-0">
              <Skeleton class="aspect-[2/3] w-full rounded-xl" />
            </div>
          </div>

          <!-- Bottom Meta & Title Skeleton -->
          <div class="mt-2.5 px-1 space-y-1.5">
            <Skeleton class="h-3.5 w-5/6 rounded" />
            <Skeleton class="h-3 w-1/2 rounded" />
          </div>
        </div>
      {/each}
    </div>

    <!-- Nav Buttons Placeholder (Matching layout position) -->
    <div class="absolute top-0 right-0 bottom-2 w-10 flex flex-col justify-between">
      <Skeleton class="h-[48%] w-full rounded-[10px]" />
      <Skeleton class="h-[48%] w-full rounded-[10px]" />
    </div>
  </div>
</section>