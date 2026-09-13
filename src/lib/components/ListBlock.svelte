<script lang="ts">
  import type { Anime, ListFilter } from "$lib/types";
  import AnimeListItem from "./AnimeListItem.svelte";

  let {
    title,
    anime,
    filter,
  }: { title: string; anime: Anime[]; filter: ListFilter } = $props();
</script>

{#if anime.length}
  <section class="flex flex-col rounded-xl border border-border-subtle p-3">
    <h2 class="mb-2 text-sm font-semibold tracking-tight text-accent">
      {title}
    </h2>

    <ul class="flex-1 divide-y divide-border-subtle">
      {#each anime as item (item.id)}
        <li>
          <AnimeListItem anime={item} />
        </li>
      {/each}
    </ul>

    <!-- `mt-auto` keeps the footer pinned to the bottom, so blocks with
         different item counts still line their "View more" up. -->
    <a
      href={`/browse/${filter}`}
      class="mt-auto pt-3 text-sm text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      View more ›
    </a>
  </section>
{/if}