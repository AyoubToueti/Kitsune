<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { page } from "$app/state";

  import { getAnime } from "$lib/api/anime";
  import {
    getContinueWatching,
    getLastPlayed,
    onAuthChanged,
    onListChanged,
  } from "$lib/api/auth";
  import { resumeIndex } from "$lib/resume";
  import { isActive } from "$lib/now-playing.svelte";
  import { displayTitle, type Anime } from "$lib/types";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  let item = $state<{ anime: Anime; episode?: number } | null>(null);
  let loading = $state(true);
  let unlisten: UnlistenFn | null = null;
  let unlistenList: UnlistenFn | null = null;

  // Rotation Animation State
  let isHovered = $state(false);
  let rotation = $state(0);
  let animationFrameId: number;
  let lastTimestamp: number | null = null;

  // Base speed: deg/ms (~30 deg/s normal, ~140 deg/s on hover)
  let currentSpeed = 0.03;

  const visible = $derived(
    !loading &&
      item !== null &&
      !onWatchPage(page.url.pathname) &&
      !onOwnDetailPage(page.url.pathname, item.anime.id) &&
      !isActive(),
  );

  function onWatchPage(pathname: string): boolean {
    return pathname.startsWith("/watch/");
  }

  function onOwnDetailPage(pathname: string, animeId: number): boolean {
    const match = /^\/anime\/(\d+)/.exec(pathname);
    return match !== null && Number(match[1]) === animeId;
  }

  const title = $derived(item ? (displayTitle(item.anime.title) ?? "Untitled") : "");

  /**
   * Smoothly animates rotation frame-by-frame.
   * Interpolates currentSpeed towards targetSpeed without ever resetting the angle.
   */
  function animateRotation(timestamp: number) {
    if (lastTimestamp !== null) {
      const delta = timestamp - lastTimestamp;
      const targetSpeed = isHovered ? 0.16 : 0.03; // Faster when hovered

      // Smoothly ease current speed towards target speed (lerp)
      currentSpeed += (targetSpeed - currentSpeed) * 0.05;

      // Accumulate total angle continuously
      rotation = (rotation + currentSpeed * delta) % 360;
    }
    lastTimestamp = timestamp;
    animationFrameId = requestAnimationFrame(animateRotation);
  }

  async function resolveTarget(): Promise<{ anime: Anime; episode?: number } | null> {
    const record = await getLastPlayed().catch(() => null);
    if (record) {
      const anime = await getAnime(record.animeId).catch(() => null);
      if (anime) return { anime, episode: record.episode };
    }

    const items = await getContinueWatching(1);
    const first = items[0];
    if (!first) return null;

    return {
      anime: first.anime,
      episode: first.progress > 0 ? first.progress : undefined,
    };
  }

  async function load() {
    loading = true;
    try {
      item = await resolveTarget();
    } catch {
      item = null;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    let cancelled = false;

    load();

    // Start continuous smooth rotation loop
    animationFrameId = requestAnimationFrame(animateRotation);

    onAuthChanged((signedIn) => {
      if (signedIn) void load();
      else item = null;
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    onListChanged(() => {
      void load();
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlistenList = fn;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  });

  onDestroy(() => {
    unlisten?.();
    unlistenList?.();
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
  });
</script>

{#if visible && item}
  <div
    class="fixed right-6 bottom-6 z-40 flex items-center justify-end"
  >
    <!-- Floating Info Card (Uses reactive class toggles driven ONLY by disc interaction) -->
    <div
      aria-hidden="true"
      data-testid="resume-disc-card"
      class="pointer-events-none mr-3 w-60 origin-right transition-all duration-300 ease-out motion-reduce:transition-none
        {isHovered ? 'translate-x-0 scale-100 opacity-100' : 'translate-x-2 scale-95 opacity-0'}"
    >
      <div
        class="rounded-2xl border border-border-subtle/80 bg-surface-raised/95 p-3.5 shadow-2xl backdrop-blur-md"
      >
        <div class="flex items-center justify-between gap-2">
          <span class="flex items-center gap-1.5 text-[10px] font-bold tracking-wider text-accent uppercase">
            <span class="relative flex h-2 w-2">
              <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent opacity-75"></span>
              <span class="relative inline-flex h-2 w-2 rounded-full bg-accent"></span>
            </span>
            Continue Watching
          </span>
        </div>

        <p class="mt-1.5 line-clamp-1 text-xs font-bold text-ink">{title}</p>
        <p class="mt-0.5 text-[11px] font-medium text-ink-muted">
          {item.episode ? `Episode ${item.episode}` : "Not started"}
        </p>

        <!-- CTA Action Row -->
        <div class="mt-2.5 flex items-center justify-between border-t border-border-subtle/50 pt-2 text-xs font-semibold text-accent">
          <span>Play now</span>
          <svg 
            class="h-3.5 w-3.5 transition-transform {isHovered ? 'translate-x-0.5' : ''}" 
            viewBox="0 0 24 24" 
            fill="currentColor"
          >
            <path d="M8 5v14l11-7z" />
          </svg>
        </div>
      </div>
    </div>

    <!-- Vinyl Disc (The exclusive pointer event target) -->
    <a
      href={`/anime/${item.anime.id}?ep=${resumeIndex(item.episode ?? 0)}`}
      data-testid="resume-disc"
      aria-label={`Resume ${title}${item.episode ? ` at episode ${item.episode}` : ""}`}
      onmouseenter={() => (isHovered = true)}
      onmouseleave={() => (isHovered = false)}
      onfocusin={() => (isHovered = true)}
      onfocusout={() => (isHovered = false)}
      class="relative flex h-14 w-14 shrink-0 items-center justify-center rounded-full bg-neutral-900 shadow-2xl ring-2 ring-white/10 transition-all duration-300 focus:outline-none {isHovered ? 'scale-110 ring-accent/50' : ''}"
    >
      <!-- Spin Outer Ring & Cover Art -->
      <span class="relative h-full w-full overflow-hidden rounded-full">
        {#if item.anime.coverImage}
          <img
            src={item.anime.coverImage}
            alt=""
            style="transform: rotate({rotation}deg);"
            class="h-full w-full object-cover transition-opacity {isHovered ? 'opacity-100' : 'opacity-85'}"
          />
        {:else}
          <span
            style="transform: rotate({rotation}deg);"
            class="block h-full w-full bg-neutral-800"
          ></span>
        {/if}

        <!-- Concentric Vinyl Grooves Effect -->
        <span
          aria-hidden="true"
          class="pointer-events-none absolute inset-0 rounded-full border border-white/5 bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-transparent via-black/20 to-black/60"
        ></span>
      </span>

      <!-- Gloss Reflection Overlay -->
      <span
        aria-hidden="true"
        class="pointer-events-none absolute inset-0 rounded-full bg-gradient-to-tr from-transparent via-white/15 to-transparent"
      ></span>

      <!-- Vinyl Center Hole & Label Hub -->
      <span
        aria-hidden="true"
        class="pointer-events-none absolute top-1/2 left-1/2 flex h-4 w-4 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-accent ring-2 ring-neutral-950/80 shadow-inner"
      >
        <span class="h-1.5 w-1.5 rounded-full bg-neutral-950"></span>
      </span>
    </a>
  </div>
{/if}
