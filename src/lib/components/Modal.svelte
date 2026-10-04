<script lang="ts">
  import type { Snippet } from "svelte";

  import { portal } from "$lib/portal";

  /**
   * A generic overlay dialog.
   *
   * Owns the things every modal must get right, so no caller has to: it dims
   * and blurs the page behind, traps focus inside, restores focus to whatever
   * was focused before, closes on Escape or a backdrop click, and locks the
   * page behind from scrolling.
   *
   * The content is supplied as snippets rather than being built in, so the same
   * shell fits the trailer viewer, the episode picker and anything after it.
   */

  let {
    open,
    onClose,
    label,
    children,
    footer,
  }: {
    /** Whether the dialog is showing. */
    open: boolean;
    /**
     * Called when the reader asks to close: Escape, the backdrop, or a control
     * the content wires to it. The caller owns the boolean, so this never sets
     * `open` itself -- a dialog that closed itself could not be reopened by the
     * same state that opened it.
     */
    onClose: () => void;
    /** Accessible name for the dialog, read by screen readers. */
    label: string;
    children: Snippet;
    /** Optional persistent footer, e.g. the always-available actions. */
    footer?: Snippet;
  } = $props();

  /** The dialog element, for trapping focus inside it. */
  let dialogEl: HTMLElement | null = $state(null);
  /** What had focus before opening, so it can be restored on close. */
  let previouslyFocused: HTMLElement | null = null;

  /**
   * Elements focusable by keyboard, in DOM order.
   *
   * The selector is the standard set; a disabled control is skipped because it
   * cannot take focus, and `[hidden]` because a hidden control should not trap
   * the tab key on nothing.
   */
  function focusable(): HTMLElement[] {
    if (dialogEl === null) return [];
    return Array.from(
      dialogEl.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((el) => !el.hidden);
  }

  /**
   * Wrap the tab key inside the dialog.
   *
   * Without this, tabbing past the last control moves focus into the page
   * behind the overlay, which the reader cannot see -- the classic modal bug.
   */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }

    if (event.key !== "Tab") return;

    const items = focusable();
    if (items.length === 0) {
      // Nothing focusable: keep focus on the dialog itself rather than letting
      // it escape to the page behind.
      event.preventDefault();
      dialogEl?.focus();
      return;
    }

    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (event.shiftKey && (active === first || active === dialogEl)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  // Open/close side effects: lock the page scroll, move focus in on open and
  // restore it on close. Keyed on `open` so it runs on each transition.
  $effect(() => {
    if (!open) return;

    previouslyFocused = document.activeElement as HTMLElement | null;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    // Focus the first real control if there is one, else the dialog itself.
    // Deferred a frame so the content has mounted -- a focus call before paint
    // would land on nothing.
    requestAnimationFrame(() => {
      const items = focusable();
      (items[0] ?? dialogEl)?.focus();
    });

    return () => {
      document.body.style.overflow = previousOverflow;
      // Restore focus to whatever opened the dialog, so a keyboard reader
      // continues from where they were rather than the top of the page.
      previouslyFocused?.focus?.();
    };
  });
</script>

{#if open}
  <!-- The backdrop is a click target, not interactive content: it exists only
       to catch an outside click, so it carries a presentation role and the
       keyboard path is Escape, handled on the dialog. -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    use:portal
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/72 p-4 backdrop-blur-md"
    role="presentation"
    data-testid="modal-backdrop"
    onclick={(event) => {
      // Only a click on the backdrop itself closes: a click that started inside
      // the dialog and drifted out still targets the dialog, so this compares
      // the target rather than listening for a bubbling click alone.
      if (event.target === event.currentTarget) onClose();
    }}
  >
    <div
      bind:this={dialogEl}
      role="dialog"
      aria-modal="true"
      aria-label={label}
      tabindex="-1"
      data-testid="modal-dialog"
      class="flex max-h-[min(88vh,820px)] w-full max-w-230 flex-col overflow-hidden rounded-2xl border border-border-subtle bg-surface-raised shadow-2xl outline-none"
      onkeydown={onKeydown}
    >
      {@render children()}
      {#if footer}
        {@render footer()}
      {/if}
    </div>
  </div>
{/if}