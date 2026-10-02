<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { errorMessage } from "$lib/api/anime";
  import {
    authStatus,
    beginLogin,
    getUserList,
    onAuthChanged,
  } from "$lib/api/auth";
  import type { ListStatus, UserListEntry } from "$lib/types";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import AnimeCard from "$lib/components/AnimeCard.svelte";
  import AnimeGridSkeleton from "$lib/components/AnimeGridSkeleton.svelte";
  import ListItemMenu from "$lib/components/ListItemMenu.svelte";

  /** Tabs configuration with explicit ordering and display labels */
  const TABS: { value: ListStatus | null; label: string }[] = [
    { value: null, label: "All" },
    { value: "current", label: "Watching" },
    { value: "planning", label: "Plan to Watch" },
    { value: "completed", label: "Completed" },
    { value: "paused", label: "On-Hold" },
    { value: "dropped", label: "Dropped" },
  ];

  let signedIn = $state<boolean | null>(null);
  let entries = $state<UserListEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let activeTab = $state<ListStatus | null>(null);

  let unlisten: UnlistenFn | null = null;

  /** Visible entries for active tab */
  const visible = $derived(
    activeTab === null
      ? entries
      : entries.filter((entry) => entry.status === activeTab),
  );

  /** Calculate tab count cache efficiently in a single pass */
  const counts = $derived.by(() => {
    const map = new Map<ListStatus | null, number>();
    map.set(null, entries.length);

    for (const entry of entries) {
      map.set(entry.status, (map.get(entry.status) ?? 0) + 1);
    }
    return map;
  });

  async function load() {
    loading = true;
    error = null;
    try {
      entries = await getUserList();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    let cancelled = false;

    authStatus()
      .then((result) => {
        if (cancelled) return;
        signedIn = result;
        if (result) void load();
        else loading = false;
      })
      .catch(() => {
        if (cancelled) return;
        signedIn = false;
        loading = false;
      });

    onAuthChanged((nowSignedIn) => {
      signedIn = nowSignedIn;
      if (nowSignedIn) void load();
      else {
        entries = [];
        loading = false;
      }
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  });

  onDestroy(() => {
    unlisten?.();
  });

  async function signIn() {
    try {
      const url = await beginLogin();
      await openUrl(url);
    } catch {
      error = "Could not open the sign-in page.";
    }
  }

  function onStatusChange(animeId: number, status: ListStatus) {
    entries = entries.map((entry) =>
      entry.anime.id === animeId ? { ...entry, status } : entry,
    );
  }

  function onRemove(animeId: number) {
    entries = entries.filter((entry) => entry.anime.id !== animeId);
  }
</script>

<div class="space-y-6">
  <!-- Header Section -->
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-ink">My List</h1>
      <p class="text-xs text-ink-muted">Manage your anime library and track progress</p>
    </div>
  </div>

  {#if signedIn === false}
    <!-- Signed Out Empty Banner -->
    <div class="flex flex-col items-center justify-center rounded-2xl border border-border-subtle/80 bg-surface-raised/50 py-16 px-4 text-center backdrop-blur-sm">
      <div class="flex h-12 w-12 items-center justify-center rounded-full bg-accent/10 text-accent mb-3">
        <svg class="h-6 w-6 stroke-current fill-none stroke-2" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" />
        </svg>
      </div>
      <h2 class="text-base font-bold text-ink">Sign in to sync your list</h2>
      <p class="mt-1 text-xs text-ink-muted max-w-sm">Connect your AniList account to track episodes, sync watch states, and organize your collection.</p>
      
      <button
        type="button"
        data-testid="list-signin"
        onclick={signIn}
        class="mt-5 inline-flex items-center gap-2 rounded-xl bg-accent px-5 py-2.5 text-xs font-bold text-white shadow-md transition-all hover:bg-accent-hover hover:shadow-accent/30 active:scale-95 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        Sign in with AniList
      </button>
    </div>
  {:else}
    <!-- Filter Navigation Tabs -->
    <div
      class="no-scrollbar flex items-center gap-1.5 overflow-x-auto pb-1"
      role="tablist"
      data-testid="list-tabs"
    >
      {#each TABS as { value, label } (label)}
        {@const count = counts.get(value) ?? 0}
        <button
          type="button"
          role="tab"
          data-testid="list-tab-{value ?? 'all'}"
          aria-selected={activeTab === value}
          onclick={() => (activeTab = value)}
          class="flex shrink-0 items-center gap-2 rounded-xl px-3.5 py-2 text-xs font-bold transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {activeTab ===
          value
            ? 'bg-accent text-white shadow-md shadow-accent/20'
            : 'border border-border-subtle/60 bg-surface-raised/80 text-ink-muted hover:border-accent/40 hover:text-ink'}"
        >
          <span>{label}</span>
          {#if signedIn === true && !loading}
            <span
              class="rounded-md px-1.5 py-0.5 text-[10px] font-extrabold transition-colors {activeTab === value
                ? 'bg-white/20 text-white'
                : 'bg-surface-hover text-ink-faint'}"
            >
              {count}
            </span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- Content Grid Area -->
    <div class="mt-4">
      {#if loading}
        <AnimeGridSkeleton count={10} />
      {:else if error}
        <!-- Error State -->
        <div class="flex flex-col items-center justify-center rounded-2xl border border-border-subtle/80 bg-surface-raised/50 py-16 text-center">
          <p class="text-sm font-bold text-ink">Could not load your list</p>
          <p class="mt-1 text-xs text-ink-faint max-w-sm">{error}</p>
          <button
            type="button"
            onclick={load}
            class="mt-4 rounded-xl border border-border-subtle bg-surface-hover px-4 py-2 text-xs font-bold text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            Try again
          </button>
        </div>
      {:else if visible.length === 0}
        <!-- Empty State -->
        <div class="flex flex-col items-center justify-center rounded-2xl border border-border-subtle/60 bg-surface-raised/30 py-16 text-center" data-testid="list-empty">
          <p class="text-sm font-bold text-ink">No titles found</p>
          <p class="mt-1 text-xs text-ink-muted">
            {activeTab === null
              ? "You haven't added any anime to your list yet."
              : `You have no anime marked as "${TABS.find(t => t.value === activeTab)?.label}".`}
          </p>
        </div>
      {:else}
        <!-- Anime List Grid -->
        <ul class="flex flex-wrap gap-4" data-testid="list-grid">
          {#each visible as entry (entry.anime.id)}
            <li class="group/item relative">
              <AnimeCard anime={entry.anime} />
              
              <!-- Floating Context Menu Button -->
              <div class="absolute top-2 left-2 z-30">
                <ListItemMenu
                  {entry}
                  onchange={(status) => onStatusChange(entry.anime.id, status)}
                  onremove={() => onRemove(entry.anime.id)}
                />
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>