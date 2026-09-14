<script lang="ts">
  import Skeleton from "./Skeleton.svelte";
</script>

<!-- Mirrors HeroCarousel: the fixed-height banner, the spotlight column on
     the left (kicker, title, facts, chips, synopsis, button), the arrow
     buttons on the right edge, and the dot controls at the bottom. -->
<section
  role="status"
  aria-busy="true"
  data-testid="hero-skeleton"
  class="relative h-[22rem] overflow-hidden rounded-xl border border-border-subtle md:h-[26rem]"
>
  <span class="sr-only">Loading…</span>

  <!-- Backdrop artwork. -->
  <Skeleton class="absolute inset-0 rounded-none" />

  <!-- Spotlight column. -->
  <div class="relative flex h-full max-w-2xl flex-col justify-center gap-3 px-8">
    <Skeleton class="h-4 w-24" />

    <Skeleton class="h-9 w-2/3 md:w-1/2" />

    <div class="flex flex-wrap gap-4">
      <Skeleton class="h-4 w-8" />
      <Skeleton class="h-4 w-10" />
      <Skeleton class="h-4 w-12" />
      <Skeleton class="h-4 w-14" />
    </div>

    <div class="flex gap-2">
      <Skeleton class="h-5 w-16 rounded-full" />
      <Skeleton class="h-5 w-20 rounded-full" />
      <Skeleton class="h-5 w-14 rounded-full" />
    </div>

    <div class="max-w-xl space-y-2">
      <Skeleton class="h-3 w-full" />
      <Skeleton class="h-3 w-5/6" />
      <Skeleton class="h-3 w-2/3" />
    </div>

    <div class="mt-2">
      <Skeleton class="h-9 w-28 rounded-full" />
    </div>
  </div>

  <!-- Arrow buttons, stacked on the right edge like the real carousel. -->
  <div class="absolute top-1/2 right-4 flex -translate-y-1/2 flex-col gap-2">
    <Skeleton class="h-9 w-9 rounded-full" />
    <Skeleton class="h-9 w-9 rounded-full" />
  </div>

  <!-- Dot controls. -->
  <ul class="absolute bottom-4 left-8 flex gap-2" aria-hidden="true">
    {#each Array(5) as _, i (i)}
      <li>
        <Skeleton class="h-2 w-2 rounded-full {i === 0 ? '' : 'opacity-60'}" />
      </li>
    {/each}
  </ul>
</section>