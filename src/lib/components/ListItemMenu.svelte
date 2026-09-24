<script lang="ts">
  import { deleteListEntry, setListEntry } from "$lib/api/auth";
  import { chooseMenuAlign, MENU_WIDTH, type MenuAlign } from "$lib/menu-position";
  import type { ListStatus, UserListEntry } from "$lib/types";

  let {
    entry,
    onchange,
    onremove,
  }: {
    /** The row this menu belongs to. */
    entry: UserListEntry;
    /** Called after a status change succeeds, with the new status. */
    onchange: (status: ListStatus) => void;
    /** Called after the entry is removed. */
    onremove: () => void;
  } = $props();

  /**
   * The five lists, in the order they are offered.
   *
   * Same labels and values as `ListStatusMenu`, so the two controls agree. The
   * menu opens on the current status via `aria-current`.
   */
  const STATUSES: { value: ListStatus; label: string }[] = [
    { value: "current", label: "Watching" },
    { value: "completed", label: "Completed" },
    { value: "paused", label: "On hold" },
    { value: "dropped", label: "Dropped" },
    { value: "planning", label: "Plan to watch" },
  ];

  let open = $state(false);
  let busy = $state(false);
  let container = $state<HTMLElement | null>(null);
  let button = $state<HTMLElement | null>(null);

  /**
   * Which way the menu opens, decided when it is opened.
   *
   * A card in the last column would otherwise push a right-opening menu off the
   * screen. Decided once at open time rather than reactively: the menu is
   * short-lived and the reader does not resize mid-selection.
   */
  let align = $state<MenuAlign>("start");

  /** Close the menu when a click lands outside the control. */
  function onWindowClick(event: MouseEvent) {
    if (!open || container === null) return;

    const target = event.target as Node | null;
    if (target !== null && !container.contains(target)) {
      open = false;
    }
  }

  /**
   * Open or close, choosing the alignment at the moment it opens.
   *
   * Measured on open rather than kept reactive: the reader is not resizing
   * mid-selection, and a single read is cheaper than tracking the viewport.
   */
  function toggle() {
    if (open) {
      open = false;
      return;
    }

    // `getBoundingClientRect` is absent-but-safe to call in jsdom, returning
    // zeros -- which resolves to "start", the default, so tests are unaffected.
    const left = button?.getBoundingClientRect().left ?? 0;
    align = chooseMenuAlign(left, window.innerWidth, MENU_WIDTH);
    open = true;
  }

  /** Move the work to `next`, then tell the page so it can re-group. */
  async function choose(next: ListStatus) {
    open = false;
    if (busy || next === entry.status) return;

    busy = true;
    try {
      // Progress is deliberately omitted: changing the list must not reset how
      // far the reader has got.
      await setListEntry(entry.anime.id, next);
      onchange(next);
    } catch {
      // The page owns the error surface; a failed move leaves the row as it
      // was, which is honest without a popup.
    } finally {
      busy = false;
    }
  }

  /** Remove the entry from the reader's list. */
  async function remove() {
    open = false;
    if (busy) return;

    busy = true;
    try {
      await deleteListEntry(entry.entryId);
      onremove();
    } catch {
      // As above: the row stays, so the reader can try again.
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onclick={onWindowClick} />

<!-- Top-LEFT, not top-right: `AnimeCard` already puts its score pill at
     `top-2 right-2`, and two controls in one corner would collide. -->
<div bind:this={container} class="absolute top-1 left-1">
  <button
    type="button"
    bind:this={button}
    data-testid="item-menu-button"
    aria-label="List options"
    aria-expanded={open}
    aria-controls="item-menu"
    disabled={busy}
    onclick={toggle}
    class="flex h-7 w-7 items-center justify-center rounded-full bg-surface/85 text-ink shadow transition-colors hover:bg-surface-hover hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
  >
    <span aria-hidden="true">⋮</span>
  </button>

  {#if open}
    <div
      id="item-menu"
      data-testid="item-menu"
      class="absolute top-full z-30 mt-1 w-44 {align === 'start'
        ? 'left-0'
        : 'right-0'}"
    >
      <ul class="rounded-xl bg-surface py-2 shadow-xl">
        {#each STATUSES as { value, label } (value)}
          <li>
            <button
              type="button"
              data-testid="item-status-{value}"
              aria-current={entry.status === value ? "true" : undefined}
              onclick={() => choose(value)}
              class="block w-full px-4 py-1.5 text-left text-sm transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {entry.status ===
              value
                ? 'text-accent'
                : 'text-ink'}"
            >
              {label}
            </button>
          </li>
        {/each}

        <li class="mt-1 border-t border-border-subtle pt-1">
          <button
            type="button"
            data-testid="item-remove"
            onclick={remove}
            class="block w-full px-4 py-1.5 text-left text-sm text-red-400 transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            Remove from list
          </button>
        </li>
      </ul>
    </div>
  {/if}
</div>