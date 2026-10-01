<script lang="ts">
  import FilterPanel from "./FilterPanel.svelte";
  import { activeFilters, SORT_LABELS, SORT_MENU_VALUES, sortHref } from "$lib/filter";
  import type { BrowseQuery, MediaTag } from "$lib/types";

  let {
    params,
    genres,
    tags = [],
    current,
    base = "/filter",
    showSearch = true,
    showSort = true,
    extraParams = {},
  }: {
    /**
     * The live URL parameters.
     *
     * Passed in rather than read from `$app/state` so the component stays
     * testable without SvelteKit's runtime, and because a sort link has to
     * carry the current filters through.
     */
    params: URLSearchParams;
    genres: string[];
    tags?: MediaTag[];
    current: BrowseQuery;
    base?: string;
    showSearch?: boolean;
    /**
     * Whether to offer the sort menu.
     *
     * /search hides it: a text search is ranked by relevance, and relevance is
     * deliberately not on the menu because it means nothing without a query.
     */
    showSort?: boolean;
    extraParams?: Record<string, string>;
  } = $props();

  /** Which dropdown is open, if either. Only one at a time. */
  let open = $state<"filters" | "sort" | null>(null);

  let container = $state<HTMLElement | null>(null);

  const applied = $derived(activeFilters(current));

  /** The current sort's label, for the button. */
  const sortLabel = $derived(SORT_LABELS[current.sort]);

  /**
   * Close whichever dropdown is open when a click lands outside the bar.
   *
   * Guarded on `container` because the handler is attached before the element
   * is bound, so an early click could arrive first.
   */
  function onWindowClick(event: MouseEvent) {
    if (open === null || container === null) return;

    // `composedPath()` rather than `event.target`: a click inside a dropdown
    // can remove the node it landed on before this window handler runs (the
    // Select closes its list on the same click), and a detached target is no
    // longer `container.contains(...)`. The composed path is captured at
    // dispatch, so it still includes the ancestors the click passed through.
    const path = event.composedPath();
    if (!path.includes(container)) {
      open = null;
    }
  }

  /** Toggle one dropdown, closing the other so they cannot overlap. */
  function toggle(which: "filters" | "sort") {
    open = open === which ? null : which;
  }

  /** Button styling, shared so the two ends of the row match. */
  const buttonClass =
    "flex shrink-0 items-center gap-2 rounded-full border border-border-subtle bg-surface-hover px-4 py-2 text-sm text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent";
</script>

<svelte:window onclick={onWindowClick} />

<div bind:this={container} class="relative">
  <!-- Filters on the left, sort on the right: the two ends of one row. -->
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div class="flex items-center gap-3">
      <button
        type="button"
        data-testid="open-filters"
        aria-expanded={open === "filters"}
        aria-controls="filter-dropdown"
        onclick={() => toggle("filters")}
        class={buttonClass}
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

    {#if showSort}
      <button
        type="button"
        data-testid="open-sort"
        aria-expanded={open === "sort"}
        aria-controls="sort-dropdown"
        onclick={() => toggle("sort")}
        class={buttonClass}
      >
        <span aria-hidden="true">⇅</span>
        {sortLabel}
      </button>
    {/if}
  </div>

  {#if open === "filters"}
    <!-- z-30 clears the navbar, which is `sticky top-0 z-10`. Without a
         deliberate value the dropdown would render underneath it. -->
    <div
      id="filter-dropdown"
      data-testid="filter-dropdown"
      class="absolute left-0 top-full z-30 mt-2 w-max max-w-full"
    >
      <!-- No `overflow-hidden` here: the Select lists are absolutely
           positioned children that extend past this box, and clipping them
           would hide (and make unclickable) the open dropdown. The panel's own
           padding keeps content clear of the rounded corners. -->
      <div
        class="rounded-xl border border-border-subtle bg-surface-raised shadow-xl"
      >
        <FilterPanel {genres} {tags} {current} {base} {showSearch} {extraParams} />
      </div>
    </div>
  {/if}

  {#if open === "sort"}
    <!-- Right-aligned, since the button that opens it is. -->
    <div
      id="sort-dropdown"
      data-testid="sort-dropdown"
      class="absolute right-0 top-full z-30 mt-2 w-44"
    >
      <ul
        class="overflow-hidden rounded-xl border border-border-subtle bg-surface-raised py-2 shadow-xl"
      >
        {#each SORT_MENU_VALUES as value (value)}
          <!-- Links, not buttons: the sort lives in the URL, so choosing one is
               navigation. The back button undoes it and the result is
               shareable. -->
          <li>
            <a
              href={sortHref(params, value, base)}
              aria-current={current.sort === value ? "true" : undefined}
              class="block px-4 py-1.5 text-sm transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {current.sort ===
              value
                ? 'text-accent'
                : 'text-ink'}"
            >
              {SORT_LABELS[value]}
            </a>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>