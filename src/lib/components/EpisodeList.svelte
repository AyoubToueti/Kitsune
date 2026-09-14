<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { StreamingEpisode } from "$lib/types";

  let { episodes = [] }: { episodes?: StreamingEpisode[] } = $props();

  /**
   * Best label for an episode.
   *
   * Falls back through the title, the site, then the URL, mirroring
   * `StreamingLinks`: AniList sometimes leaves the title blank, but a card with
   * no caption at all would look broken.
   */
  function label(ep: StreamingEpisode): string {
    return ep.title ?? ep.site ?? ep.url;
  }
</script>

{#if episodes.length > 0}
  <div>
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Episodes</h2>
    <ul class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
      {#each episodes as ep (ep.url)}
        <li>
          <button
            type="button"
            onclick={() => openUrl(ep.url)}
            aria-label={label(ep)}
            class="group relative block aspect-video w-full overflow-hidden rounded-lg border border-border-subtle bg-surface-hover text-left transition-colors hover:border-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            {#if ep.thumbnail}
              <img
                src={ep.thumbnail}
                alt=""
                loading="lazy"
                class="absolute inset-0 h-full w-full object-cover transition-transform duration-200 group-hover:scale-105"
              />
            {/if}

            <!-- A bottom gradient so the caption stays readable over any frame. -->
            <span
              aria-hidden="true"
              class="absolute inset-x-0 bottom-0 h-2/3 bg-gradient-to-t from-black/85 to-transparent"
            ></span>

            <span
              class="absolute inset-x-0 bottom-0 line-clamp-2 px-2 py-1.5 text-xs font-medium text-white"
            >
              {label(ep)}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  </div>
{/if}