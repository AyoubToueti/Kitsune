<script lang="ts">
  import type { TorrentProgress } from "$lib/types";

  /**
   * The watch page's download panel.
   *
   * It replaces the in-app video element, which on Linux is a WebKitGTK
   * GStreamer pipeline: one per element, buffering unbounded by default, which
   * is what made playback consume all available memory. Playback now happens
   * in an external player, and this panel reports what the torrent is doing
   * while the reader waits for enough of it to be playable.
   *
   * The two bars answer different questions. "This episode" is the chosen
   * file, which is what the reader is waiting on; "Whole torrent" is every
   * file, which is what the torrent is actually doing. They differ whenever a
   * release ships subtitles, artwork or a batch of episodes alongside the one
   * being watched.
   */

  let {
    progress = null,
    fileFraction = 0,
    torrentFraction = 0,
    empty = false,
  }: {
    /** Latest snapshot, or `null` before the first poll has answered. */
    progress?: TorrentProgress | null;
    /** Downloaded fraction of the chosen file, 0..1. */
    fileFraction?: number;
    /** Downloaded fraction of the whole torrent, 0..1. */
    torrentFraction?: number;
    /** True when no torrent has been loaded yet. */
    empty?: boolean;
  } = $props();

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
  <div
    class="flex aspect-video w-full flex-col justify-center gap-4 rounded-xl border border-border-subtle bg-surface-hover p-5"
    data-testid="stream-status"
  >
    <!-- This episode: the file the reader is actually waiting on. -->
    <div>
      <div class="mb-1 flex items-center justify-between text-xs">
        <span class="font-medium text-ink">This episode</span>
        <span class="text-ink-muted" data-testid="file-percent"
          >{percent(fileFraction)}%</span
        >
      </div>
      <div class="h-2 w-full overflow-hidden rounded-full bg-border-subtle">
        <div
          class="h-full rounded-full bg-accent transition-[width] duration-300"
          style="width: {percent(fileFraction)}%"
          data-testid="file-bar"
        ></div>
      </div>
    </div>

    <!-- Whole torrent: everything the release ships, not just the episode. -->
    <div>
      <div class="mb-1 flex items-center justify-between text-xs">
        <span class="font-medium text-ink-muted">Whole torrent</span>
        <span class="text-ink-faint" data-testid="torrent-percent"
          >{percent(torrentFraction)}%</span
        >
      </div>
      <div class="h-1.5 w-full overflow-hidden rounded-full bg-border-subtle">
        <div
          class="h-full rounded-full bg-ink-faint transition-[width] duration-300"
          style="width: {percent(torrentFraction)}%"
          data-testid="torrent-bar"
        ></div>
      </div>
    </div>

    {#if progress?.error}
      <p class="text-xs text-danger" role="status">
        {progress.error}
      </p>
    {/if}

    <!-- The live line: connection, speed and ETA, the things a stalled
         download needs to be diagnosable at a glance. -->
    <dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs sm:grid-cols-4">
      <div>
        <dt class="text-ink-faint">Status</dt>
        <dd class="text-ink-muted" data-testid="connection">{connection}</dd>
      </div>
      <div>
        <dt class="text-ink-faint">Down</dt>
        <dd class="text-ink-muted" data-testid="down-speed"
          >{formatSpeed(progress?.downloadMbps ?? 0)}</dd
        >
      </div>
      <div>
        <dt class="text-ink-faint">Up</dt>
        <dd class="text-ink-muted" data-testid="up-speed"
          >{formatSpeed(progress?.uploadMbps ?? 0)}</dd
        >
      </div>
      <div>
        <dt class="text-ink-faint">ETA</dt>
        <dd class="text-ink-muted" data-testid="eta"
          >{formatEta(progress?.etaSeconds ?? null)}</dd
        >
      </div>
    </dl>

    {#if progress}
      <p class="text-xs text-ink-faint" data-testid="bytes">
        {formatBytes(progress.progressBytes)} / {formatBytes(progress.totalBytes)}
      </p>
    {/if}
  </div>
{/if}