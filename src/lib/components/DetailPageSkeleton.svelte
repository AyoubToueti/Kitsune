<script lang="ts">
  import Skeleton from "./Skeleton.svelte";
</script>

<!-- Mirrors the detail page's four regions in order: hero, episodes, sidebar,
     recommendations. Heights match the real blocks so the swap-in does not
     shift the page. -->
<div role="status" aria-busy="true" data-testid="detail-skeleton">
  <span class="sr-only">Loading…</span>

  <!-- Hero: banner backdrop, cover, title, genre chips, synopsis. -->
  <section class="relative overflow-hidden rounded-xl border border-border-subtle">
    <Skeleton class="absolute inset-0 rounded-none" />
    <div class="relative flex gap-5 p-6">
      <Skeleton class="hidden w-32 shrink-0 rounded-lg sm:block" />
      <div class="min-w-0 flex-1">
        <Skeleton class="h-8 w-1/2" />
        <div class="mt-3 flex gap-2">
          {#each Array(6) as _, i (i)}
            <Skeleton class="h-5 w-16 rounded-full" />
          {/each}
        </div>
        <div class="mt-4 space-y-2">
          <Skeleton class="h-3 w-full" />
          <Skeleton class="h-3 w-5/6" />
          <Skeleton class="h-3 w-2/3" />
        </div>
      </div>
    </div>
  </section>

  <!-- Episodes and relations share one row, like the real page. -->
  <div class="mt-8 grid gap-8 lg:grid-cols-[1fr_20rem]">
    <div>
      <Skeleton class="mb-3 h-6 w-24" />
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
        {#each Array(8) as _, i (i)}
          <Skeleton class="aspect-video w-full rounded-lg" />
        {/each}
      </div>
    </div>

    <aside>
      <Skeleton class="mb-4 h-40 w-full rounded-lg" />
      <Skeleton class="mb-3 h-6 w-32" />
      {#each Array(3) as _, i (i)}
        <div class="mb-2 flex gap-3">
          <Skeleton class="aspect-2/3 w-12 shrink-0" />
          <div class="min-w-0 flex-1">
            <Skeleton class="h-4 w-3/4" />
            <Skeleton class="mt-1.5 h-3 w-20" />
          </div>
        </div>
      {/each}
    </aside>
  </div>

  <!-- Recommendations poster row. -->
  <div class="mt-8">
    <Skeleton class="mb-3 h-6 w-40" />
    <div class="flex gap-4 overflow-hidden">
      {#each Array(6) as _, i (i)}
        <div class="w-40 shrink-0">
          <Skeleton class="aspect-[2/3] w-full rounded-lg" />
          <Skeleton class="mt-2 h-4 w-3/4" />
        </div>
      {/each}
    </div>
  </div>
</div>