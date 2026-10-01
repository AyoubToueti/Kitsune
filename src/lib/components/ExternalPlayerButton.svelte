<script lang="ts">
  import { errorMessage } from "$lib/api/anime";
  import { getPlayer, openInPlayer, setPlayer, suggestedPlayers } from "$lib/api/player";
  import Select, { type SelectOption } from "./Select.svelte";

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

  let player = $state("mpv");
  let options = $state<string[]>([]);
  let launching = $state(false);
  let error = $state<string | null>(null);

  /**
   * The dropdown's choices, as `{ value, label }`.
   *
   * A stored player that is not in the suggestion list is appended: without
   * it, the control would show the first suggestion and silently change the
   * user's choice.
   */
  const choices = $derived<SelectOption[]>(
    options.includes(player)
      ? options.map((name) => ({ value: name, label: name }))
      : [...options.map((name) => ({ value: name, label: name })),
         { value: player, label: player }],
  );

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
      await openInPlayer(url, player, torrentId);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      launching = false;
    }
  }

  /** Remember the choice so the next visit opens the same player. */
  async function remember(name: string): Promise<void> {
    await setPlayer(name).catch(() => {
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

  <!-- Themed dropdown, replacing the native `<select>` whose popup the browser
       draws and CSS cannot restyle. -->
  <Select
    label="External player"
    bind:value={player}
    options={choices}
    onchange={remember}
  />
</div>

{#if error}
  <p class="mt-2 text-xs text-ink-faint">{error}</p>
{/if}