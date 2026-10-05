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

  const resolutionChoices: SelectOption[] = RESOLUTION_ORDER.map(
    (resolution) => ({ value: resolution, label: resolution }),
  );

  let { open = false, onClose }: { open?: boolean; onClose: () => void } =
    $props();

  let settings = $state<Settings | null>(null);
  let signedIn = $state(false);
  let players = $state<string[]>([]);
  let error = $state<string | null>(null);
  let saving = $state(false);
  let pendingResolution = $state<Resolution>("1080p");

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
    const unlisten = onAuthChanged((next) => {
      signedIn = next;
    });
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

  async function persist(patch: Partial<Settings>): Promise<void> {
    if (settings === null) return;
    const next = { ...settings, ...patch };
    settings = next;
    saving = true;
    error = null;
    try {
      settings = await setSettings(next);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      saving = false;
    }
  }

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
  <!-- Header -->
  <header class="flex items-center justify-between gap-3 border-b border-border-subtle p-5">
    <div class="flex items-center gap-2.5 min-w-0">
      <h2 class="text-lg font-bold tracking-tight text-ink">Settings</h2>
      {#if saving}
        <span class="inline-flex items-center gap-1.5 rounded-full bg-accent/10 px-2 py-0.5 text-[10px] font-medium text-accent">
          <span class="size-1.5 animate-pulse rounded-full bg-accent"></span>
          Saving…
        </span>
      {/if}
    </div>
    <button
      type="button"
      onclick={onClose}
      aria-label="Close settings"
      class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-border-subtle bg-surface-hover text-ink-muted transition-colors hover:border-accent hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      <svg class="size-4 fill-none stroke-current stroke-2" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </header>

  <!-- Body -->
  <div class="min-h-0 flex-1 overflow-y-auto p-5 space-y-6">
    {#if settings === null}
      <div class="py-12 text-center">
        <div class="mx-auto size-6 animate-spin rounded-full border-2 border-border-subtle border-t-accent"></div>
        <p class="mt-3 text-xs text-ink-muted">Loading preferences…</p>
      </div>
    {:else}
      <!-- Account -->
      <section class="rounded-xl border border-border-subtle bg-surface-raised p-4">
        <h3 class="mb-3 text-xs font-bold tracking-wider text-ink-faint uppercase">
          AniList Account
        </h3>
        <div class="flex items-center justify-between gap-3 rounded-lg border border-border-subtle bg-surface p-3">
          <div class="flex items-center gap-2.5 min-w-0">
            <span class="size-2 shrink-0 rounded-full {signedIn ? 'bg-health-green' : 'bg-ink-faint'}"></span>
            <span class="truncate text-xs font-medium text-ink">
              {signedIn ? "Connected to AniList" : "Not connected"}
            </span>
          </div>
          {#if signedIn}
            <button
              type="button"
              onclick={disconnect}
              class="shrink-0 rounded-lg border border-border-subtle bg-surface px-3 py-1.5 text-xs font-medium text-ink-muted transition-colors hover:border-danger hover:text-danger focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              Disconnect
            </button>
          {:else}
            <button
              type="button"
              onclick={connect}
              class="shrink-0 rounded-lg bg-accent px-3 py-1.5 text-xs font-semibold text-white shadow-sm transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              Connect
            </button>
          {/if}
        </div>
      </section>

      <!-- Player Options -->
      <section class="rounded-xl border border-border-subtle bg-surface-raised p-4 space-y-3">
        <h3 class="text-xs font-bold tracking-wider text-ink-faint uppercase">
          Player Configuration
        </h3>

        <div>
          <label for="settings-player" class="mb-1 block text-xs font-medium text-ink">
            Executable / Command
          </label>
          <input
            id="settings-player"
            list="player-suggestions"
            value={settings.player}
            oninput={(event) =>
              (settings = { ...settings!, player: event.currentTarget.value })}
            onchange={(event) => persist({ player: event.currentTarget.value })}
            placeholder="e.g. mpv, vlc"
            class="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-xs text-ink placeholder:text-ink-faint focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20"
          />
          <datalist id="player-suggestions">
            {#each players as name (name)}
              <option value={name}></option>
            {/each}
          </datalist>
        </div>

        <div>
          <label for="settings-player-args" class="mb-1 block text-xs font-medium text-ink">
            Extra Arguments
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
            placeholder="e.g. --fs --force-media-title"
            class="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-xs text-ink placeholder:text-ink-faint focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20"
          />
          <p class="mt-1 text-[11px] text-ink-faint">
            Passed directly to player before stream URL.
          </p>
        </div>

        <!-- Switch Button for Prompting Player -->
        <div class="pt-2 border-t border-border-subtle/50">
          <button
            type="button"
            role="switch"
            aria-checked={settings.askEveryTime}
            onclick={() =>
              settings && persist({ askEveryTime: !settings.askEveryTime })}
            class="flex items-center justify-between gap-3 w-full text-left group cursor-pointer"
          >
            <span class="text-xs font-medium text-ink">
              Always prompt for media player choice before streaming
            </span>
            <span
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {settings.askEveryTime
                ? 'bg-accent'
                : 'bg-border-subtle'}"
            >
              <span
                class="pointer-events-none inline-block size-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out {settings.askEveryTime
                  ? 'translate-x-4'
                  : 'translate-x-0'}"
              ></span>
            </span>
          </button>
        </div>
      </section>

      <!-- Appearance -->
      <section class="rounded-xl border border-border-subtle bg-surface-raised p-4">
        <h3 class="mb-3 text-xs font-bold tracking-wider text-ink-faint uppercase">
          Appearance
        </h3>
        <div class="grid grid-cols-3 gap-2" role="group" aria-label="Theme selection">
          {#each ["system", "light", "dark"] as const as choice (choice)}
            {@const isSelected = themeChoice(settings.theme) === choice}
            <button
              type="button"
              onclick={() => chooseTheme(choice)}
              aria-pressed={isSelected}
              class="rounded-lg border px-3 py-2 text-xs font-semibold capitalize transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {isSelected
                ? 'border-accent bg-accent/10 text-accent ring-1 ring-accent/30'
                : 'border-border-subtle bg-surface text-ink-muted hover:border-accent/60 hover:text-ink'}"
            >
              {choice}
            </button>
          {/each}
        </div>
      </section>

      <!-- Playback -->
      <section class="rounded-xl border border-border-subtle bg-surface-raised p-4 space-y-4">
        <h3 class="text-xs font-bold tracking-wider text-ink-faint uppercase">
          Playback & Downloads
        </h3>

        <div>
          <span class="mb-1 block text-xs font-medium text-ink">Download Directory</span>
          <div class="flex items-center gap-2">
            <span class="min-w-0 flex-1 truncate rounded-lg border border-border-subtle bg-surface px-3 py-2 text-xs text-ink-muted">
              {settings.downloadDir ?? "Default Downloads Directory"}
            </span>
            <button
              type="button"
              onclick={pickDownloadDir}
              class="shrink-0 rounded-lg border border-border-subtle bg-surface px-3 py-1.5 text-xs font-medium text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              Browse…
            </button>
            {#if settings.downloadDir}
              <button
                type="button"
                onclick={() => persist({ downloadDir: null })}
                class="shrink-0 rounded-lg border border-border-subtle bg-surface px-3 py-1.5 text-xs font-medium text-ink-muted transition-colors hover:border-danger hover:text-danger focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
              >
                Reset
              </button>
            {/if}
          </div>
        </div>

        <div>
          <div class="flex items-center justify-between mb-1">
            <label for="settings-fraction" class="text-xs font-medium text-ink">
              Playback Buffer Threshold
            </label>
            <span class="text-xs font-bold text-accent">
              {Math.round(settings.readyFraction * 100)}%
            </span>
          </div>
          <input
            id="settings-fraction"
            type="range"
            min="1"
            max="50"
            value={Math.round(settings.readyFraction * 100)}
            onchange={(event) =>
              persist({ readyFraction: Number(event.currentTarget.value) / 100 })}
            class="w-full accent-accent cursor-pointer"
          />
        </div>
      </section>

      <!-- Releases & Filtering -->
      <section class="rounded-xl border border-border-subtle bg-surface-raised p-4 space-y-3">
        <h3 class="text-xs font-bold tracking-wider text-ink-faint uppercase">
          Release Preferences
        </h3>

        <div>
          <span class="mb-1.5 block text-xs font-medium text-ink">
            Preferred Resolutions (Prioritized)
          </span>
          <div class="mb-3 flex flex-wrap gap-1.5">
            {#if settings.preferredResolutions.length === 0}
              <span class="text-xs text-ink-faint italic">No resolution filters set</span>
            {/if}
            {#each settings.preferredResolutions as resolution (resolution)}
              <button
                type="button"
                onclick={() => toggleResolution(resolution)}
                title="Remove {resolution}"
                class="group flex items-center gap-1.5 rounded-full border border-accent bg-accent/10 px-3 py-0.5 text-xs font-semibold text-accent transition-colors hover:border-danger hover:bg-danger/10 hover:text-danger focus:outline-none"
              >
                <span>{resolution}</span>
                <svg class="size-3 stroke-current stroke-2 fill-none" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            {/each}
          </div>

          <div class="flex gap-2">
            <div class="flex-1 min-w-0">
              <Select
                label="Add resolution"
                value={pendingResolution}
                options={resolutionChoices}
                onchange={(next) => (pendingResolution = next as Resolution)}
              />
            </div>
            <button
              type="button"
              onclick={() => toggleResolution(pendingResolution)}
              class="shrink-0 rounded-lg border border-border-subtle bg-surface px-4 py-2 text-xs font-semibold text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              Add
            </button>
          </div>
        </div>

        <div class="pt-2">
          <label for="settings-seeders" class="mb-1 block text-xs font-medium text-ink">
            Minimum Seeders Count
          </label>
          <input
            id="settings-seeders"
            type="number"
            min="0"
            value={settings.minSeeders}
            onchange={(event) =>
              persist({ minSeeders: Math.max(0, Number(event.currentTarget.value)) })}
            class="w-28 rounded-lg border border-border-subtle bg-surface px-3 py-1.5 text-xs text-ink focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20"
          />
        </div>
      </section>

      <!-- Diagnostics -->
      <section class="rounded-xl border border-border-subtle bg-surface-raised p-4">
        <DiagnosticsPanel />
      </section>

      {#if error}
        <div class="rounded-xl border border-danger/30 bg-danger/5 p-3 text-center">
          <p class="text-xs font-medium text-danger" role="status">{error}</p>
        </div>
      {/if}
    {/if}
  </div>

  <!-- Footer -->
  {#snippet footer()}
    <div class="flex justify-end border-t border-border-subtle bg-surface-raised px-5 py-3.5">
      <button
        type="button"
        onclick={onClose}
        class="rounded-lg bg-accent px-4 py-1.5 text-xs font-semibold text-white shadow-sm transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        Done
      </button>
    </div>
  {/snippet}
</Modal>