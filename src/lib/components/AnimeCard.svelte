<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { createHoverPreview } from "$lib/hover-preview.svelte";
  import HoverPreview from "./HoverPreview.svelte";

  let { anime }: { anime: Anime } = $props();

  const title = $derived(displayTitle(anime.title) ?? "Untitled");

  let poster = $state<HTMLElement | null>(null);
  const hover = createHoverPreview(() => poster);
</script>

<a
  href={`/anime/${anime.id}`}
  aria-label={title}
  class="group block w-40 shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  onfocusin={hover.show}
  onfocusout={hover.scheduleClose}
>
  <!-- The pointer trigger lives on the artwork, not the whole link, so
       hovering the title below does not open the preview. Keyboard focus stays
       on the anchor, which is what makes the preview reachable without a
       mouse. -->
  <!-- `role="presentation"` because this wrapper is purely decorative: the
       link above carries the accessible name, and the cover inside is alt="".
       It has a role only because a pointer handler requires one. -->
  <div
    bind:this={poster}
    role="presentation"
    data-testid="poster"
    class="relative aspect-[2/3] overflow-hidden rounded-lg bg-surface-hover"
    onmouseenter={hover.show}
    onmouseleave={hover.scheduleClose}
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

{#if hover.open}
  <!-- Sibling of the anchor, not a child: nesting links inside a link is
       invalid, and the preview has its own genre links. -->
  <HoverPreview
    {anime}
    placement={hover.placement}
    measure={hover.measure}
    onenter={hover.cancelClose}
    onleave={hover.scheduleClose}
  />
{/if}