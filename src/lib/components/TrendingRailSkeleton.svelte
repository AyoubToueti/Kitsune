<script lang="ts">
  import Skeleton from "./Skeleton.svelte";
</script>

<!-- Mirrors TrendingRail: the accent heading, then a row of cards each
     fronted by a tall vertical rank number. -->
<section class="mt-8">
  <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
    Trending
  </h2>

  <div
    role="status"
    aria-busy="true"
    class="flex gap-4 overflow-hidden pb-2"
    data-testid="trending-skeleton"
  >
    <span class="sr-only">Loading…</span>

    {#each Array(6) as _, i (i)}
      <div class="flex shrink-0 items-start gap-2">
        <!-- The rank column: a tall bar where the number would sit. -->
        <div class="flex h-60 items-center">
          <Skeleton class="h-10 w-6" />
        </div>

        <div class="w-40 shrink-0">
          <Skeleton class="aspect-[2/3] w-full rounded-lg" />
          <Skeleton class="mt-2 h-4 w-3/4" />
        </div>
      </div>
    {/each}
  </div>
</section>