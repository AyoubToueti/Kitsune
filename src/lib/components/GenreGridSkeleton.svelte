<script lang="ts">
  import Skeleton from "./Skeleton.svelte";

  /** Widths cycle so the pill row reads as varied labels, not a barcode. */
  const WIDTHS = [
    "w-16",
    "w-24",
    "w-20",
    "w-14",
    "w-28",
    "w-18",
    "w-22",
    "w-16",
    "w-24",
    "w-20",
    "w-14",
    "w-28",
    "w-18",
    "w-22",
  ];
</script>

<!-- Mirrors GenreGrid: a heading over a wrapped row of pill placeholders. -->
<section class="mt-8">
  <h2 class="mb-3 text-lg font-semibold tracking-tight">Browse by genre</h2>

  <div
    role="status"
    aria-busy="true"
    class="flex flex-wrap gap-2"
    data-testid="genre-skeleton"
  >
    <span class="sr-only">Loading…</span>
    {#each WIDTHS as width, i (i)}
      <Skeleton class="h-8 {width} rounded-full" />
    {/each}
  </div>
</section>