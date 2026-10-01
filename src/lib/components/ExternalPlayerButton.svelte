<script lang="ts">
  import { errorMessage } from "$lib/api/anime";
  import { openInPlayer } from "$lib/api/player";

  let {
    url,
    label = "Open in external player",
    torrentId,
  }: {
    /** Stream URL. The button is disabled without one. */
    url?: string;
    label?: string;
    /**
     * The torrent the URL streams from.
     *
     * Passed to the backend so it holds the torrent while the player runs:
     * without it, leaving the watch page would remove the torrent and stop the
     * player mid-episode.
     */
    torrentId?: number;
  } = $props();

  let launching = $state(false);
  let error = $state<string | null>(null);

  async function launch(): Promise<void> {
    if (!url || launching) return;

    launching = true;
    error = null;
    try {
      // No player argument: the backend uses the reader's stored choice, read
      // live. The picker lives in the settings drawer, so the choice is made in
      // exactly one place and a change there applies to this launch without a
      // reload.
      await openInPlayer(url, undefined, torrentId);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      launching = false;
    }
  }
</script>

<div class="flex flex-wrap items-center gap-2">
  <button
    type="button"
    onclick={launch}
    disabled={!url || launching}
    class="rounded-lg border border-border-subtle bg-surface-hover px-4 py-2 text-sm font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
  >
    {launching ? "Opening…" : label}
  </button>
</div>

{#if error}
  <p class="mt-2 text-xs text-ink-faint">{error}</p>
{/if}