<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { beginLogin, authStatus, logout, onAuthChanged } from "$lib/api/auth";
  import {
    getSettings,
    onSettingsChanged,
    setSettings,
  } from "$lib/api/settings";
  import { suggestedPlayers } from "$lib/api/player";
  import { errorMessage } from "$lib/api/anime";
  import { RESOLUTION_ORDER } from "$lib/resolution";
  import { applyTheme, type ThemeChoice } from "$lib/theme.svelte";
  import type { Resolution, Settings } from "$lib/types";

  import DiagnosticsPanel from "./DiagnosticsPanel.svelte";
  import Modal from "./Modal.svelte";
  import Select, { type SelectOption } from "./Select.svelte";

  /** The resolution choices for the "add" dropdown, highest first. */
  const resolutionChoices: SelectOption[] = RESOLUTION_ORDER.map(
    (resolution) => ({ value: resolution, label: resolution }),
  );

  /**
   * The settings surface, opened from the NavBar avatar.
   *
   * Loads the whole `Settings` object, edits it locally, and saves the whole
   * object back on each change -- the backend owns the file, so there is no
   * partial-update surface. The theme is applied immediately (not only on
   * save), so a toggle feels instant even if the write is slow.
   */
  let { open = false, onClose }: { open?: boolean; onClose: () => void } =
    $props();

  let settings = $state<Settings | null>(null);
  let signedIn = $state(false);
  let players = $state<string[]>([]);
  let error = $state<string | null>(null);
  let saving = $state(false);
  /** New resolution picked in the "add" dropdown. */
  let pendingResolution = $state<Resolution>("1080p");

  // Load once when the drawer first opens: nothing here changes server-side
  // except through this panel, so re-reading on every open is unnecessary.
  $effect(() => {
    if (!open || settings !== null) return;
    load();
  });

  async function load(): Promise<void> {
    error = null;
    try {
      settings = await getSettings();
      applyTheme(themeChoice(settings.theme));
    } catch (err) {
      error = errorMessage(err);
    }
    players = await suggestedPlayers().catch(() => []);
    signedIn = await authStatus().catch(() => false);
  }

  onMount(() => {
    // A sign-in completing while the drawer is open updates the button.
    const unlisten = onAuthChanged((next) => {
      signedIn = next;
    });
    // Settings changed by another surface (a second window, or the player
    // picker) re-read, so the drawer never shows stale values.
    const unlistenSettings = onSettingsChanged((next) => {
      settings = next;
    });
    return () => {
      void unlisten.then((fn) => fn());
      void unlistenSettings.then((fn) => fn());
    };
  });

  function themeChoice(value: string): ThemeChoice {
    return value === "light" || value === "dark" ? value : "system";
  }

  /** Save the current settings, optionally after applying a local change. */
  async function persist(patch: Partial<Settings>): Promise<void> {
    if (settings === null) return;
    const next = { ...settings, ...patch };
    settings = next;
    saving = true;
    error = null;
    try {
      // The backend may clamp a value (the ready fraction); keep its answer.
      settings = await setSettings(next);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      saving = false;
    }
  }

  /** Theme changes apply immediately, then persist. */
  function chooseTheme(choice: ThemeChoice): void {
    applyTheme(choice);
    void persist({ theme: choice });
  }

  async function connect(): Promise<void> {
    try {
      const url = await beginLogin();
      await openUrl(url);
    } catch (err) {
      error = errorMessage(err);
    }
  }

  async function disconnect(): Promise<void> {
    try {
      await logout();
      signedIn = false;
    } catch (err) {
      error = errorMessage(err);
    }
  }

  async function pickDownloadDir(): Promise<void> {
    const picked = await openDialog({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    await persist({ downloadDir: picked });
  }

  /** Toggle a resolution's presence in the preferred list. */
  function toggleResolution(resolution: Resolution): void {
    if (settings === null) return;
    const current = settings.preferredResolutions;
    const next = current.includes(resolution)
      ? current.filter((value) => value !== resolution)
      : [...current, resolution];
    void persist({ preferredResolutions: next });
  }
</script>

<Modal {open} {onClose} label="Settings">
  <header class="flex items-center gap-3 border-b border-border-subtle p-5">
    <h2 class="text-lg font-semibold tracking-tight">Settings</h2>
    <button
      type="button"
      onclick={onClose}
      aria-label="Close"
      class="ml-auto size-8 rounded-lg border border-border-subtle bg-surface-hover text-ink-muted transition-colors hover:border-accent hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      ✕
    </button>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto p-5">
    {#if settings === null}
      <p class="py-10 text-center text-sm text-ink-muted">Loading…</p>
    {:else}
      <!-- Account -->
      <section class="mb-6">
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-ink-faint uppercase">
          AniList account
        </h3>
        <div class="flex items-center justify-between rounded-lg border border-border-subtle bg-surface-hover px-4 py-3">
          <span class="text-sm text-ink-muted">
            {signedIn ? "Connected to AniList" : "Not connected"}
          </span>
          {#if signedIn}
            <button
              type="button"
              onclick={disconnect}
              class="rounded-lg border border-border-subtle px-3 py-1.5 text-xs font-medium text-ink-muted transition-colors hover:border-danger hover:text-danger focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              Disconnect
            </button>
          {:else}
            <button
              type="button"
              onclick={connect}
              class="rounded-lg bg-accent px-3 py-1.5 text-xs font-semibold text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              Connect
            </button>
          {/if}
        </div>
      </section>

      <!-- Player -->
      <section class="mb-6">
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-ink-faint uppercase">
          Player
        </h3>
        <label for="settings-player" class="mb-1 block text-xs text-ink-muted">
          Program
        </label>
        <div class="mb-3 flex gap-2">
          <input
            id="settings-player"
            list="player-suggestions"
            value={settings.player}
            oninput={(event) =>
              (settings = { ...settings!, player: event.currentTarget.value })}
            onchange={(event) => persist({ player: event.currentTarget.value })}
            placeholder="mpv"
            class="min-w-0 flex-1 rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm text-ink placeholder:text-ink-faint focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
          <datalist id="player-suggestions">
            {#each players as name (name)}
              <option value={name}></option>
            {/each}
          </datalist>
        </div>
        <label for="settings-player-args" class="mb-1 block text-xs text-ink-muted">
          Extra arguments (before the stream URL)
        </label>
        <input
          id="settings-player-args"
          value={settings.playerArgs.join(" ")}
          oninput={(event) =>
            (settings = {
              ...settings!,
              playerArgs: event.currentTarget.value.trim().split(/\s+/).filter(Boolean),
            })}
          onchange={(event) =>
            persist({
              playerArgs: event.currentTarget.value.trim().split(/\s+/).filter(Boolean),
            })}
          placeholder="e.g. run io.mpv.Mpv"
          class="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm text-ink placeholder:text-ink-faint focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        />
        <p class="mt-1 text-[11px] text-ink-faint">
          The URL is appended after these. Not shell-parsed — each word is one
          argument.
        </p>
      </section>

      <!-- Appearance -->
      <section class="mb-6">
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-ink-faint uppercase">
          Appearance
        </h3>
        <div class="flex gap-2" role="group" aria-label="Theme">
          {#each ["system", "light", "dark"] as const as choice (choice)}
            <button
              type="button"
              onclick={() => chooseTheme(choice)}
              aria-pressed={themeChoice(settings.theme) === choice}
              class="flex-1 rounded-lg border px-3 py-2 text-xs font-medium capitalize transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {themeChoice(
                settings.theme,
              ) === choice
                ? 'border-accent bg-accent/15 text-accent'
                : 'border-border-subtle text-ink-muted hover:border-accent hover:text-ink'}"
            >
              {choice}
            </button>
          {/each}
        </div>
      </section>

      <!-- Playback -->
      <section class="mb-6">
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-ink-faint uppercase">
          Playback
        </h3>

        <p class="mb-1 block text-xs text-ink-muted">Download folder</p>
        <div class="mb-3 flex items-center gap-2">
          <span class="min-w-0 flex-1 truncate rounded-lg border border-border-subtle bg-surface px-3 py-2 text-xs text-ink-muted">
            {settings.downloadDir ?? "System default"}
          </span>
          <button
            type="button"
            onclick={pickDownloadDir}
            class="shrink-0 rounded-lg border border-border-subtle px-3 py-1.5 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            Browse…
          </button>
          {#if settings.downloadDir}
            <button
              type="button"
              onclick={() => persist({ downloadDir: null })}
              class="shrink-0 rounded-lg border border-border-subtle px-3 py-1.5 text-xs text-ink-muted transition-colors hover:border-danger hover:text-danger focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              Reset
            </button>
          {/if}
        </div>

        <label for="settings-fraction" class="mb-1 block text-xs text-ink-muted">
          Buffer before playing: {Math.round(settings.readyFraction * 100)}%
        </label>
        <input
          id="settings-fraction"
          type="range"
          min="1"
          max="50"
          value={Math.round(settings.readyFraction * 100)}
          onchange={(event) =>
            persist({ readyFraction: Number(event.currentTarget.value) / 100 })}
          class="w-full accent-accent"
        />
      </section>

      <!-- Release preference -->
      <section>
        <h3 class="mb-2 text-xs font-semibold tracking-wide text-ink-faint uppercase">
          Releases
        </h3>
        <p class="mb-1 block text-xs text-ink-muted">
          Preferred resolutions (in order)
        </p>
        <div class="mb-3 flex flex-wrap gap-1.5">
          {#if settings.preferredResolutions.length === 0}
            <span class="text-xs text-ink-faint">No preference</span>
          {/if}
          {#each settings.preferredResolutions as resolution (resolution)}
            <button
              type="button"
              onclick={() => toggleResolution(resolution)}
              class="rounded-full border border-accent bg-accent/15 px-3 py-1 text-xs text-accent transition-colors hover:border-danger hover:text-danger focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              {resolution} ✕
            </button>
          {/each}
        </div>
        <div class="flex gap-2">
          <div class="flex-1">
            <Select
              label="Add preferred resolution"
              value={pendingResolution}
              options={resolutionChoices}
              onchange={(next) => (pendingResolution = next as Resolution)}
            />
          </div>
          <button
            type="button"
            onclick={() => toggleResolution(pendingResolution)}
            class="rounded-lg border border-border-subtle px-4 py-2 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            Add
          </button>
        </div>

        <label for="settings-seeders" class="mt-3 mb-1 block text-xs text-ink-muted">
          Minimum seeders
        </label>
        <input
          id="settings-seeders"
          type="number"
          min="0"
          value={settings.minSeeders}
          onchange={(event) =>
            persist({ minSeeders: Math.max(0, Number(event.currentTarget.value)) })}
          class="w-24 rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        />
      </section>

      <!-- Diagnostics -->
      <DiagnosticsPanel />

      {#if error}
        <p class="mt-4 text-xs text-danger" role="status">{error}</p>
      {/if}
    {/if}
  </div>
</Modal>