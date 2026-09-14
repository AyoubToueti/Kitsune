<script lang="ts">
  import { errorMessage } from "$lib/api/anime";
  import { getPlayer, openInPlayer, setPlayer, suggestedPlayers } from "$lib/api/player";

  let {
    url,
    label = "Open in external player",
  }: {
    /** Stream URL. The button is disabled without one. */
    url?: string;
    label?: string;
  } = $props();

  let player = $state("mpv");
  let options = $state<string[]>([]);
  let launching = $state(false);
  let error = $state<string | null>(null);

  // Fetched once: neither the list nor the stored choice changes while the
  // page is open, except through this component, which updates the local copy.
  getPlayer()
    .then((name) => {
      player = name;
    })
    .catch(() => {
      // A missing preference is not an error: the backend default applies.
    });

  suggestedPlayers()
    .then((found) => {
      options = found;
    })
    .catch(() => {
      options = [];
    });

  async function launch(): Promise<void> {
    if (!url || launching) return;

    launching = true;
    error = null;
    try {
      // Pass the chosen player explicitly so a change here takes effect on
      // this call rather than only after a save.
      await openInPlayer(url, player);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      launching = false;
    }
  }

  async function choose(event: Event): Promise<void> {
    const select = event.currentTarget as HTMLSelectElement;
    player = select.value;
    // Remembered so the next visit opens the same player.
    await setPlayer(player).catch(() => {
      // A failed save is not worth blocking playback over; the choice still
      // applies to this session.
    });
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

  <label class="sr-only" for="player-choice">External player</label>
  <select
    id="player-choice"
    bind:value={player}
    onchange={choose}
    class="rounded-lg border border-border-subtle bg-surface-hover px-2 py-2 text-sm text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  >
    {#each options as option (option)}
      <option value={option}>{option}</option>
    {/each}
    <!-- A stored player that is not in the suggestion list still needs to
         appear, or opening the page would silently reset the choice. -->
    {#if !options.includes(player)}
      <option value={player}>{player}</option>
    {/if}
  </select>
</div>

{#if error}
  <p class="mt-2 text-xs text-ink-faint">{error}</p>
{/if}