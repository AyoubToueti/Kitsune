<script lang="ts">
  import { onMount } from "svelte";

  import { getTrailerEmbedBase } from "$lib/api/anime";
  import { trailerEmbedUrl, trailerWatchUrl } from "$lib/trailer";
  import type { Trailer } from "$lib/types";
  import Modal from "./Modal.svelte";

  let { trailer }: { trailer?: Trailer } = $props();

  // Whether the trailer overlay is open.
  let isOpen = $state(false);

  // The URL builders live in `$lib/trailer` so the card and this page agree.
  const url = $derived(trailer ? trailerWatchUrl(trailer) : null);

  /**
   * The loopback server's base URL, or `null` when it is not running.
   *
   * Read once: the port is fixed for the app's life. Until it resolves, the
   * YouTube embed falls back to the direct URL, which only works in dev.
   */
  let embedBase = $state<string | null>(null);

  onMount(() => {
    getTrailerEmbedBase()
      .then((base) => {
        embedBase = base;
      })
      .catch(() => {
        // Fall back to the direct embed; the trailer may still play in dev.
        embedBase = null;
      });
  });

  /**
   * Where the iframe points.
   *
   * YouTube is served through the loopback page when it is available: its
   * player refuses to configure on a page with no HTTP referer, which the app's
   * `tauri://localhost` origin is, so a direct embed fails with Error 153 in a
   * production build. Other sites embed directly as before.
   */
  const videoEmbedUrl = $derived.by(() => {
    if (!trailer) return null;
    if (trailer.site.toLowerCase() === "youtube" && embedBase !== null) {
      return `${embedBase}/embed?v=${encodeURIComponent(trailer.id)}`;
    }
    return trailerEmbedUrl(trailer);
  });
</script>

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

<!-- Trailer overlay. `Modal` portals to <body>, so this escapes the sidebar's
     sticky stacking context and paints above the Recommended row. -->
<Modal
  open={isOpen && videoEmbedUrl !== null}
  onClose={() => (isOpen = false)}
  label="Trailer"
>
  <div class="aspect-video w-full overflow-hidden bg-black">
    <iframe
      src={videoEmbedUrl ?? ""}
      title="Trailer Player"
      class="h-full w-full border-0"
      allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
      allowfullscreen
    ></iframe>
  </div>
</Modal>