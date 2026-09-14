<script lang="ts">
  import FilterPanel from "./FilterPanel.svelte";
  import { activeFilters } from "$lib/filter";
  import type { BrowseQuery, MediaTag } from "$lib/types";

  let {
    genres,
    tags = [],
    current,
    base = "/filter",
    showSearch = true,
    showSort = true,
    extraParams = {},
  }: {
    genres: string[];
    tags?: MediaTag[];
    current: BrowseQuery;
    base?: string;
    showSearch?: boolean;
    showSort?: boolean;
    extraParams?: Record<string, string>;
  } = $props();

  /**
   * Whether the filter dropdown is open.
   *
   * Not persisted: the filters themselves live in the URL, and reopening the
   * dropdown should not surprise anyone with last session's state.
   */
  let open = $state(false);

  let container = $state<HTMLElement | null>(null);

  const applied = $derived(activeFilters(current));

  /**
   * Close when a click lands outside the bar.
   *
   * Guarded on `container` because the handler is attached to the window
   * before the element is bound, so an early click could arrive first.
   */
  function onWindowClick(event: MouseEvent) {
    if (!open || container === null) return;

    const target = event.target as Node | null;
    if (target !== null && !container.contains(target)) {
      open = false;
    }
  }
</script>

<svelte:window onclick={onWindowClick} />

<div bind:this={container} class="relative">
  <div class="flex flex-wrap items-center gap-3">
    <button
      type="button"
      data-testid="open-filters"
      aria-expanded={open}
      aria-controls="filter-dropdown"
      onclick={() => (open = !open)}
      class="flex shrink-0 items-center gap-2 rounded-full border border-border-subtle bg-surface-hover px-4 py-2 text-sm text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      <span aria-hidden="true">⚙</span>
      Filters
      {#if applied.length > 0}
        <!-- Says how much is applied without opening the dropdown. -->
        <span
          data-testid="filter-count"
          class="rounded-full bg-accent px-2 py-0.5 text-xs text-white"
        >
          {applied.length}
        </span>
      {/if}
    </button>

    {#if applied.length === 0}
      <p class="text-xs text-ink-faint">Nothing filtered yet.</p>
    {/if}
  </div>

  {#if open}
    <!-- z-30 clears the navbar, which is `sticky top-0 z-10`. Without a
         deliberate value the dropdown would render underneath it. -->
    <div
      id="filter-dropdown"
      data-testid="filter-dropdown"
      class="absolute left-0 top-full z-30 mt-2 w-max max-w-full"
    >
      <div class="rounded-xl bg-surface shadow-xl">
        <FilterPanel
          {genres}
          {tags}
          {current}
          {base}
          {showSearch}
          {showSort}
          {extraParams}
        />
      </div>
    </div>
  {/if}
</div>