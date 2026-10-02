<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { createHoverPreview } from "$lib/hover-preview.svelte";
  import { resumeIndex } from "$lib/resume";
  import { isPlayableTrailer, trailerEmbedUrl } from "$lib/trailer";
  import { progressFor } from "$lib/watch-progress.svelte";
  import HoverPreview from "./HoverPreview.svelte";
  import Modal from "./Modal.svelte";

  let { anime, fluid = false }: { anime: Anime; fluid?: boolean } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");

  /**
   * How far the reader got, in episodes watched.
   *
   * Reading this starts the shared progress store on first use, so every card
   * on a page shares one read of the list.
   */
  const watched = $derived(progressFor(anime.id));

  /**
   * The episode the Watch button opens.
   *
   * Resume when there is progress, otherwise the first episode. The index is
   * 0-based (`?ep=`), matching the watch page.
   */
  const targetEpisode = $derived(watched > 0 ? resumeIndex(watched) : 0);

  /** The embed URL when the work has a playable trailer, else null. */
  const trailerUrl = $derived(
    isPlayableTrailer(anime.trailer) ? trailerEmbedUrl(anime.trailer) : null,
  );

  /** Whether the preview anchors to the info trigger button. */
  let trigger = $state<HTMLElement | null>(null);
  const hover = createHoverPreview(() => trigger);

  /** Whether the trailer modal is open. */
  let trailerOpen = $state(false);
</script>

<div
  data-testid="anime-card"
  class="group relative flex flex-col overflow-hidden rounded-2xl border border-border-subtle/80 bg-surface-raised/90 shadow-sm transition-all duration-300 ease-out hover:-translate-y-1.5 hover:border-accent/50 hover:shadow-xl hover:shadow-accent/5 {fluid
    ? 'w-full min-w-0'
    : 'w-44 shrink-0'}"
>
  <!-- Artwork Container -->
  <div
    data-testid="poster"
    class="relative aspect-[2/3] w-full overflow-hidden bg-surface-hover"
  >
    <a
      href={`/anime/${anime.id}`}
      aria-label={title}
      draggable="false"
      class="block h-full w-full focus:outline-none"
    >
      {#if anime.coverImage}
        <img
          src={anime.coverImage}
          alt=""
          loading="lazy"
          draggable="false"
          aria-hidden="true"
          class="h-full w-full object-cover transition-transform duration-500 ease-out group-hover:scale-108 group-hover:brightness-90"
        />
      {:else}
        <div
          class="flex h-full w-full items-center justify-center text-xs font-medium text-ink-faint"
        >
          No Cover
        </div>
      {/if}
    </a>

    <!-- Top Metadata Badges -->
    <div
      class="pointer-events-none absolute top-2 inset-x-2 flex items-center justify-between gap-1 z-10"
    >
      {#if anime.format}
        <span
          class="rounded-md border border-white/10 bg-black/40 px-1.5 py-0.5 text-[10px] font-bold text-white uppercase backdrop-blur-md shadow-sm"
        >
          {anime.format}
        </span>
      {:else}
        <span></span>
      {/if}

      {#if anime.averageScore != null}
        <span
          class="flex items-center gap-0.5 rounded-md border border-border-subtle/40 bg-surface/85 px-1.5 py-0.5 text-[10px] font-extrabold text-score backdrop-blur-md shadow-sm"
        >
          <span class="text-amber-400">★</span>
          {anime.averageScore}
        </span>
      {/if}
    </div>

    <!-- Floating Quick Bar (Slides & Fades in cleanly) -->
    <div
      class="absolute inset-x-2 bottom-2 z-20 flex translate-y-3 opacity-0 items-center gap-1.5 rounded-xl border border-white/10 bg-black/60 p-1.5 backdrop-blur-md shadow-lg transition-all duration-250 cubic-bezier(0.16,1,0.3,1) group-hover:translate-y-0 group-hover:opacity-100 has-focus-visible:translate-y-0 has-focus-visible:opacity-100"
    >
      <!-- Quick Watch Action. Resumes the last episode when there is progress,
           otherwise starts from the first. -->
      <a
        href={`/anime/${anime.id}?ep=${targetEpisode}`}
        data-testid="watch-button"
        class="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-accent py-1.5 text-xs font-bold text-white shadow-sm transition-all hover:bg-accent-hover hover:shadow-accent/40 active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <svg
          class="h-3.5 w-3.5 fill-current"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path d="M8 5v14l11-7z" />
        </svg>
        <span>{watched > 0 ? "Resume" : "Watch"}</span>
      </a>

      <!-- Trailer, only when the work has a playable one. -->
      {#if trailerUrl}
        <button
          type="button"
          data-testid="trailer-trigger"
          aria-label={`Play trailer for ${title}`}
          onclick={() => (trailerOpen = true)}
          class="flex h-7 w-7 items-center justify-center rounded-lg border border-white/10 bg-white/10 text-white transition-all hover:bg-white/25 active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent cursor-pointer"
        >
          <svg
            class="h-3.5 w-3.5 stroke-current fill-none stroke-2"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <polygon points="10 8 16 12 10 16 10 8" />
          </svg>
        </button>
      {/if}

      <!-- Info Trigger -->
      <button
        bind:this={trigger}
        type="button"
        data-testid="info-trigger"
        aria-label={`Preview ${title}`}
        class="flex h-7 w-7 items-center justify-center rounded-lg border border-white/10 bg-white/10 text-xs font-bold text-white transition-all hover:bg-white/25 active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        onmouseenter={hover.show}
        onmouseleave={hover.scheduleClose}
        onfocusin={hover.show}
        onfocusout={hover.scheduleClose}
      >
        <svg
          class="h-3.5 w-3.5 stroke-current fill-none stroke-2"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <circle cx="12" cy="12" r="9" />
          <path stroke-linecap="round" d="M12 8h.01M12 11v5" />
        </svg>
      </button>
    </div>
  </div>

  <!-- Title & Extra Details -->
  <div class="flex flex-1 flex-col justify-between p-3">
    <p
      class="line-clamp-2 text-xs font-bold leading-tight text-ink transition-colors group-hover:text-accent"
      {title}
    >
      {title}
    </p>

    {#if anime.episodeCount}
      <p class="mt-1 text-[11px] font-medium text-ink-faint">
        {anime.episodeCount}
        {anime.episodeCount === 1 ? "ep" : "eps"}
      </p>
    {/if}
  </div>
</div>

{#if hover.open}
  <HoverPreview
    {anime}
    placement={hover.placement}
    measure={hover.measure}
    onenter={hover.enterPreview}
    onleave={hover.leavePreview}
  />
{/if}

{#if trailerOpen && trailerUrl}
  <Modal
    open={trailerOpen}
    onClose={() => (trailerOpen = false)}
    label={`${title} trailer`}
  >
    <div class="aspect-video w-full overflow-hidden rounded-lg bg-black">
      <iframe
        src={trailerUrl}
        title={`${title} trailer`}
        class="h-full w-full border-0"
        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
        allowfullscreen
      ></iframe>
    </div>
  </Modal>
{/if}
