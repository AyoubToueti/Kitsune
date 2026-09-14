<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Trailer } from "$lib/types";

  let { trailer }: { trailer?: Trailer } = $props();

  /**
   * Build the watch URL for a trailer.
   *
   * AniList reports the platform and the video id separately, and each platform
   * spells its URL differently, so the mapping lives here. An unknown site has no
   * URL to build, which the caller treats as "no trailer".
   */
  function watchUrl(video: Trailer): string | null {
    switch (video.site.toLowerCase()) {
      case "youtube":
        return `https://www.youtube.com/watch?v=${video.id}`;
      case "dailymotion":
        return `https://www.dailymotion.com/video/${video.id}`;
      default:
        return null;
    }
  }

  const url = $derived(trailer ? watchUrl(trailer) : null);
</script>

{#if trailer && url}
  <div class="mt-6">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Trailer</h2>
    <button
      type="button"
      onclick={() => openUrl(url)}
      aria-label="Play trailer"
      class="group relative block aspect-video w-full overflow-hidden rounded-lg border border-border-subtle bg-surface-hover transition-colors hover:border-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      {#if trailer.thumbnail}
        <img
          src={trailer.thumbnail}
          alt=""
          loading="lazy"
          class="absolute inset-0 h-full w-full object-cover transition-transform duration-200 group-hover:scale-105"
        />
      {/if}

      <!-- Centred play affordance, matching AnimeCard's hover mark. Decorative:
           the button already carries the accessible name. -->
      <span
        aria-hidden="true"
        class="absolute inset-0 flex items-center justify-center text-4xl text-white/90"
      >
        ▶
      </span>
    </button>
  </div>
{/if}