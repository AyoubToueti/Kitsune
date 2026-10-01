<script lang="ts">
  import Skeleton from "./Skeleton.svelte";

  /**
   * `fluid` mirrors AnimeCard: lets the card fill its parent container cell
   * instead of staying fixed at w-40.
   */
  let { fluid = false }: { fluid?: boolean } = $props();
</script>

<div
  role="status"
  aria-label="Loading anime card..."
  class="flex flex-col overflow-hidden rounded-xl border border-border-subtle bg-surface-raised shadow-md {fluid
    ? 'w-full min-w-0'
    : 'w-40 shrink-0'}"
>
  <!-- Artwork Skeleton -->
  <div class="relative aspect-2/3 w-full bg-surface-hover">
    <Skeleton class="h-full w-full rounded-none" />
  </div>

  <!-- Title & Metadata Skeleton -->
  <div class="flex flex-1 flex-col justify-between p-2.5">
    <div class="space-y-1.5">
      <Skeleton class="h-3.5 w-full rounded" />
      <!-- <Skeleton class="h-3.5 w-2/3 rounded" /> -->
    </div>
  </div>
</div>