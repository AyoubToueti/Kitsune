<script lang="ts">
  import Skeleton from "./Skeleton.svelte";
</script>

<!-- Mirrors the redesigned detail page's regions: hero, episodes, sticky sidebar,
     and recommendations poster row. Sizes match the active layout to eliminate layout shift on load. -->
<div role="status" aria-busy="true" data-testid="detail-skeleton">
  <span class="sr-only">Loading…</span>

  <!-- Redesigned Hero: banner backdrop, cover, title, genre chips, action bar, metadata, synopsis -->
  <section class="relative overflow-hidden rounded-2xl border border-border-subtle/80 bg-surface">
    <Skeleton class="absolute inset-0 rounded-none opacity-40" />
    <div class="relative flex flex-col gap-6 p-6 sm:p-8 md:flex-row md:items-start">
      <!-- Poster Skeleton -->
      <Skeleton class="hidden w-40 shrink-0 rounded-xl sm:block sm:w-48 aspect-2/3 shadow-2xl" />

      <!-- Info Column Skeleton -->
      <div class="min-w-0 flex-1">
        <!-- Title -->
        <Skeleton class="h-9 w-2/3 max-w-md rounded-lg" />

        <!-- Genre Pills -->
        <div class="mt-3 flex flex-wrap gap-1.5">
          {#each Array(5) as _, i (i)}
            <Skeleton class="h-6 w-16 rounded-lg" />
          {/each}
        </div>

        <!-- Action Bar Button -->
        <div class="mt-4 flex items-center border-y border-border-subtle/50 py-3">
          <Skeleton class="h-9 w-36 rounded-xl" />
        </div>

        <!-- Metadata Strip Placeholder -->
        <div class="mt-3 flex items-center gap-3">
          <Skeleton class="h-4 w-16" />
          <Skeleton class="h-4 w-20" />
          <Skeleton class="h-4 w-14" />
        </div>

        <!-- Synopsis Lines -->
        <div class="mt-4 space-y-2">
          <Skeleton class="h-3.5 w-full rounded-md" />
          <Skeleton class="h-3.5 w-11/12 rounded-md" />
          <Skeleton class="h-3.5 w-4/5 rounded-md" />
        </div>
      </div>
    </div>
  </section>

  <!-- Content Row: Episodes & Sidebar -->
  <div class="mt-8 grid gap-8 lg:grid-cols-[1fr_20rem]">
    <!-- Left Column: Episodes Grid -->
    <div>
      <Skeleton class="mb-4 h-6 w-32 rounded-md" />
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
        {#each Array(8) as _, i (i)}
          <Skeleton class="aspect-video w-full rounded-xl" />
        {/each}
      </div>
    </div>

    <!-- Right Sidebar -->
    <aside class="space-y-6">
      <!-- Trailer Card -->
      <Skeleton class="aspect-video w-full rounded-xl" />
      
      <!-- Related Items -->
      <div class="space-y-3">
        <Skeleton class="h-6 w-36 rounded-md" />
        {#each Array(3) as _, i (i)}
          <div class="flex items-center gap-3">
            <Skeleton class="aspect-2/3 w-12 shrink-0 rounded-lg" />
            <div class="min-w-0 flex-1 space-y-2">
              <Skeleton class="h-4 w-3/4 rounded-md" />
              <Skeleton class="h-3 w-1/2 rounded-md" />
            </div>
          </div>
        {/each}
      </div>
    </aside>
  </div>

  <!-- Recommendations Carousel Section -->
  <div class="mt-12 border-t border-border-subtle/60 pt-8">
    <Skeleton class="mb-4 h-6 w-44 rounded-md" />
    <div class="flex gap-4 overflow-hidden">
      {#each Array(6) as _, i (i)}
        <div class="w-40 shrink-0 space-y-2">
          <Skeleton class="aspect-2/3 w-full rounded-xl" />
          <Skeleton class="h-4 w-3/4 rounded-md" />
        </div>
      {/each}
    </div>
  </div>
</div>