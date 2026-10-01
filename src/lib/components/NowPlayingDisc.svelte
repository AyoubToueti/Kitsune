<script lang="ts">
  import { goto } from "$app/navigation";

  import { displayTitle } from "$lib/types";
  import {
    isActive,
    nowPlaying,
    nowPlayingSession,
    requestOpen,
  } from "$lib/now-playing.svelte";

  /**
   * The app-wide "now playing" widget.
   *
   * A spinning vinyl disc (the episode's cover art) with an outer ring showing
   * how much of the episode is buffered. It floats over every page while
   * something is playing, and clicking it reopens the watch modal for that
   * episode -- which is just a link to `/anime/<id>?ep=<index>`, the same
   * auto-open the resume links use.
   *
   * It reads the shared session, so it keeps reporting after the modal that
   * started playback has closed. It disappears when the session resets, which
   * the player-exit handler does.
   */

  const session = nowPlayingSession();

  /** The work being watched, or `null`. */
  const current = $derived(nowPlaying());
  const title = $derived(
    current ? (displayTitle(current.anime.title) ?? "Untitled") : "",
  );

  /** The buffered fraction of the chosen file, 0..1. */
  const fraction = $derived(session.fileFraction);

  // The ring geometry. A single SVG circle whose visible arc is set by a dash
  // offset: the circumference is the full dash, and the offset hides the part
  // not yet buffered. This is the standard trick for a progress ring and needs
  // no path maths per update.
  const RADIUS = 26;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
  const dashOffset = $derived(CIRCUMFERENCE * (1 - Math.min(Math.max(fraction, 0), 1)));

  /**
   * Reopen the watch modal for the playing episode.
   *
   * Bumps the store's open request AND navigates to the work's page. The
   * request is what actually opens the modal: `?ep=` only fires on a change, so
   * a click from the same page (URL already carrying `?ep=`) would otherwise do
   * nothing. The page consumes the request once it has the episode list.
   */
  function reopen(): void {
    if (!current) return;
    requestOpen();
    void goto(`/anime/${current.anime.id}`);
  }
</script>

{#if isActive() && current}
  <button
    type="button"
    onclick={reopen}
    aria-label={`Now playing ${title}. Open the buffer window.`}
    title={`${title} — ${Math.round(fraction * 100)}% buffered`}
    data-testid="now-playing-disc"
    class="group fixed right-6 bottom-6 z-40 grid size-16 place-items-center rounded-full focus:outline-none"
  >
    <!-- The progress ring, behind the art. Rotated -90deg so the arc starts at
         the top rather than at 3 o'clock. -->
    <svg
      class="absolute inset-0 size-16 -rotate-90"
      viewBox="0 0 64 64"
      aria-hidden="true"
    >
      <circle
        cx="32"
        cy="32"
        r={RADIUS}
        fill="none"
        stroke="var(--color-border-subtle)"
        stroke-width="3"
      />
      <circle
        cx="32"
        cy="32"
        r={RADIUS}
        fill="none"
        stroke="var(--color-accent)"
        stroke-width="3"
        stroke-linecap="round"
        stroke-dasharray={CIRCUMFERENCE}
        stroke-dashoffset={dashOffset}
        data-testid="now-playing-ring"
        style="transition: stroke-dashoffset 400ms ease"
      />
    </svg>

    <!-- The record. Cover art fills it; a ring and a centre hole sit on top so
         it still reads as vinyl rather than a round thumbnail. -->
    <span
      class="relative block h-12 w-12 shrink-0 overflow-hidden rounded-full shadow-lg ring-1 ring-white/15 transition-transform duration-200 group-hover:scale-110 group-focus-visible:scale-110 motion-reduce:transition-none"
    >
      {#if current.anime.coverImage}
        <img
          src={current.anime.coverImage}
          alt=""
          class="h-full w-full animate-spin-record rounded-full object-cover motion-reduce:animate-none group-hover:animate-spin-record-fast group-focus-visible:animate-spin-record-fast"
        />
      {:else}
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
        class="pointer-events-none absolute top-1/2 left-1/2 h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-surface ring-2 ring-white/20"
      ></span>
    </span>
  </button>
{/if}