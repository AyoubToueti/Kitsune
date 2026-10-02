<script lang="ts">
  import { setListEntry } from "$lib/api/auth";
  import { createHoverPreview } from "$lib/hover-preview.svelte";
  import { resumeIndex } from "$lib/resume";
  import { displayTitle, type ContinueWatchingItem } from "$lib/types";
  import HoverPreview from "./HoverPreview.svelte";

  let { entry }: { entry: ContinueWatchingItem } = $props();

  const title = $derived(displayTitle(entry.anime.title) ?? "Untitled");

  /** The episode to resume: the last one started, or the first if none. */
  const episode = $derived(Math.max(1, entry.progress));

  /** Progress percentage calculation */
  const percent = $derived(
    entry.anime.episodeCount
      ? Math.min(
          100,
          Math.round((entry.progress / entry.anime.episodeCount) * 100),
        )
      : null,
  );

  /** Preview trigger ref and hover controller */
  let trigger = $state<HTMLElement | null>(null);
  const hover = createHoverPreview(() => trigger);

  let skipping = $state(false);

  async function skip() {
    if (skipping) return;
    skipping = true;
    try {
      await setListEntry(entry.anime.id, "current", entry.progress + 1);
    } catch {
      // Ignore error silently
    } finally {
      skipping = false;
    }
  }
</script>

<div
  class="group relative flex flex-col overflow-hidden rounded-2xl border border-border-subtle/80 bg-surface-raised/90 shadow-sm transition-all duration-300 ease-out hover:-translate-y-1.5 hover:border-accent/50 hover:shadow-xl hover:shadow-accent/5"
>
  <!-- Artwork Container -->
  <div
    data-testid="poster"
    class="relative aspect-[2/3] w-full overflow-hidden bg-surface-hover"
  >
    <a
      href={`/anime/${entry.anime.id}?ep=${resumeIndex(entry.progress)}`}
      aria-label={`Resume ${title}`}
      draggable="false"
      class="block h-full w-full focus:outline-none"
    >
      {#if entry.anime.coverImage}
        <img
          src={entry.anime.coverImage}
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

    <!-- Top Score Badge -->
    {#if entry.anime.averageScore != null}
      <div class="pointer-events-none absolute top-2 right-2 z-10">
        <span
          class="flex items-center gap-0.5 rounded-md border border-border-subtle/40 bg-surface/85 px-1.5 py-0.5 text-[10px] font-extrabold text-score backdrop-blur-md shadow-sm"
        >
          <span class="text-amber-400">★</span> {entry.anime.averageScore}
        </span>
      </div>
    {/if}

    <!-- Progress Indicator Bar -->
    {#if percent != null}
      <div class="pointer-events-none absolute inset-x-0 bottom-0 z-10 h-1 bg-black/40">
        <div
          class="h-full bg-accent transition-all duration-300"
          style={`width: ${percent}%`}
        ></div>
      </div>
    {/if}

    <!-- Floating Quick Bar (Slides & Fades in cleanly) -->
    <div
      class="absolute inset-x-2 bottom-2 z-20 flex translate-y-3 opacity-0 items-center gap-1.5 rounded-xl border border-white/10 bg-black/60 p-1.5 backdrop-blur-md shadow-lg transition-all duration-250 cubic-bezier(0.16,1,0.3,1) group-hover:translate-y-0 group-hover:opacity-100 focus-within:translate-y-0 focus-within:opacity-100"
    >
      <!-- Resume / Watch Link -->
      <a
        href={`/anime/${entry.anime.id}?ep=${resumeIndex(entry.progress)}`}
        data-testid="resume"
        class="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-accent py-1.5 text-xs font-bold text-white shadow-sm transition-all hover:bg-accent-hover hover:shadow-accent/40 active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <svg class="h-3.5 w-3.5 fill-current" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M8 5v14l11-7z" />
        </svg>
        <span>Ep. {episode}</span>
      </a>

      <!-- +1 Episode Action -->
      <button
        type="button"
        data-testid="skip-episode"
        title="Mark next episode as watched"
        aria-label="Skip to next episode"
        disabled={skipping}
        onclick={skip}
        class="flex h-7 px-2 items-center justify-center rounded-lg border border-white/10 bg-white/10 text-xs font-bold text-white transition-all hover:bg-white/25 active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50"
      >
        +1
      </button>

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
        <svg class="h-3.5 w-3.5 stroke-current fill-none stroke-2" viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="12" cy="12" r="9" />
          <path stroke-linecap="round" d="M12 8h.01M12 11v5" />
        </svg>
      </button>
    </div>
  </div>

  <!-- Content Details -->
  <div class="flex flex-1 flex-col justify-between p-3">
    <p
      class="line-clamp-2 text-xs font-bold leading-tight text-ink transition-colors group-hover:text-accent"
      title={title}
    >
      {title}
    </p>

    <div class="mt-2 flex items-center justify-between text-[11px] font-medium text-ink-muted">
      <span>{percent != null ? `${percent}%` : "In progress"}</span>
      <span>
        Ep {episode}{entry.anime.episodeCount != null
          ? ` / ${entry.anime.episodeCount}`
          : ""}
      </span>
    </div>
  </div>
</div>

{#if hover.open}
  <HoverPreview
    anime={entry.anime}
    placement={hover.placement}
    measure={hover.measure}
    onenter={hover.enterPreview}
    onleave={hover.leavePreview}
  />
{/if}