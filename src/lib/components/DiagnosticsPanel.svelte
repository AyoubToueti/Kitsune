<script lang="ts">
  import { onMount } from "svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import { getLogPath, getSystemStats } from "$lib/api/diagnostics";
  import { formatSize } from "$lib/release-display";
  import type { SystemStats } from "$lib/types";

  /**
   * Live resource usage for the app process, plus a shortcut to its log file.
   *
   * Polls `get_system_stats` while mounted. It is placed inside the settings
   * drawer, whose modal renders its content only while open, so the poll runs
   * only when the reader is actually looking at it -- no timer churns in the
   * background.
   */

  /** How often the snapshot is refreshed while the panel is visible. */
  const POLL_MS = 2000;

  let stats = $state<SystemStats | null>(null);
  let error = $state<string | null>(null);

  onMount(() => {
    let active = true;

    const tick = async (): Promise<void> => {
      try {
        const next = await getSystemStats();
        if (!active) return;
        stats = next;
        error = null;
      } catch (err) {
        if (active) error = describe(err);
      }
    };

    // Read once immediately, then on an interval. The first snapshot may read
    // 0% CPU, since sysinfo needs two samples; the next poll fills it in.
    void tick();
    const timer = setInterval(() => void tick(), POLL_MS);

    return () => {
      active = false;
      clearInterval(timer);
    };
  });

  /** A readable message from an unknown rejection. */
  function describe(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
  }

  /** Seconds as "1h 5m" / "3m 20s" / "12s". */
  function formatUptime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    if (hours > 0) return `${hours}h ${minutes}m`;
    if (minutes > 0) return `${minutes}m ${secs}s`;
    return `${secs}s`;
  }

  /** Open the log file's folder in the OS file manager. */
  async function revealLog(): Promise<void> {
    try {
      const path = await getLogPath();
      await revealItemInDir(path);
    } catch (err) {
      error = describe(err);
    }
  }
</script>

<section class="mt-6" data-testid="diagnostics-panel">
  <h3 class="mb-2 text-xs font-semibold tracking-wide text-ink-faint uppercase">
    Diagnostics
  </h3>

  {#if stats === null}
    <p class="py-4 text-center text-xs text-ink-faint">
      {error ?? "Reading resource usage…"}
    </p>
  {:else}
    <dl class="grid grid-cols-2 gap-2">
      <div
        class="rounded-lg border border-border-subtle bg-surface-hover px-3 py-2"
      >
        <dt class="text-[11px] text-ink-faint">CPU</dt>
        <dd class="text-sm text-ink" data-testid="diag-cpu">
          {stats.cpuPercentOfMachine.toFixed(1)}%
          <span class="text-[11px] text-ink-faint" data-testid="diag-cpu-detail">
            ({stats.cpuPercent.toFixed(0)}% of a core · {stats.cpuCores} cores)
          </span>
        </dd>
      </div>
      <div
        class="rounded-lg border border-border-subtle bg-surface-hover px-3 py-2"
      >
        <dt class="text-[11px] text-ink-faint">Memory</dt>
        <dd class="text-sm text-ink" data-testid="diag-memory">
          {formatSize(stats.rssBytes) ?? "—"}
        </dd>
      </div>
      <div
        class="rounded-lg border border-border-subtle bg-surface-hover px-3 py-2"
      >
        <dt class="text-[11px] text-ink-faint">Threads</dt>
        <dd class="text-sm text-ink" data-testid="diag-threads">
          {stats.threadCount}
        </dd>
      </div>
      <div
        class="rounded-lg border border-border-subtle bg-surface-hover px-3 py-2"
      >
        <dt class="text-[11px] text-ink-faint">Processes</dt>
        <dd class="text-sm text-ink" data-testid="diag-processes">
          {stats.processCount}
        </dd>
      </div>
      <div
        class="rounded-lg border border-border-subtle bg-surface-hover px-3 py-2"
      >
        <dt class="text-[11px] text-ink-faint">Uptime</dt>
        <dd class="text-sm text-ink" data-testid="diag-uptime">
          {formatUptime(stats.uptimeSeconds)}
        </dd>
      </div>
    </dl>

    <p class="mt-2 text-[11px] text-ink-faint">
      System memory in use:
      {formatSize(stats.systemTotalBytes - stats.systemAvailableBytes) ?? "—"}
      of {formatSize(stats.systemTotalBytes) ?? "—"}
    </p>
  {/if}

  <button
    type="button"
    onclick={revealLog}
    class="mt-3 w-full rounded-lg border border-border-subtle px-3 py-1.5 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  >
    Reveal log file
  </button>

  {#if error && stats !== null}
    <p class="mt-2 text-[11px] text-danger" data-testid="diag-error">
      {error}
    </p>
  {/if}
</section>