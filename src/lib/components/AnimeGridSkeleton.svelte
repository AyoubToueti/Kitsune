<script lang="ts">
  import AnimeCardSkeleton from "./AnimeCardSkeleton.svelte";

  /**
   * How many placeholders to draw.
   *
   * Defaults to a screenful; callers pass the page size they are about to
   * fetch so the skeleton predicts the real density.
   */
  let { count = 10 }: { count?: number } = $props();
</script>

<!-- Same flex-wrap layout as AnimeGrid, so nothing reflows when the real
     cards arrive. -->
<div
  role="status"
  aria-busy="true"
  class="flex flex-wrap gap-4"
  data-testid="grid-skeleton"
>
  <span class="sr-only">Loading…</span>
  {#each Array(count) as _, i (i)}
    <AnimeCardSkeleton />
  {/each}
</div>