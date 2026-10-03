<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { goto } from "$app/navigation";

  import { displayTitle } from "$lib/types";
  import {
    isActive,
    nowPlaying,
    nowPlayingSession,
    requestOpen,
  } from "$lib/now-playing.svelte";
  import { formatSize } from "$lib/release-display";
  import { healthLabel, streamWarning } from "$lib/stream-health";

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
    isHovered = false; // the modal is about to cover the disc; drop the hover state
    requestOpen();
    void goto(`/anime/${current.anime.id}`);
  }

  // A rAF loop, not a CSS animation: a CSS animation restarts from its own
  // timeline when the duration changes, so speeding up on hover would jump the
  // angle. Accumulating the angle and easing the speed keeps the spin
  // continuous -- the same approach the resume disc uses.
  let isHovered = $state(false);
  let rotation = $state(0);
  let animationFrameId: number | undefined;
  let lastTimestamp: number | null = null;
  let currentSpeed = 0.03; // deg/ms, ~30 deg/s idle

  /** The one notice worth showing, shared with the modal's StreamStatus. */
  const warning = $derived(
    streamWarning(session.progress, session.staleSeconds, fraction),
  );
  const health = $derived(healthLabel(session.progress));
  const complete = $derived(warning?.level === "ok");
  const percent = $derived(Math.round(Math.min(Math.max(fraction, 0), 1) * 100));

  /** A transfer rate in MiB/s, or a dash when nothing is moving. */
  function formatSpeed(mbps: number): string {
    return mbps > 0 ? `${mbps.toFixed(1)} MiB/s` : "—";
  }

  /** A duration in seconds as "1h 2m" / "3m 4s" / "12s". */
  function formatEta(seconds: number | null): string {
    if (seconds === null || seconds <= 0) return "—";
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  }

  function animateRotation(timestamp: number) {
    if (lastTimestamp !== null) {
      const delta = timestamp - lastTimestamp;
      const targetSpeed = isHovered ? 0.16 : 0.03; // faster while hovered
      currentSpeed += (targetSpeed - currentSpeed) * 0.05; // ease, no snap
      rotation = (rotation + currentSpeed * delta) % 360;
    }
    lastTimestamp = timestamp;
    animationFrameId = requestAnimationFrame(animateRotation);
  }

  onMount(() => {
    animationFrameId = requestAnimationFrame(animateRotation);
  });

  onDestroy(() => {
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
  });
</script>

