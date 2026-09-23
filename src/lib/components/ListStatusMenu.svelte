<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import {
    authStatus,
    beginLogin,
    getListEntry,
    onAuthChanged,
    setListEntry,
  } from "$lib/api/auth";
  import type { ListStatus } from "$lib/types";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  let { mediaId }: { mediaId: number } = $props();

  /**
   * The five lists, in the order they are offered.
   *
   * The labels are the reader's words ("Watching"), the values are AniList's
   * own enum members -- the same wire format the backend passes through, so no
   * translation happens here. "On hold" and "Plan to watch" are AniList's own
   * phrasings for PAUSED and PLANNING.
   */
  const STATUSES: { value: ListStatus; label: string }[] = [
    { value: "current", label: "Watching" },
    { value: "completed", label: "Completed" },
    { value: "paused", label: "On hold" },
    { value: "dropped", label: "Dropped" },
    { value: "planning", label: "Plan to watch" },
  ];

  /**
   * Whether a token is stored.
   *
   * `null` until the first check resolves, so the control can hold its shape
   * rather than flash "Sign in" at a reader who is already signed in.
   */
  let signedIn = $state<boolean | null>(null);

  /** The current status, or `null` when the work is not on the reader's list. */
  let status = $state<ListStatus | null>(null);

  let open = $state(false);
  let busy = $state(false);
  let container = $state<HTMLElement | null>(null);

  /** A short-lived message when a write fails, so a click is never silent. */
  let error = $state<string | null>(null);

  let unlisten: UnlistenFn | null = null;

  /** Fetch the entry for the current work, if signed in. */
  async function loadEntry() {
    try {
      const entry = await getListEntry(mediaId);
      status = entry?.status ?? null;
    } catch {
      // A failed read is not worth an error message: it only means the menu
      // opens showing no selection, and the write path reports its own errors.
      status = null;
    }
  }

  onMount(() => {
    let cancelled = false;

    authStatus()
      .then((result) => {
        if (cancelled) return;
        signedIn = result;
        if (result) void loadEntry();
      })
      .catch(() => {
        if (cancelled) return;
        signedIn = false;
      });

    // Sign-in can complete while this page is open (the redirect lands back in
    // the app), so react rather than requiring a reload. The unlisten promise
    // resolves after mount; guard against unmounting first.
    onAuthChanged((nowSignedIn) => {
      signedIn = nowSignedIn;
      if (nowSignedIn) void loadEntry();
      else status = null;
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Without a listener the control still works on a reload; not fatal.
      });

    return () => {
      cancelled = true;
    };
  });

  onDestroy(() => {
    unlisten?.();
  });

  /** Close the menu when a click lands outside the control. */
  function onWindowClick(event: MouseEvent) {
    if (!open || container === null) return;

    const target = event.target as Node | null;
    if (target !== null && !container.contains(target)) {
      open = false;
    }
  }

  /** Start sign-in: get the authorize URL, then hand it to the system browser. */
  async function signIn() {
    try {
      const url = await beginLogin();
      await openUrl(url);
    } catch {
      error = "Could not open the sign-in page.";
    }
  }

  /** Move the work to `next`, updating the button optimistically. */
  async function choose(next: ListStatus) {
    open = false;
    error = null;

    const previous = status;
    status = next;
    busy = true;

    try {
      // Progress is deliberately omitted: changing the list must not reset how
      // far the reader has got.
      await setListEntry(mediaId, next);
    } catch {
      status = previous;
      error = "Could not update your list.";
    } finally {
      busy = false;
    }
  }

  /** The button's label: the current status, or a prompt when off the list. */
  const buttonLabel = $derived(
    status === null
      ? "Add to list"
      : (STATUSES.find((s) => s.value === status)?.label ?? "Add to list"),
  );

  const buttonClass =
    "flex shrink-0 items-center gap-2 rounded-full border border-border-subtle bg-surface-hover px-4 py-2 text-sm text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60";
</script>

<svelte:window onclick={onWindowClick} />

<div bind:this={container} class="relative inline-block">
  {#if signedIn === false}
    <!-- No menu for a reader with no account: there is nothing to select, and
         the useful action is the one that creates the account link. -->
    <button
      type="button"
      data-testid="list-status-signin"
      onclick={signIn}
      class={buttonClass}
    >
      <span aria-hidden="true">＋</span>
      Sign in to AniList
    </button>
  {:else if signedIn === true}
    <button
      type="button"
      data-testid="list-status-button"
      aria-expanded={open}
      aria-controls="list-status-menu"
      disabled={busy}
      onclick={() => (open = !open)}
      class={buttonClass}
    >
      <span aria-hidden="true">＋</span>
      {buttonLabel}
    </button>

    {#if open}
      <!-- z-30 clears the navbar, matching FilterBar's dropdowns. -->
      <div
        id="list-status-menu"
        data-testid="list-status-menu"
        class="absolute left-0 top-full z-30 mt-2 w-48"
      >
        <ul class="rounded-xl bg-surface py-2 shadow-xl">
          {#each STATUSES as { value, label } (value)}
            <li>
              <button
                type="button"
                data-testid="list-status-option-{value}"
                aria-current={status === value ? "true" : undefined}
                onclick={() => choose(value)}
                class="block w-full px-4 py-1.5 text-left text-sm transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {status ===
                value
                  ? 'text-accent'
                  : 'text-ink'}"
              >
                {label}
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if error}
      <p data-testid="list-status-error" class="mt-1 text-xs text-red-400">
        {error}
      </p>
    {/if}
  {/if}
</div>