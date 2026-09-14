<script lang="ts">
  import Skeleton from "./Skeleton.svelte";
</script>

<!-- Mirrors AnimeCard: a w-40 poster block with a two-line title beneath. -->
<div class="block w-40 shrink-0">
  <Skeleton class="aspect-[2/3] w-full rounded-lg" />
  <Skeleton class="mt-2 h-4 w-3/4" />
</div>