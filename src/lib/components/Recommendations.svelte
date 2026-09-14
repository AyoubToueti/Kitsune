<script lang="ts">
  import type { RecommendedAnime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";

  let { recommendations = [] }: { recommendations?: RecommendedAnime[] } = $props();
</script>

{#if recommendations.length > 0}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Recommended</h2>
    <!-- A poster row rather than a list: recommendations are "more like this",
         so they read best at cover size, matching the home page's rails. -->
    <ul class="flex gap-4 overflow-x-auto pb-2">
      {#each recommendations as item (item.anime.id)}
        <li>
          <AnimeCard anime={item.anime} />
        </li>
      {/each}
    </ul>
  </section>
{/if}