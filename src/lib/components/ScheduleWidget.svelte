<script lang="ts">
  import { onMount, untrack } from "svelte";

  import { errorMessage, getSchedule } from "$lib/api/anime";
  import { buildDayStrip, formatTime } from "$lib/schedule";
  import { displayTitle, type ScheduledEpisode } from "$lib/types";
  import Skeleton from "./Skeleton.svelte";

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

  <!-- Every day in range gets a tab whether or not anything airs, so the strip
       keeps a stable width instead of shifting as responses arrive. -->
  <div
    bind:this={strip}
    data-testid="day-strip"
    class="mb-3 flex gap-2 overflow-x-auto pb-1"
  >
    {#each window_.days as day (day.key)}
      <button
        type="button"
        onclick={() => (selectedKey = day.key)}
        aria-pressed={day.key === selectedKey}
        class="flex min-w-18 shrink-0 flex-col items-center rounded-lg px-3 py-1.5 text-xs transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {day.key ===
        selectedKey
          ? 'bg-accent text-white'
          : 'bg-surface-hover text-ink-muted hover:text-ink'}"
      >
        <span class="font-medium">{day.weekday}</span>
        <span class="text-[10px] opacity-80">{day.shortDate}</span>
      </button>
    {/each}
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