{#if isActive() && current}
  <div
    class="fixed right-6 bottom-6 z-40"
    role="button"
    tabindex="0"
    onmouseenter={() => (isHovered = true)}
    onmouseleave={() => (isHovered = false)}
    onfocusin={(event) => {
      // Only keyboard focus opens the card. A mouse click (and the modal
      // restoring focus to this button on close) does NOT match :focus-visible,
      // so it cannot leave the card stuck open with no pointer over it.
      if (event.target instanceof Element && event.target.matches(":focus-visible")) {
        isHovered = true;
      }
    }}
    onfocusout={() => (isHovered = false)}
  >
    {#if isHovered}
      <div
        aria-hidden="true"
        data-testid="now-playing-card"
        class="absolute right-full bottom-0 mr-3 w-64 max-h-[80vh] overflow-y-auto origin-bottom-right transition-all duration-300 ease-out motion-reduce:transition-none"
      >
        <div
          class="rounded-2xl border border-border-subtle/80 bg-surface-raised/95 p-3.5 shadow-2xl backdrop-blur-md"
        >
          <!-- Header -->
          <span class="flex items-center gap-1.5 text-[10px] font-bold tracking-wider text-accent uppercase">
            <span class="relative flex h-2 w-2">
              <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent opacity-75"></span>
              <span class="relative inline-flex h-2 w-2 rounded-full bg-accent"></span>
            </span>
            {complete ? "Ready to play" : "Now Playing"}
          </span>

          <p class="mt-1.5 line-clamp-1 text-xs font-bold text-ink">{title}</p>
          <p class="mt-0.5 text-[11px] font-medium text-ink-muted">
            {current.episode !== undefined ? `Episode ${current.episode}` : "Playing"}
          </p>

          <!-- Buffer bar (the chosen file, what playback waits on) -->
          <div class="mt-2.5">
            <div class="flex items-center justify-between text-[11px]">
              <span class="font-semibold {complete ? 'text-health-green' : 'text-accent-hover'}">
                {complete ? "Fully buffered" : "Buffering"}
              </span>
              <span class="{complete ? 'text-health-green' : 'text-accent-hover'} font-bold">{percent}%</span>
            </div>
            <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-border-subtle">
              <div
                class="h-full rounded-full {complete ? 'bg-health-green' : 'bg-accent'} transition-[width] duration-300"
                style="width: {percent}%"
              ></div>
            </div>
          </div>

          <!-- The one notice, or a plain waiting line until the first poll -->
          {#if warning}
            <p
              class="mt-2 text-[11px] leading-snug {warning.level === 'error'
                ? 'text-danger'
                : warning.level === 'warn'
                  ? 'text-health-yellow'
                  : warning.level === 'ok'
                    ? 'text-health-green'
                    : 'text-accent-hover'}"
            >
              <span aria-hidden="true">{warning.icon}</span> {warning.message}
            </p>
          {:else}
            <p class="mt-2 text-[11px] text-ink-muted">Waiting for stats…</p>
          {/if}

          <!-- Live line: only meaningful before it is fully buffered -->
          {#if !complete && session.progress}
            <dl class="mt-2 flex flex-wrap gap-x-4 gap-y-0.5 border-t border-border-subtle/50 pt-2 text-[10.5px]">
              <div class="flex gap-1">
                <dt class="text-ink-faint">Down</dt>
                <dd class="text-ink-muted">{formatSpeed(session.progress.downloadMbps)}</dd>
              </div>
              <div class="flex gap-1">
                <dt class="text-ink-faint">Peers</dt>
                <dd class="text-ink-muted">{session.progress.peersLive}</dd>
              </div>
              <div class="flex gap-1">
                <dt class="text-ink-faint">ETA</dt>
                <dd class="text-ink-muted">{formatEta(session.progress.etaSeconds)}</dd>
              </div>
              <div class="flex gap-1">
                <dt class="text-ink-faint">Health</dt>
                <dd class="font-medium {health === 'Healthy' ? 'text-health-green' : health === 'Error' || health === 'Slow' ? 'text-health-yellow' : 'text-ink-muted'}">{health}</dd>
              </div>
            </dl>
          {/if}

          <!-- CTA row, matching the resume card -->
          <div class="mt-2.5 flex items-center justify-between border-t border-border-subtle/50 pt-2 text-xs font-semibold text-accent">
            <span>Open buffer window</span>
            <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="currentColor">
              <path d="M8 5v14l11-7z" />
            </svg>
          </div>
        </div>
      </div>
    {/if}

    <button
      type="button"
      onclick={reopen}
      aria-label={`Now playing ${title}. Open the buffer window.`}
      data-testid="now-playing-disc"
      class="group grid size-16 place-items-center rounded-full transition-transform duration-300 ease-out motion-reduce:transition-none {isHovered
        ? 'scale-110'
        : ''}"
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
        class="relative block h-12 w-12 shrink-0 overflow-hidden rounded-full shadow-lg ring-1 ring-white/10 transition-all duration-300 {isHovered
          ? 'ring-accent/50'
          : ''}"
      >
        {#if current.anime.coverImage}
          <img
            src={current.anime.coverImage}
            alt=""
            style="transform: rotate({rotation}deg);"
            class="h-full w-full rounded-full object-cover transition-opacity {isHovered
              ? 'opacity-100'
              : 'opacity-85'}"
          />
        {:else}
          <span
            style="transform: rotate({rotation}deg);"
            class="block h-full w-full rounded-full bg-neutral-800"
          ></span>
        {/if}

        <!-- Concentric vinyl grooves. -->
        <span
          aria-hidden="true"
          class="pointer-events-none absolute inset-0 rounded-full border border-white/5 bg-[radial-gradient(ellipse_at_center,var(--tw-gradient-stops))] from-transparent via-black/20 to-black/60"
        ></span>

        <!-- Gloss reflection. -->
        <span
          aria-hidden="true"
          class="pointer-events-none absolute inset-0 rounded-full bg-linear-to-tr from-transparent via-white/15 to-transparent"
        ></span>

        <!-- Centre hub: accent label with a spindle hole, like the resume disc. -->
        <span
          aria-hidden="true"
          class="pointer-events-none absolute top-1/2 left-1/2 flex h-3 w-3 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-accent ring-2 ring-neutral-950/80 shadow-inner"
        >
          <span class="h-1 w-1 rounded-full bg-neutral-950"></span>
        </span>

        <span
          aria-hidden="true"
          class="pointer-events-none absolute top-1/2 left-1/2 flex h-3 w-3 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-accent ring-2 ring-neutral-950/80 shadow-inner"
        >
          <span class="h-1 w-1 rounded-full bg-neutral-950"></span>
        </span>
      </span>
    </button>
  </div>
{/if}