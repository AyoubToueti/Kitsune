<script lang="ts">
  import { onMount, untrack } from "svelte";

  import { errorMessage, getSchedule } from "$lib/api/anime";
  import { buildDayStrip, formatTime } from "$lib/schedule";
  import {
    breakpointCardWidth,
    canScrollLeft,
    canScrollRight,
  } from "$lib/scroll-strip";
  import { createStripDrag } from "$lib/strip-drag.svelte";
  import { displayTitle, type ScheduledEpisode } from "$lib/types";
  import Skeleton from "./Skeleton.svelte";

  /** The strip's `gap-2`, in pixels. Used to divide the width into cards. */
  const GAP_PX = 8;

  /** The narrowest a day tab may be, matching the old `min-w-18`. */
  const MIN_CARD_WIDTH = 72;

  /**
   * Per-day cap.
   *
   * AniList rejects `perPage` above 50, so one day is the largest window that
   * can be fetched whole. A busier day is truncated rather than silently
   * mis-ordered.
   */
  const DAY_LIMIT = 50;

  // Pure date arithmetic, so every tab renders immediately and nothing is
  // fetched until a day is actually shown.
  const window_ = buildDayStrip();

  let selectedKey = $state<string>("");
  let cache = $state<Record<string, ScheduledEpisode[]>>({});
  /**
   * Which day a request is in flight for, if any.
   *
   * Per-key rather than a boolean, so switching days cannot leave a stale
   * "loading" showing for the newly selected one.
   */
  let loadingKey = $state<string | null>(null);
  let error = $state<string | null>(null);
  let strip = $state<HTMLElement | null>(null);

  // Drag-to-scroll. The strip hides its scrollbar, so dragging is the main way
  // to move it by hand; the arrows beside it are the precise way. The stride
  // getter is a closure, so it may read `cardWidth` before that is assigned --
  // it is only called once a gesture ends.
  const drag = createStripDrag(() => strip, {
    getStride: () => (cardWidth ?? MIN_CARD_WIDTH) + GAP_PX,
  });

  /**
   * Whether an arrow has anywhere to go.
   *
   * `false` until the strip is measured on mount, so neither arrow flashes as
   * enabled on a strip that has not scrolled yet. Kept as plain state rather
   * than derived because `scrollLeft` and `clientWidth` are not reactive.
   */
  let atStart = $state(true);
  let atEnd = $state(true);

  /**
   * The exact width each day tab is given.
   *
   * Set to fill the strip with a whole number of tabs, so no half tab is ever
   * visible. `null` until measured -- the fallback `MIN_CARD_WIDTH` keeps the
   * tabs rendering before layout (SSR, jsdom).
   */
  let cardWidth = $state<number | null>(null);

  /**
   * Re-read the strip's geometry: resize the tabs, then update both arrows.
   *
   * Called on mount, on resize, and after every scroll and arrow press. Writes
   * only from the element's own measurements, never scrolls, so the three
   * callers cannot fight each other.
   */
  function syncArrows() {
    if (!strip) return;

    cardWidth = breakpointCardWidth(strip.clientWidth, MIN_CARD_WIDTH, GAP_PX);

    atStart = !canScrollLeft(strip.scrollLeft);
    atEnd = !canScrollRight(strip.scrollLeft, strip.clientWidth, strip.scrollWidth);
  }

  /** Move the strip by whole tabs, as an arrow press would. */
  function scrollStep(direction: 1 | -1) {
    if (!strip) return;
    // One tab plus the gap, so whole tabs land at the edges. Routed through the
    // drag composable's glide, so a press eases exactly like a drag release.
    const stride = (cardWidth ?? MIN_CARD_WIDTH) + GAP_PX;
    const maxScroll = strip.scrollWidth - strip.clientWidth;
    const target = Math.min(
      Math.max(strip.scrollLeft + stride * direction, 0),
      Math.max(0, maxScroll),
    );
    drag.glideTo(target);
  }

  const loading = $derived(loadingKey === selectedKey);

  const selected = $derived(
    window_.days.find((day) => day.key === selectedKey) ?? null,
  );

  /** `null` until the selected day has been fetched. */
  const entries = $derived(
    selectedKey === "" ? null : (cache[selectedKey] ?? null),
  );

  function title(entry: ScheduledEpisode): string {
    return displayTitle(entry.anime.title) ?? "Untitled";
  }

  // Defaults to today, set on mount so the full strip renders first.
  onMount(() => {
    selectedKey = window_.days[window_.todayIndex].key;

    // The strip is bound by now, so its geometry can be read. `resize` covers
    // the arrows going stale when the window -- and so the visible width --
    // changes; `scroll` covers a wheel or touch scroll the arrows did not
    // cause. Both only re-read, never scroll, so they cannot fight each other.
    syncArrows();
    window.addEventListener("resize", syncArrows);

    return () => window.removeEventListener("resize", syncArrows);
  });

  $effect(() => {
    const day = selected;

    if (!day) return;

    // Read the cache WITHOUT tracking it. Tracking would make filling the cache
    // re-run this effect, whose cleanup sets `cancelled` and so swallows the
    // in-flight request's own `loadingKey = null` -- leaving the widget stuck
    // on "Loading…" forever. Untracked, `cancelled` means exactly one thing:
    // the user moved to another day.
    if (untrack(() => cache[day.key]) !== undefined) return;

    let cancelled = false;
    loadingKey = day.key;
    error = null;

    getSchedule(day.from, day.to, DAY_LIMIT)
      .then((found) => {
        if (cancelled) return;
        // A fresh object: mutating the existing one would not be observed by
        // the derived above.
        cache = { ...cache, [day.key]: found };
      })
      .catch((err) => {
        if (cancelled) return;
        error = errorMessage(err);
      })
      .finally(() => {
        if (cancelled) return;
        loadingKey = null;
      });

    return () => {
      cancelled = true;
    };
  });

  // Keep the active tab visible. Today sits seven tabs in, so without this it
  // would often start off-screen.
  $effect(() => {
    const el = strip;
    const key = selectedKey;
    if (!el || !key) return;

    const active = el.querySelector<HTMLElement>('[aria-pressed="true"]');
    // Guarded: jsdom does not implement scrollIntoView.
    if (active && typeof active.scrollIntoView === "function") {
      active.scrollIntoView({ block: "nearest", inline: "center" });
    }
  });
