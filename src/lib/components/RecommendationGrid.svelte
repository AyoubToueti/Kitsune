<script lang="ts">
  import type { RecommendedAnime } from "$lib/types";
  import RecommendationCard from "./RecommendationCard.svelte";

  let {
    recommendations,
    votable = false,
    mediaId,
  }: {
    recommendations: readonly RecommendedAnime[];
    votable?: boolean;
    mediaId?: number;
  } = $props();
</script>

<!-- Same flex-wrap layout as AnimeGrid, so a recommendations page reads like
     every other poster grid in the app. -->
<ul class="flex flex-wrap gap-4" data-testid="recommendation-grid">
  {#each recommendations as item (item.anime.id)}
    <li>
      <RecommendationCard recommendation={item} {votable} {mediaId} />
    </li>
  {/each}
</ul>