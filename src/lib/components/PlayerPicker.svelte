<script lang="ts">
  import { errorMessage } from "$lib/api/anime";
  import {
    listPlayers,
    openInPlayerChoice,
    type PlayerOption,
  } from "$lib/api/player";
  import { setDefaultPlayer } from "$lib/api/settings";

  import Modal from "./Modal.svelte";

  let {
    open,
    url,
    torrentId,
    onClose,
    onLaunched,
  }: {
    open: boolean;
    url?: string;
    torrentId?: number;
    onClose: () => void;
    onLaunched?: (program: string) => void;
  } = $props();

  let players = $state<PlayerOption[] | null>(null);
  let error = $state<string | null>(null);
  let launching = $state<string | null>(null);
  let selected = $state<string | null>(null);

  $effect(() => {
    if (!open) return;

    let cancelled = false;
    players = null;
    error = null;

    listPlayers()
      .then((result) => {
        if (cancelled) return;
        players = result;
        const preferred = result.find((p) => p.isDefault) ?? result[0];
        selected = preferred?.program ?? null;
      })
      .catch((err) => {
        if (cancelled) return;
        error = errorMessage(err);
      });

    return () => {
      cancelled = true;
    };
  });

  const selectedPlayer = $derived(
    players?.find((p) => p.program === selected)
  );

  async function always(): Promise<void> {
    if (!selectedPlayer || url === undefined || launching !== null) return;

    launching = selectedPlayer.program;
    error = null;
    try {
      await setDefaultPlayer(selectedPlayer.program, selectedPlayer.extraArgs);
      const program = await openInPlayerChoice(
        url,
        selectedPlayer.program,
        selectedPlayer.extraArgs,
        torrentId
      );
      onLaunched?.(program);
      onClose();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      launching = null;
    }
  }

  async function pick(player: PlayerOption): Promise<void> {
    if (url === undefined || launching !== null) return;

    // Remember which row was last opened, so the "Always" button acts on it.
    selected = player.program;
    launching = player.program;
    error = null;
    try {
      const program = await openInPlayerChoice(
        url,
        player.program,
        player.extraArgs,
        torrentId
      );
      onLaunched?.(program);
      onClose();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      launching = null;
    }
  }
</script>

<Modal {open} {onClose} label="Choose a player">
  <!-- Modal Header -->
  <header class="flex items-start justify-between gap-4 border-b border-border-subtle p-5">
    <div class="min-w-0">
      <h2 class="text-lg font-bold tracking-tight text-ink">Choose a player</h2>
      <p class="mt-0.5 text-xs text-ink-faint">
        Select an external video player installed on your system.
      </p>
    </div>
    <button
      type="button"
      onclick={onClose}
      aria-label="Close dialog"
      class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-border-subtle bg-surface-hover text-ink-muted transition-colors hover:border-accent hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      <svg class="size-4 fill-none stroke-current stroke-2" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </header>

  <!-- Modal Body -->
  <div class="min-h-0 flex-1 overflow-y-auto p-5" data-testid="player-picker">
    {#if error}
      <div class="rounded-xl border border-danger/30 bg-danger/5 p-4 text-center">
        <p class="text-xs font-medium text-danger" data-testid="player-picker-error">
          {error}
        </p>
      </div>
    {:else if players === null}
      <div class="py-8 text-center" data-testid="player-picker-loading">
        <div class="mx-auto size-6 animate-spin rounded-full border-2 border-border-subtle border-t-accent"></div>
        <p class="mt-3 text-xs text-ink-muted">Looking for installed media players…</p>
      </div>
    {:else if players.length === 0}
      <div class="py-8 text-center" data-testid="player-picker-empty">
        <p class="text-xs text-ink-muted">
          No external video players detected.
        </p>
        <p class="mt-1 text-[11px] text-ink-faint">
          Install MPV or VLC to play video streams directly.
        </p>
      </div>
    {:else}
      <ul class="flex flex-col gap-2">
        {#each players as player (player.id)}
          {@const isSelected = player.program === selected}
          {@const isLaunchingThis = launching === player.program}
          <li>
            <button
              type="button"
              onclick={() => pick(player)}
              disabled={launching !== null}
              data-testid="player-option"
              data-default={player.isDefault}
              data-selected={isSelected}
              class="group flex w-full items-center justify-between gap-3 rounded-xl border p-3 text-left transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50 {isSelected
                ? 'border-accent bg-accent/5 ring-1 ring-accent/30'
                : 'border-border-subtle bg-surface-raised hover:border-accent/60 hover:bg-surface-hover'}"
            >
              <div class="flex min-w-0 items-center gap-3">
                {#if player.icon}
                  <img
                    src={player.icon}
                    alt=""
                    aria-hidden="true"
                    data-testid="player-icon"
                    class="size-9 shrink-0 rounded-lg border border-border-subtle bg-surface object-contain p-1"
                  />
                {:else}
                  <div
                    data-testid="player-icon-fallback"
                    class="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border-subtle bg-surface text-ink-muted"
                  >
                    <svg class="size-5 fill-none stroke-current stroke-2" viewBox="0 0 24 24">
                      <rect x="2" y="4" width="20" height="16" rx="3" />
                      <path d="M10 9l5 3-5 3z" fill="currentColor" stroke="none" />
                    </svg>
                  </div>
                {/if}

                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="truncate text-sm font-semibold text-ink">
                      {player.name}
                    </span>
                    {#if player.isDefault}
                      <span class="rounded bg-accent/15 px-1.5 py-0.2 text-[0.65rem] font-bold tracking-wider text-accent uppercase">
                        Default
                      </span>
                    {/if}
                  </div>
                  <span class="block truncate text-xs text-ink-faint">
                    {player.program}
                  </span>
                </div>
              </div>

              <!-- Status indicator: the row itself opens the player. -->
              <div class="flex items-center gap-2 shrink-0">
                {#if isLaunchingThis}
                  <span class="text-xs font-medium text-accent">Opening…</span>
                {:else}
                  <svg
                    class="size-4 fill-none stroke-current stroke-2 text-ink-faint group-hover:text-accent"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      d="M9 5l7 7-7 7"
                    />
                  </svg>
                {/if}
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <!-- Modal Footer Actions -->
  {#snippet footer()}
    {#if players && players.length > 0}
      <div class="flex items-center justify-between gap-3 border-t border-border-subtle bg-surface-raised px-5 py-3.5">
        <button
          type="button"
          onclick={always}
          disabled={selected === null || launching !== null}
          title="Set as default player and open"
          data-testid="player-always"
          class="rounded-lg border border-accent bg-accent/10 px-4 py-2 text-xs font-semibold text-accent transition-colors hover:bg-accent hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-50"
        >
          Always use this player
        </button>

        <div class="flex items-center gap-2">
          <button
            type="button"
            onclick={onClose}
            class="rounded-lg px-4 py-2 text-xs font-medium text-ink-muted transition-colors hover:bg-surface-hover hover:text-ink focus:outline-none"
          >
            Cancel
          </button>
          {#if selectedPlayer}
            <button
              type="button"
              onclick={() => pick(selectedPlayer)}
              disabled={launching !== null}
              class="rounded-lg bg-accent px-4 py-2 text-xs font-semibold text-white shadow-sm transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50"
            >
              Open once
            </button>
          {/if}
        </div>
      </div>
    {/if}
  {/snippet}
</Modal>