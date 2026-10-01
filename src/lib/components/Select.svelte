<script lang="ts">
  /**
   * A themed dropdown, replacing the native `<select>`.
   *
   * Native `<select>` popups are drawn by the OS/browser (WebKitGTK here) and
   * cannot be styled from CSS -- the highlighted row is the system accent, not
   * anything the app controls. This renders its own listbox instead, so the
   * open list uses the app's surface and accent like every other menu.
   *
   * Form-compatible: when `name` is given, a hidden input carries the value, so
   * a plain GET form still submits this field exactly as a `<select>` would.
   */

  /** One choice. */
  export interface SelectOption {
    value: string;
    label: string;
  }

  let {
    name,
    value = $bindable(""),
    options,
    label,
    placeholder,
    onchange,
    widthClass = "w-max",
  }: {
    /** Form field name. Omit for a control that never submits (e.g. a setting). */
    name?: string;
    /** The chosen value. Two-way bindable. */
    value?: string;
    options: SelectOption[];
    /** Accessible name; also what tests and screen readers match on. */
    label: string;
    /** Shown when the current value matches no option. */
    placeholder?: string;
    /** Called with the new value on choose, for callers that persist on change. */
    onchange?: (value: string) => void;
    /** Width behaviour; the trigger defaults to content width. */
    widthClass?: string;
  } = $props();

  let open = $state(false);
  let container = $state<HTMLElement | null>(null);
  let trigger = $state<HTMLButtonElement | null>(null);

  /** Ids so the trigger can point at the list it controls (a11y). */
  const listId = `select-list-${crypto.randomUUID()}`;

  /** Which row the keyboard is on while open. */
  let activeIndex = $state(0);

  /** The label for the current value, falling back to the placeholder. */
  const selectedLabel = $derived(
    options.find((option) => option.value === value)?.label ??
      placeholder ??
      options[0]?.label ??
      "",
  );

  function close(returnFocus = false) {
    if (!open) return;
    open = false;
    if (returnFocus) trigger?.focus();
  }

  function openList() {
    open = true;
    // Start on the current choice, so Enter re-selects rather than jumping.
    const index = options.findIndex((option) => option.value === value);
    activeIndex = index >= 0 ? index : 0;
  }

  function toggle() {
    if (open) close();
    else openList();
  }

  function choose(option: SelectOption) {
    value = option.value;
    onchange?.(option.value);
    close(true);
  }

  /** Close when a click lands outside the whole control. */
  function onWindowClick(event: MouseEvent) {
    if (!open || container === null) return;
    const target = event.target as Node | null;
    if (target !== null && !container.contains(target)) close();
  }

  function onKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        if (!open) openList();
        else activeIndex = Math.min(activeIndex + 1, options.length - 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        if (!open) openList();
        else activeIndex = Math.max(activeIndex - 1, 0);
        break;
      case "Home":
        if (open) {
          event.preventDefault();
          activeIndex = 0;
        }
        break;
      case "End":
        if (open) {
          event.preventDefault();
          activeIndex = options.length - 1;
        }
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (open) choose(options[activeIndex]);
        else openList();
        break;
      case "Escape":
        if (open) {
          event.preventDefault();
          close(true);
        }
        break;
    }
  }
</script>

<svelte:window onclick={onWindowClick} />

<div bind:this={container} class="relative {widthClass}">
  <!-- The hidden field is what a GET form actually submits, so the panel keeps
       working with no JavaScript submit handler. -->
  {#if name}
    <input type="hidden" {name} {value} />
  {/if}

  <button
    bind:this={trigger}
    type="button"
    role="combobox"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={listId}
    aria-label={label}
    data-testid="select-trigger"
    data-value={value}
    onclick={toggle}
    onkeydown={onKeydown}
    class="flex w-full items-center justify-between gap-2 rounded-full border border-border-subtle bg-surface-hover px-4 py-2 text-sm text-ink transition-colors hover:border-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  >
    <span class="truncate">{selectedLabel}</span>
    <span
      aria-hidden="true"
      class="shrink-0 text-ink-faint transition-transform {open
        ? 'rotate-180'
        : ''}"
    >
      ▾
    </span>
  </button>

  {#if open}
    <ul
      id={listId}
      role="listbox"
      aria-label={label}
      data-testid="select-list"
      class="absolute left-0 top-full z-40 mt-2 max-h-64 w-full min-w-max overflow-y-auto rounded-xl border border-border-subtle bg-surface-raised py-1 shadow-xl"
    >
      {#each options as option, i (option.value)}
        <!-- Keyboard selection is handled on the trigger, so this only needs
             to not be a bare click target: the keydown keeps the a11y lint
             happy without duplicating the combobox's key handling. -->
        <li
          role="option"
          aria-selected={option.value === value}
          data-testid="select-option"
          onclick={() => choose(option)}
          onkeydown={(event) => {
            if (event.key === "Enter" || event.key === " ") choose(option);
          }}
          onmouseenter={() => (activeIndex = i)}
          class="cursor-pointer px-4 py-1.5 text-sm transition-colors {i ===
          activeIndex
            ? 'bg-surface-hover'
            : ''} {option.value === value
            ? 'text-accent'
            : 'text-ink'}"
        >
          {option.label}
        </li>
      {/each}
    </ul>
  {/if}
</div>