<script lang="ts">
  import type { Snippet } from "svelte";

  import type { TorrentProgress } from "$lib/types";
  import { healthLabel, streamWarning } from "$lib/stream-health";

  /**
   * The watch page's download panel.
   *
   * It replaces the in-app video element, which on Linux is a WebKitGTK
   * GStreamer pipeline: one per element, buffering unbounded by default, which
   * is what made playback consume all available memory. Playback now happens
   * in an external player, and this panel reports what the torrent is doing
   * while the reader waits for enough of it to be playable.
   *
   * The headline answers the reader's question -- "can I watch yet?" -- with
   * the chosen file's progress, since that is what playback waits on. The
   * whole-torrent figure is kept as a quiet aside: the two differ whenever a
   * release ships subtitles, artwork or a batch of episodes alongside the one
   * being watched, but the file's own progress is the one that matters.
   */

  let {
    progress = null,
    fileFraction = 0,
    torrentFraction = 0,
    empty = false,
    title = "Buffering…",
    speedHistory = [],
    staleSeconds = 0,
    playedFraction = 0,
    paused = false,
    fileBytes,
    fileTotalBytes,
    actions,
  }: {
    /** Latest snapshot, or `null` before the first poll has answered. */
    progress?: TorrentProgress | null;
    /** Downloaded fraction of the chosen file, 0..1. */
    fileFraction?: number;
    /** Downloaded fraction of the whole torrent, 0..1. */
    torrentFraction?: number;
    /** True when no torrent has been loaded yet. */
    empty?: boolean;
    /**
     * The headline for the loaded state.
     *
     * Defaults to "Buffering…" rather than "Downloading…": the app is only
     * fetching enough of the file to hand to the external player, not
     * downloading the whole episode, so "buffering" is the honest word.
     */
    title?: string;
    /** Recent download-speed samples (MiB/s), newest last, for the sparkline. */
    speedHistory?: number[];
    /** Seconds since the byte count last changed, for the stall warning. */
    staleSeconds?: number;
    /**
     * True while the reader has paused the buffering download.
     *
     * Only changes presentation: the connection line reads "Paused" and the
     * status dot stops pulsing, so a deliberate pause does not look like a
     * download still working.
     */
    paused?: boolean;
    /**
     * Extra controls for the header row, when a caller wants them.
     *
     * A snippet rather than fixed buttons so only the episode modal shows
     * pause/cancel; the watch page passes nothing and is unchanged.
     */
    actions?: Snippet;
    /**
     * How far into the chosen file the player has got, 0..1.
     *
     * Drawn as the "played" marker over the buffered bar, so the reader sees
     * the margin ahead of the playhead. `0` when nothing is playing yet.
     */
    playedFraction?: number;
    /**
     * Bytes of the chosen file downloaded so far.
     *
     * The reader watches one episode, so its own byte count is the number that
     * matters -- the whole-torrent total (which includes every other episode in
     * a pack) tells them nothing about whether they can start. Falls back to the
     * torrent totals when omitted.
     */
    fileBytes?: number;
    /** Total size of the chosen file, paired with `fileBytes`. */
    fileTotalBytes?: number;
  } = $props();

  /** The single notice worth showing, or `null`. */
  const warning = $derived(
    streamWarning(progress, staleSeconds, fileFraction),
  );
  /** The one-word health state beside the numbers. */
  const health = $derived(healthLabel(progress));

  /**
   * Whether the chosen episode is fully buffered.
   *
   * Drives a different layout: once complete, the swarm numbers (peers, live
   * speed, ETA) are all zero or meaningless -- the torrent is idle -- so the
   * card shows the finished facts instead of a row of dashes.
   */
  const complete = $derived(warning?.level === "ok");

  /** How many pixels a sparkline bar gets for a sample, given the max. */
  function sparkHeight(sample: number, max: number): string {
    if (max <= 0) return "2px";
    return `${Math.max(2, Math.round((sample / max) * 30))}px`;
  }

  /** The tallest sample in the history, so the sparkline self-scales. */
  const sparkMax = $derived(
    speedHistory.reduce((max, value) => Math.max(max, value), 0),
  );

  /** A byte count in binary units, e.g. "1.4 GB". */
  function formatBytes(bytes: number): string {
    if (bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${value.toFixed(value < 10 && unit > 0 ? 1 : 0)} ${units[unit]}`;
  }

  /** A transfer rate in MiB/s, or a dash when nothing is moving. */
  function formatSpeed(mbps: number): string {
    if (mbps <= 0) return "—";
    return `${mbps.toFixed(1)} MiB/s`;
  }

  /** A duration in seconds as "1h 2m" / "3m 4s" / "12s". */
  function formatEta(seconds: number | null): string {
    if (seconds === null || seconds <= 0) return "—";
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    if (hours > 0) return `${hours}h ${minutes}m`;
    if (minutes > 0) return `${minutes}m ${secs}s`;
    return `${secs}s`;
  }

  /** A whole-number percent from a 0..1 fraction. */
  function percent(fraction: number): number {
    return Math.round(Math.min(Math.max(fraction, 0), 1) * 100);
  }

  /**
   * The connection line, derived from the peer counts.
   *
   * "Connected" requires a live peer actually transferring pieces; a torrent
   * with only seen/connecting peers is still searching, and saying otherwise
   * would make a stalled download look healthy.
   */
  const connection = $derived.by(() => {
    if (progress === null) return "Waiting for stats…";
    if (progress.state === "error") return "Error";
    // The prop wins: the reader just pressed pause and the next poll may not
    // have landed yet, so trusting the snapshot alone would show "Connected"
    // for up to a poll interval after the button was pressed.
    if (paused || progress.state === "paused") return "Paused";
    if (progress.peersLive > 0) return `Connected (${progress.peersLive} peers)`;
    if (progress.peersConnecting > 0 || progress.peersQueued > 0) {
      return "Connecting…";
    }
    return "Searching for peers";
  });
</script>

{#if empty}
  <div
    class="flex aspect-video w-full items-center justify-center rounded-xl border border-dashed border-border-subtle bg-surface-hover"
  >
    <p class="text-sm text-ink-muted">Load a torrent to start watching.</p>
  </div>
{:else}
  <!-- A compact card rather than a fixed video box: playback is external, so
       the empty `aspect-video` frame was just dead space. It hugs its content
       and leads with what the reader is waiting for. -->
  <div
    class="flex w-full flex-col gap-3 rounded-xl border border-border-subtle bg-surface-hover p-4"
    data-testid="stream-status"
  >
    <!-- Headline: a status dot, what is happening, and the percentage. A
         finished download stops pulsing (nothing is happening) and turns
         green. -->
    <div class="flex items-center gap-2.5">
      <span
        aria-hidden="true"
        class="size-2 shrink-0 rounded-full {complete
          ? 'bg-health-green'
          : paused
            ? 'bg-ink-faint'
            : 'bg-accent motion-safe:animate-pulse'}"
      ></span>
      <span class="text-sm font-semibold {complete ? 'text-health-green' : 'text-ink'}"
        >{complete ? "Fully buffered" : paused ? "Paused" : title}</span
      >
      <span
        class="ml-auto text-sm font-bold {complete
          ? 'text-health-green'
          : 'text-accent-hover'}"
        data-testid="file-percent">{percent(fileFraction)}%</span
      >
      {#if actions}
        <div class="flex items-center gap-1.5">{@render actions()}</div>
      {/if}
    </div>

    <!-- The chosen file's bar, the thing the reader is actually waiting on. -->
    <div>
      <div class="relative h-1.5 w-full overflow-hidden rounded-full bg-border-subtle">
        <div
          class="h-full rounded-full bg-accent transition-[width] duration-300"
          style="width: {percent(fileFraction)}%"
          data-testid="file-bar"
        ></div>
        <!-- The playhead: how far into the buffered file the player has got.
             Only drawn once there is a played position to show. -->
        {#if playedFraction > 0}
          <div
            class="absolute inset-y-0 w-0.5 bg-ink"
            style="left: {percent(playedFraction)}%"
            data-testid="played-marker"
            aria-hidden="true"
          ></div>
        {/if}
      </div>
      {#if progress}
        <div class="mt-1.5 flex items-center justify-between text-[11px]">
          <span class="text-ink-faint" data-testid="bytes">
            {#if fileBytes !== undefined && fileTotalBytes !== undefined}
              {formatBytes(fileBytes)} / {formatBytes(fileTotalBytes)}
            {:else}
              {formatBytes(progress.progressBytes)} / {formatBytes(
                progress.totalBytes,
              )}
            {/if}
          </span>
          <!-- The whole-torrent figure is kept for tests and context, but it
               sits quietly beside the byte count rather than owning a bar. -->
          <span class="text-ink-faint" data-testid="torrent-percent"
            >whole torrent {percent(torrentFraction)}%</span
          >
        </div>
      {/if}
      <!-- The whole-torrent bar is retained (hidden) so the existing tests and
           the watch page keep one element to measure; it is no longer drawn. -->
      <div class="hidden" aria-hidden="true">
        <div data-testid="torrent-bar" style="width: {percent(torrentFraction)}%"></div>
      </div>
    </div>

    <!-- The live line: connection, speed and ETA, the things a stalled
         download needs to be diagnosable at a glance. Hidden once the file is
         complete, when every one of these reads as a dash. -->
    {#if !complete}
    <dl class="flex flex-wrap gap-x-5 gap-y-1 text-[11px]">
      <div class="flex items-center gap-1.5">
        <dt class="text-ink-faint">Peers</dt>
        <dd class="text-ink-muted" data-testid="connection">{connection}</dd>
      </div>
      <div class="flex items-center gap-1.5">
        <dt class="text-ink-faint">Down</dt>
        <dd class="text-ink-muted" data-testid="down-speed"
          >{formatSpeed(progress?.downloadMbps ?? 0)}</dd
        >
      </div>
      <div class="flex items-center gap-1.5">
        <dt class="text-ink-faint">Up</dt>
        <dd class="text-ink-muted" data-testid="up-speed"
          >{formatSpeed(progress?.uploadMbps ?? 0)}</dd
        >
      </div>
      <div class="flex items-center gap-1.5">
        <dt class="text-ink-faint">ETA</dt>
        <dd class="text-ink-muted" data-testid="eta"
          >{formatEta(progress?.etaSeconds ?? null)}</dd
        >
      </div>
      <div class="flex items-center gap-1.5">
        <dt class="text-ink-faint">Health</dt>
        <dd
          class="font-medium {health === 'Healthy'
            ? 'text-health-green'
            : health === 'Error' || health === 'Slow'
              ? 'text-health-yellow'
              : 'text-ink-muted'}"
          data-testid="health">{health}</dd
        >
      </div>
    </dl>

    <!-- Sparkline: the last several speeds. A flat line reads as "stalled"
         without the reader having to interpret any single number. -->
    {#if speedHistory.length > 1}
      <div
        class="flex h-8 items-end gap-0.5"
        data-testid="speed-spark"
        aria-hidden="true"
        title="Download speed, most recent last"
      >
        {#each speedHistory as sample, i (i)}
          <span
            class="flex-1 rounded-t-sm {sample <= 0 ? 'bg-ink-faint/50' : 'bg-accent/80'}"
            style="height: {sparkHeight(sample, sparkMax)}"
          ></span>
        {/each}
      </div>
    {/if}
    {/if}

    <!-- The one notice worth showing, if any, in a sentence. -->
    {#if warning}
      <p
        class="flex items-start gap-2 rounded-lg border px-3 py-2 text-xs leading-relaxed {warning.level ===
        'error'
          ? 'border-danger/35 bg-danger/10 text-danger'
          : warning.level === 'warn'
            ? 'border-health-yellow/35 bg-health-yellow/10 text-health-yellow'
            : warning.level === 'ok'
              ? 'border-health-green/35 bg-health-green/10 text-health-green'
              : 'border-accent/30 bg-accent/10 text-accent-hover'}"
        role="status"
        data-testid="stream-warning"
        data-level={warning.level}
      >
        <span aria-hidden="true">{warning.icon}</span>
        <span>{warning.message}</span>
      </p>
    {/if}
  </div>
{/if}