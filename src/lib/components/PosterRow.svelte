<script lang="ts">
  import type { Anime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";

  let { title, anime }: { title: string; anime: Anime[] } = $props();
</script>

{#if anime.length}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">{title}</h2>

    <ul class="flex gap-4 overflow-x-auto pb-2">
      {#each anime as item (item.id)}
        <li>
          <AnimeCard anime={item} />
        </li>
      {/each}
    </ul>
  </section>
{/if}