</script>

<section class="mt-8">
  <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
    Estimated schedule
  </h2>

  <!-- Wrapper: the arrows sit over the strip's edges, which needs a positioned
       ancestor. -->
  <div class="relative mb-3">
    <!-- Every day in range gets a tab whether or not anything airs, so the
         strip keeps a stable width instead of shifting as responses arrive. -->
    <!-- `no-scrollbar`: the arrows and dragging are the intended ways to move
         it, and a scrollbar under a row of day tabs is noise. `role` exists
         only because the pointer handler requires one; keyboard users get the
         same scrolling from the arrow buttons below. -->
    <div
      bind:this={strip}
      data-testid="day-strip"
      role="presentation"
      class="no-scrollbar snap-strip flex gap-2 overflow-x-auto pb-1 {drag.dragging
        ? 'strip-dragging select-none cursor-grabbing'
        : drag.settling
          ? 'strip-settling'
          : ''}"
      onpointerdown={drag.onpointerdown}
      onclickcapture={drag.onclickcapture}
      ondragstart={drag.ondragstart}
      onscroll={syncArrows}
    >
      {#each window_.days as day (day.key)}
        <button
          type="button"
          onclick={() => (selectedKey = day.key)}
          aria-pressed={day.key === selectedKey}
          style="width: {cardWidth ?? MIN_CARD_WIDTH}px;"
          class="flex shrink-0 cursor-pointer flex-col items-center overflow-hidden rounded-lg px-1 py-1.5 text-xs transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {day.key ===
          selectedKey
            ? 'bg-accent text-white'
            : 'bg-surface-hover text-ink-muted hover:text-ink'}"
        >
          <span class="font-medium">{day.weekday}</span>
          <span class="text-[10px] opacity-80">{day.shortDate}</span>
        </button>
      {/each}
    </div>

    <!-- Arrows over the strip's edges. Disabled at each end, so they read as
         "nothing further this way" rather than silently doing nothing. -->
    <button
      type="button"
      data-testid="schedule-prev"
      aria-label="Scroll to earlier days"
      disabled={atStart}
      onclick={() => scrollStep(-1)}
      class="absolute top-1/2 left-0 flex h-8 w-8 -translate-y-1/2 items-center justify-center rounded-full border border-border-subtle bg-surface-raised text-lg text-ink shadow-lg transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-default disabled:opacity-30"
    >
      <span aria-hidden="true">‹</span>
    </button>

    <button
      type="button"
      data-testid="schedule-next"
      aria-label="Scroll to later days"
      disabled={atEnd}
      onclick={() => scrollStep(1)}
      class="absolute top-1/2 right-0 flex h-8 w-8 -translate-y-1/2 items-center justify-center rounded-full border border-border-subtle bg-surface-raised text-lg text-ink shadow-lg transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-default disabled:opacity-30"
    >
      <span aria-hidden="true">›</span>
    </button>
  </div>

  <!-- Error is checked before the not-yet-loaded branch, because a failed day
       leaves `entries` null and would otherwise read as still loading. -->
  {#if error}
    <p class="py-4 text-sm text-ink-faint">
      Could not load that day. <span class="text-ink-muted">{error}</span>
    </p>
  {:else if loading || entries === null}
    <!-- Mirrors the entry rows: a time bar, a title line, and an episode chip. -->
    <div
      role="status"
      aria-busy="true"
      class="divide-y divide-border-subtle"
      data-testid="schedule-skeleton"
    >
      <span class="sr-only">Loading…</span>
      {#each Array(10) as _, i (i)}
        <div class="flex items-center gap-3 py-3">
          <Skeleton class="h-7 w-12 shrink-0" />
          <Skeleton class="h-5 min-w-0 flex-1" />
          <Skeleton class="h-5 w-20 shrink-0 rounded" />
        </div>
      {/each}
    </div>
  {:else if entries.length === 0}
    <p class="py-4 text-sm text-ink-faint">Nothing scheduled.</p>
  {:else}
    <ul class="divide-y divide-border-subtle">
      {#each entries as entry (`${entry.anime.id}-${entry.airingAt}`)}
        <li>
          <a
            href={`/anime/${entry.anime.id}`}
            class="flex items-center gap-3 py-2 transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <time
              datetime={new Date(entry.airingAt * 1000).toISOString()}
              class="w-12 shrink-0 text-xs tabular-nums text-ink-faint"
            >
              {formatTime(entry.airingAt)}
            </time>

            <span class="min-w-0 flex-1 truncate text-sm text-ink">
              {title(entry)}
            </span>

            {#if entry.episode != null}
              <span class="shrink-0 rounded bg-surface-hover px-2 py-0.5 text-xs text-ink-muted">
                Episode {entry.episode}
              </span>
            {/if}
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>