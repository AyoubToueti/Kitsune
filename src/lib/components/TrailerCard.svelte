<script lang="ts">
  import type { Trailer } from "$lib/types";

  let { trailer }: { trailer?: Trailer } = $props();

  // Manage modal open state
  let isOpen = $state(false);

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
        return `https://youtube.com{video.id}`;
      case "dailymotion":
        return `https://dailymotion.com{video.id}`;
      default:
        return null;
    }
  }

  /**
   * Build the specific embed URL variant required to play inside an iframe overlay.
   */
  function embedUrl(video: Trailer): string | null {
    switch (video.site.toLowerCase()) {
      case "youtube":
        return `https://www.youtube.com/embed/${video.id}?autoplay=1`;
      case "dailymotion":
        return `https://www.dailymotion.com/embed/video/${video.id}?autoplay=1`;
      default:
        return null;
    }
  }

  // Handle closing modal via Escape key
  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      isOpen = false;
    }
  }

  const url = $derived(trailer ? watchUrl(trailer) : null);
  const videoEmbedUrl = $derived(trailer ? embedUrl(trailer) : null);
</script>

<!-- Global window listener to catch the Escape key shortcut automatically -->
<svelte:window onkeydown={handleKeyDown} />

{#if trailer && url && videoEmbedUrl}
  <div class="mt-6">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Trailer</h2>
    <button
      type="button"
      onclick={() => (isOpen = true)}
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

<!-- Overlay Video Modal -->
{#if isOpen && videoEmbedUrl}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/85 p-4 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    onclick={() => (isOpen = false)}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="relative w-full max-w-4xl aspect-video overflow-hidden rounded-xl bg-black shadow-2xl border border-border-subtle"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Video Player Frame -->
      <iframe
        src={videoEmbedUrl}
        title="Trailer Player"
        class="h-full w-full border-0"
        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
        allowfullscreen
      ></iframe>
    </div>
  </div>
{/if}
