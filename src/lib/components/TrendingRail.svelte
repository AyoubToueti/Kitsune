<script lang="ts">
  import type { Anime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";

  let { anime }: { anime: Anime[] } = $props();

  /** Zero-padded rank, so the badge column stays aligned past nine. */
  function rank(position: number): string {
    return String(position).padStart(2, "0");
  }
</script>

{#if anime.length}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
      Trending
    </h2>

    <ul class="flex gap-4 overflow-x-auto pb-2">
      {#each anime as item, i (item.id)}
        <li class="flex shrink-0 items-start gap-2">
          <!-- The rank, set large and rotated so it reads bottom-to-top up the
               left edge. `writing-mode` plus a 180° turn makes the text run
               upwards rather than downwards. -->
          <div class="flex h-60 items-center">
            <span
              data-testid="rank-badge"
              class="text-4xl font-bold leading-none text-ink-faint"
              style="writing-mode: vertical-rl; transform: rotate(180deg);"
            >
              {rank(i + 1)}
            </span>
          </div>

          <!-- Reusing AnimeCard means the rail inherits the hover preview, the
               poster blur and the play overlay, rather than duplicating them. -->
          <AnimeCard anime={item} />
        </li>
      {/each}
    </ul>
  </section>
{/if}