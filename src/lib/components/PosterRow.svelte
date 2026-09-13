<script lang="ts">
  import type { Anime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";

  let {
    title,
    anime,
    // Opt-in: some rows are rankings (a "top 10"), most are not. Defaulting to
    // false keeps existing callers unchanged.
    numbered = false,
  }: { title: string; anime: Anime[]; numbered?: boolean } = $props();

  /** Zero-padded rank, so the column stays visually aligned past 9. */
  function rank(position: number): string {
    return String(position).padStart(2, "0");
  }
</script>

{#if anime.length}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">{title}</h2>

    <ul class="flex gap-4 overflow-x-auto pb-2">
      {#each anime as item, i (item.id)}
        <li class="relative">
          {#if numbered}
            <!-- Decorative: the card's own link already names the title, so the
                 rank is hidden from assistive tech to avoid a duplicate
                 announcement. -->
            <span
              aria-hidden="true"
              data-testid="rank-badge"
              class="absolute -top-2 -left-2 z-10 rounded-full bg-accent px-2 py-0.5 text-xs font-bold text-white"
            >
              {rank(i + 1)}
            </span>
          {/if}
          <AnimeCard anime={item} />
        </li>
      {/each}
    </ul>
  </section>
{/if}