<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { page } from "$app/state";

  import { getContinueWatching, onAuthChanged } from "$lib/api/auth";
  import { resumeIndex } from "$lib/resume";
  import { displayTitle, type ContinueWatchingItem } from "$lib/types";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  /**
   * The most recently watched work, or `null` when there is nothing to resume.
   *
   * Only the newest is kept: the disc is a single "get back to it" control, and
   * the Continue Watching row and My List already cover everything else.
   */
  let item = $state<ContinueWatchingItem | null>(null);
  let loading = $state(true);
  let unlisten: UnlistenFn | null = null;

  /**
   * Whether the disc should show at all.
   *
   * Hidden on `/watch/*`: resuming the thing already on screen is nonsense, and
   * the disc would sit over the player. Also hidden while the answer is unknown
   * so a signed-out reader never sees it appear and vanish.
   */
  const visible = $derived(!loading && item !== null && !onWatchPage(page.url.pathname));

  function onWatchPage(pathname: string): boolean {
    return pathname.startsWith("/watch/");
  }

  const title = $derived(item ? (displayTitle(item.anime.title) ?? "Untitled") : "");

  async function load() {
    loading = true;
    try {
      // One entry: the newest, since `continue_watching` sorts by updatedAt desc.
      const items = await getContinueWatching(1);
      item = items[0] ?? null;
    } catch {
      // A failed read just means no disc; it is an affordance, not a page.
      item = null;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    let cancelled = false;

    load();

    // Sign-in can complete while any page is open, so the disc appears without
    // a reload once there is something to resume. Sign-out clears it.
    onAuthChanged((signedIn) => {
      if (signedIn) void load();
      else item = null;
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Without the listener the disc still works on the next navigation.
      });

    return () => {
      cancelled = true;
    };
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

{#if visible && item}
  <!-- `group` drives every hover state from one place. The whole thing is a
       link, so the panel is aria-hidden: the accessible name already says the
       work and the episode. -->
  <a
    href={`/watch/${item.anime.id}?ep=${resumeIndex(item.progress)}`}
    data-testid="resume-disc"
    aria-label={`Resume ${title}${item.progress > 0 ? ` at episode ${item.progress}` : ""}`}
    class="group fixed right-6 bottom-6 z-40 flex items-center justify-end focus:outline-none"
  >
    <!-- The now-playing card. Slides out to the left on hover, and is not
         focusable -- it is a label for the disc, not a second control. -->
    <div
      aria-hidden="true"
      data-testid="resume-disc-card"
      class="pointer-events-none mr-3 w-56 origin-right scale-95 opacity-0 transition-all duration-200 group-hover:scale-100 group-hover:opacity-100 group-focus-visible:scale-100 group-focus-visible:opacity-100 motion-reduce:transition-none"
    >
      <div class="rounded-xl border border-border-subtle bg-surface-raised p-3 shadow-xl">
        <p class="text-[10px] font-medium tracking-wide text-ink-faint uppercase">
          Now playing
        </p>
        <p class="mt-1 line-clamp-2 text-sm font-semibold text-ink">{title}</p>
        <p class="mt-0.5 text-xs text-ink-muted">
          {item.progress > 0 ? `Episode ${item.progress}` : "Not started"}
        </p>
        <p class="mt-2 text-xs text-accent">Resume ›</p>
      </div>
    </div>

    <!-- The record. Cover art fills it; a ring and a centre hole sit on top so
         it still reads as vinyl rather than a round thumbnail. -->
    <span
      class="relative block h-14 w-14 shrink-0 overflow-hidden rounded-full shadow-lg ring-1 ring-white/15 transition-transform duration-200 group-hover:scale-110 group-focus-visible:scale-110 motion-reduce:transition-none"
    >
      {#if item.anime.coverImage}
        <img
          src={item.anime.coverImage}
          alt=""
          class="h-full w-full animate-spin-record rounded-full object-cover motion-reduce:animate-none group-hover:animate-spin-record-fast group-focus-visible:animate-spin-record-fast"
        />
      {:else}
        <!-- No cover: a plain disc is better than a broken image. -->
        <span
          class="block h-full w-full animate-spin-record rounded-full bg-surface-hover motion-reduce:animate-none group-hover:animate-spin-record-fast"
        ></span>
      {/if}

      <!-- Vinyl sheen + centre hole, over the art and not spinning with it. -->
      <span
        aria-hidden="true"
        class="pointer-events-none absolute inset-0 rounded-full"
        style="background: radial-gradient(circle, transparent 30%, rgba(0,0,0,0.35) 100%);"
      ></span>
      <span
        aria-hidden="true"
        class="pointer-events-none absolute top-1/2 left-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full bg-surface ring-2 ring-white/20"
      ></span>
    </span>
  </a>
{/if}