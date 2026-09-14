<script lang="ts">
  import Skeleton from "./Skeleton.svelte";

  /** How many ranked rows to draw. */
  let { count = 10 }: { count?: number } = $props();
</script>

<!-- Mirrors TopList rows: rank number, poster thumb, title and genre lines,
     and the score column that appears from sm up. -->
<div
  role="status"
  aria-busy="true"
  class="divide-y divide-border-subtle"
  data-testid="toplist-skeleton"
>
  <span class="sr-only">Loading…</span>
  {#each Array(count) as _, i (i)}
    <div class="flex items-center gap-4 px-3 py-3">
      <Skeleton class="h-4 w-6 shrink-0" />

      <Skeleton class="aspect-[2/3] w-12 shrink-0" />

      <div class="min-w-0 flex-1">
        <Skeleton class="h-4 w-1/2" />
        <div class="mt-1.5 flex gap-1">
          <Skeleton class="h-3 w-14 rounded-full" />
          <Skeleton class="h-3 w-10 rounded-full" />
        </div>
      </div>

      <div class="hidden w-24 shrink-0 sm:block">
        <Skeleton class="ml-auto h-4 w-10" />
      </div>
    </div>
  {/each}
</div>