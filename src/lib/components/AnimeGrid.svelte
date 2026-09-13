<script lang="ts">
  import type { Anime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";

  let { anime }: { anime: Anime[] } = $props();
</script>

<!-- Extracted because /search, /genre and /filter all render the same grid.
     Wrapping and spacing live here so the three routes cannot drift apart. -->
<ul class="flex flex-wrap gap-4" data-testid="anime-grid">
  {#each anime as item (item.id)}
    <li>
      <AnimeCard anime={item} />
    </li>
  {/each}
</ul>