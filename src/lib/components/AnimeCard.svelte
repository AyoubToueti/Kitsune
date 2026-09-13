<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { positionPreview, PREVIEW_SIZE } from "$lib/hover";
  import HoverPreview from "./HoverPreview.svelte";

  let { anime }: { anime: Anime } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");

  /**
   * How long the preview survives after the pointer leaves.
   *
   * The preview sits outside the card, so travelling from one to the other
   * crosses a gap. Without a delay the preview would vanish mid-journey.
   */
  const CLOSE_DELAY_MS = 120;

  let poster = $state<HTMLElement | null>(null);
  let open = $state(false);
  let position = $state({ x: 0, y: 0 });

  let closeTimer: ReturnType<typeof setTimeout> | null = null;

  function cancelClose() {
    if (closeTimer !== null) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
  }

  /** Show, anchored to the poster. */
  function show() {
    cancelClose();
    if (!poster) return;

    position = positionPreview(
      poster.getBoundingClientRect(),
      PREVIEW_SIZE,
      { width: window.innerWidth, height: window.innerHeight },
    );
    open = true;
  }

  /** Hide, after a grace period so the pointer can reach the preview. */
  function scheduleClose() {
    cancelClose();
    closeTimer = setTimeout(() => {
      open = false;
      closeTimer = null;
    }, CLOSE_DELAY_MS);
  }

  // A pending timer must not outlive the component.
  $effect(() => cancelClose);
</script>

<a
  href={`/anime/${anime.id}`}
  aria-label={title}
  class="group block w-40 shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  onmouseenter={show}
  onmouseleave={scheduleClose}
  onfocusin={show}
  onfocusout={scheduleClose}
>
  <div
    bind:this={poster}
    class="relative aspect-[2/3] overflow-hidden rounded-lg bg-surface-hover"
  >
    {#if anime.coverImage}
      <img
        src={anime.coverImage}
        alt=""
        loading="lazy"
        class="h-full w-full object-cover transition-transform duration-200 group-hover:scale-105 group-hover:blur-[2px]"
      />
    {:else}
      <div class="flex h-full w-full items-center justify-center text-xs text-ink-faint">
        No cover
      </div>
    {/if}

    <!-- Play affordance, matching the reference's hover state. Decorative: the
         link already carries the accessible name. -->
    <span
      aria-hidden="true"
      data-testid="play-overlay"
      class="pointer-events-none absolute inset-0 flex items-center justify-center text-3xl text-white opacity-0 transition-opacity duration-200 group-hover:opacity-100 group-focus-visible:opacity-100"
    >
      ▶
    </span>

    {#if anime.averageScore != null}
      <span
        class="absolute top-2 right-2 rounded-full bg-surface/85 px-2 py-0.5 text-xs font-semibold text-score"
      >
        {anime.averageScore}
      </span>
    {/if}
  </div>

  <p class="mt-2 line-clamp-2 text-sm font-medium text-ink group-hover:text-accent">
    {title}
  </p>
</a>

{#if open}
  <!-- Outside the anchor so the preview's own links are not nested inside it. -->
  <HoverPreview
    {anime}
    x={position.x}
    y={position.y}
    onenter={cancelClose}
    onleave={scheduleClose}
  />
{/if}