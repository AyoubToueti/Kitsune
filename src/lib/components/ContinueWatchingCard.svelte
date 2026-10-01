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
  class="group relative flex flex-col overflow-hidden rounded-xl border border-border-subtle bg-surface-raised shadow-md transition-all duration-300 hover:-translate-y-1 hover:border-accent/60 hover:shadow-xl"
>
  <!-- Artwork Container -->
  <div class="relative aspect-[2/3] w-full overflow-hidden bg-surface-hover">
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
          class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105 group-hover:brightness-90"
        />
      {:else}
        <div
          class="flex h-full w-full items-center justify-center text-xs text-ink-faint"
        >
          No cover
        </div>
      {/if}
    </a>

    <!-- Score Tag -->
    {#if entry.anime.averageScore != null}
      <span
        class="absolute top-2 right-2 rounded-md border border-border-subtle/50 bg-surface/80 px-1.5 py-0.5 text-[11px] font-bold text-score backdrop-blur-md shadow-sm"
      >
        ★ {entry.anime.averageScore}
      </span>
    {/if}

    <!-- Info Preview Trigger -->
    <button
      bind:this={trigger}
      type="button"
      data-testid="info-trigger"
      aria-label={`Preview ${title}`}
      class="absolute top-2 left-2 flex h-6 w-6 items-center justify-center rounded-full border border-border-subtle/50 bg-surface/80 text-xs font-bold text-ink backdrop-blur-md shadow-sm transition-colors hover:bg-accent hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      onmouseenter={hover.show}
      onmouseleave={hover.scheduleClose}
      onfocusin={hover.show}
      onfocusout={hover.scheduleClose}
    >
      i
    </button>

    <!-- Progress Indicator Bar at Bottom of Artwork -->
    {#if percent != null}
      <div class="absolute inset-x-0 bottom-0 h-1 bg-surface-hover/80">
        <div
          class="h-full bg-accent transition-all duration-300"
          style={`width: ${percent}%`}
        ></div>
      </div>
    {/if}

    <!-- Hover Quick Bar -->
    <div
      class="absolute inset-x-0 bottom-0 z-10 flex translate-y-full items-center gap-1.5 bg-gradient-to-t from-black/90 via-black/70 to-transparent p-2.5 pt-6 transition-transform duration-200 group-hover:translate-y-0 focus-within:translate-y-0"
    >
      <a
        href={`/anime/${entry.anime.id}?ep=${resumeIndex(entry.progress)}`}
        data-testid="resume"
        class="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-accent py-1.5 text-xs font-semibold text-white shadow-md transition-all hover:bg-accent-hover hover:shadow-accent/30 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <svg class="h-3.5 w-3.5 fill-current" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M8 5v14l11-7z" />
        </svg>
        <span>Ep. {episode}</span>
      </a>

      <button
        type="button"
        data-testid="skip-episode"
        title="Mark next episode as watched"
        aria-label="Skip to next episode"
        disabled={skipping}
        onclick={skip}
        class="flex items-center justify-center rounded-lg border border-white/10 bg-white/15 px-2.5 py-1.5 text-xs font-bold text-white backdrop-blur-sm transition-all hover:bg-white/30 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50"
      >
        +1
      </button>
    </div>
  </div>

  <!-- Content details -->
  <div class="flex flex-1 flex-col justify-between p-3">
    <h4
      class="line-clamp-1 text-xs font-semibold leading-snug text-ink transition-colors group-hover:text-accent"
      title={title}
    >
      {title}
    </h4>
    <div class="mt-2 flex items-center justify-between text-[10px] text-ink-muted">
      <span>{percent != null ? `${percent}% completed` : "In progress"}</span>
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