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

  /**
   * The tabs, in the order they are shown.
   *
   * "All" is not a status: it is the absence of a filter. It leads because a
   * combined view is the most useful default, and it has no `value`.
   */
  const TABS: { value: ListStatus | null; label: string }[] = [
    { value: null, label: "All" },
    { value: "current", label: "Watching" },
    { value: "paused", label: "On-Hold" },
    { value: "planning", label: "Plan to Watch" },
    { value: "dropped", label: "Dropped" },
    { value: "completed", label: "Completed" },
  ];

  let signedIn = $state<boolean | null>(null);
  let entries = $state<UserListEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let activeTab = $state<ListStatus | null>(null);

  let unlisten: UnlistenFn | null = null;

  /** The entries for the active tab. "All" keeps every status. */
  const visible = $derived(
    activeTab === null
      ? entries
      : entries.filter((entry) => entry.status === activeTab),
  );

  /** How many entries each tab holds, for its count badge. */
  function countFor(status: ListStatus | null): number {
    return status === null
      ? entries.length
      : entries.filter((entry) => entry.status === status).length;
  }

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

    // Sign-in can complete while this page is open, so react rather than
    // requiring a reload.
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
      .catch(() => {
        // The page still works on a reload without the listener.
      });

    return () => {
      cancelled = true;
    };
  });

  onDestroy(() => {
    unlisten?.();
  });

  /** Start sign-in: get the authorize URL, then open it in the browser. */
  async function signIn() {
    try {
      const url = await beginLogin();
      await openUrl(url);
    } catch {
      error = "Could not open the sign-in page.";
    }
  }

  /** A status was changed from a card's menu: update the local copy. */
  function onStatusChange(animeId: number, status: ListStatus) {
    entries = entries.map((entry) =>
      entry.anime.id === animeId ? { ...entry, status } : entry,
    );
  }

  /** An entry was removed from a card's menu: drop it from the local copy. */
  function onRemove(animeId: number) {
    entries = entries.filter((entry) => entry.anime.id !== animeId);
  }
</script>

<h1 class="mb-4 text-lg font-semibold tracking-tight">My List</h1>

{#if signedIn === false}
  <div class="py-16 text-center">
    <p class="text-ink">Sign in to AniList to see your list.</p>
    <button
      type="button"
      data-testid="list-signin"
      onclick={signIn}
      class="mt-4 rounded-full bg-surface-hover px-4 py-2 text-sm text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      Sign in to AniList
    </button>
  </div>
{:else}
  <!-- Tabs stay visible while loading so the page keeps its shape. -->
  <div
    class="flex flex-wrap items-center gap-2"
    role="tablist"
    data-testid="list-tabs"
  >
    {#each TABS as { value, label } (label)}
      <button
        type="button"
        role="tab"
        data-testid="list-tab-{value ?? 'all'}"
        aria-selected={activeTab === value}
        onclick={() => (activeTab = value)}
        class="rounded-full px-4 py-2 text-sm transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {activeTab ===
        value
          ? 'bg-accent text-white'
          : 'bg-surface-hover text-ink hover:text-accent'}"
      >
        {label}
        {#if signedIn === true && !loading && countFor(value) > 0}
          <span class="ml-1 text-xs opacity-70">{countFor(value)}</span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="mt-8">
    {#if loading}
      <AnimeGridSkeleton count={12} />
    {:else if error}
      <div class="py-16 text-center">
        <p class="text-ink">Could not load your list.</p>
        <p class="mt-2 text-sm text-ink-faint">{error}</p>
        <button
          type="button"
          onclick={load}
          class="mt-4 rounded-full bg-surface-hover px-4 py-2 text-sm text-ink hover:text-ink-muted"
        >
          Try again
        </button>
      </div>
    {:else if visible.length === 0}
      <p class="py-16 text-center text-ink-muted" data-testid="list-empty">
        {activeTab === null
          ? "Nothing on your list yet."
          : "Nothing in this list."}
      </p>
    {:else}
      <ul class="flex flex-wrap gap-4" data-testid="list-grid">
        {#each visible as entry (entry.anime.id)}
          <li class="relative">
            <AnimeCard anime={entry.anime} />
            <ListItemMenu
              {entry}
              onchange={(status) => onStatusChange(entry.anime.id, status)}
              onremove={() => onRemove(entry.anime.id)}
            />
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}