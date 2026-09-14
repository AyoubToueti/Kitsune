<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { StreamingEpisode } from "$lib/types";

  let { episodes = [] }: { episodes?: StreamingEpisode[] } = $props();

  // Display name: prefer the site, fall back to the title, then the URL
  // itself. AniList sometimes leaves both blank, but `url` is always there
  // (the type enforces it).
  function label(ep: StreamingEpisode): string {
    return ep.site ?? ep.title ?? ep.url;
  }
</script>

<!-- {#if episodes.length > 0}
  <div>
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Where to watch</h2>
    <ul class="flex flex-wrap gap-3">
      {#each episodes as ep (ep.url)}
        <li>
          <button
            type="button"
            onclick={() => openUrl(ep.url)}
            class="rounded-lg border border-border-subtle bg-surface-hover px-4 py-2 text-sm font-medium text-ink transition-colors hover:border-accent hover:text-accent"
          >
            {label(ep)}
          </button>
        </li>
      {/each}
    </ul>
  </div>
{/if} -->