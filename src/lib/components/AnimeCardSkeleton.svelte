<script lang="ts">
  import Skeleton from "./Skeleton.svelte";

  /**
   * `fluid` mirrors AnimeCard: lets the card fill its parent container cell
   * instead of staying fixed at w-44.
   */
  let { fluid = false }: { fluid?: boolean } = $props();
</script>

<div
  role="status"
  aria-label="Loading anime card..."
  class="flex flex-col overflow-hidden rounded-2xl border border-border-subtle/80 bg-surface-raised/90 shadow-sm {fluid
    ? 'w-full min-w-0'
    : 'w-44 shrink-0'}"
>
  <!-- Artwork Skeleton -->
  <div class="relative aspect-[2/3] w-full bg-surface-hover">
    <Skeleton class="h-full w-full rounded-none" />
  </div>

  <!-- Title & Extra Details Skeleton -->
  <div class="flex flex-1 flex-col justify-between p-3">
    <div class="space-y-1.5">
      <Skeleton class="h-3.5 w-full rounded" />
      <Skeleton class="h-3.5 w-2/3 rounded" />
    </div>

    <!-- Episode Count Placeholder -->
    <Skeleton class="mt-2 h-3 w-1/3 rounded" />
  </div>
</div>