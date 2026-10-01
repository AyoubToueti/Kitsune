<script lang="ts">
  import FilterPanel from "./FilterPanel.svelte";
  import {
    activeFilters,
    reverseHref,
    SORT_LABELS,
    SORT_MENU_VALUES,
    sortHref,
  } from "$lib/filter";
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
    params: URLSearchParams;
    genres: string[];
    tags?: MediaTag[];
    current: BrowseQuery;
    base?: string;
    showSearch?: boolean;
    showSort?: boolean;
    extraParams?: Record<string, string>;
  } = $props();

  let open = $state<"filters" | "sort" | null>(null);

  /** Specific element bindings for buttons and dropdown elements */
  let filterBtn = $state<HTMLElement | null>(null);
  let filterDropdown = $state<HTMLElement | null>(null);
  let sortBtn = $state<HTMLElement | null>(null);
  let sortDropdown = $state<HTMLElement | null>(null);

  const applied = $derived(activeFilters(current));
  const sortLabel = $derived(SORT_LABELS[current.sort]);
  const isRelevance = $derived(current.sort === "searchMatch");
  const isReversed = $derived(current.reversed === true);

  /**
   * Closes open dropdowns when clicking outside the active trigger button or popup panel.
   */
  function onWindowClick(event: MouseEvent) {
    if (open === null) return;

    const path = event.composedPath();

    if (open === "filters") {
      const insideFilterBtn = filterBtn && path.includes(filterBtn);
      const insideFilterPanel = filterDropdown && path.includes(filterDropdown);

      if (!insideFilterBtn && !insideFilterPanel) {
        open = null;
      }
    } else if (open === "sort") {
      const insideSortBtn = sortBtn && path.includes(sortBtn);
      const insideSortPanel = sortDropdown && path.includes(sortDropdown);

      if (!insideSortBtn && !insideSortPanel) {
        open = null;
      }
    }
  }

  function toggle(which: "filters" | "sort") {
    open = open === which ? null : which;
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="relative w-full">
  <!-- Toolbar Row -->
  <div class="flex flex-wrap items-center justify-between gap-3">
    <!-- Left: Filters Toggle -->
    <div class="flex items-center gap-3">
      <button
        bind:this={filterBtn}
        type="button"
        data-testid="open-filters"
        aria-expanded={open === "filters"}
        aria-controls="filter-dropdown"
        onclick={() => toggle("filters")}
        class="flex h-9 items-center gap-2 rounded-xl border border-border-subtle bg-surface-raised px-3.5 text-xs font-semibold text-ink shadow-sm transition-all hover:border-accent hover:shadow-md focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <svg
          class="h-4 w-4 fill-none stroke-current stroke-2"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M3 4h18M3 12h18M3 20h18M7 2v4m4 4v4m-2 4v4"
          />
        </svg>
        <span>Filters</span>

        {#if applied.length > 0}
          <span
            data-testid="filter-count"
            class="flex h-5 min-w-[20px] items-center justify-center rounded-full bg-accent px-1.5 text-[10px] font-bold text-white shadow-sm"
          >
            {applied.length}
          </span>
        {/if}
      </button>

      {#if applied.length === 0}
        <p class="hidden text-xs text-ink-faint sm:block">Nothing filtered yet.</p>
      {/if}
    </div>

    <!-- Right: Sort Segmented Group -->
    {#if showSort}
      <div
        bind:this={sortBtn}
        class="inline-flex items-center rounded-xl border border-border-subtle bg-surface-raised p-1 shadow-sm transition-all hover:border-border"
      >
        <a
          href={reverseHref(params, base)}
          role="button"
          data-testid="reverse-order"
          aria-pressed={isReversed}
          aria-disabled={isRelevance}
          title={isRelevance
            ? "Best match cannot be reversed"
            : isReversed
              ? "Reversed — click for natural order"
              : "Reverse sort order"}
          class="flex h-8 w-8 items-center justify-center rounded-lg text-ink-muted transition-all hover:bg-surface-hover hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent aria-pressed:bg-accent/10 aria-pressed:text-accent {isRelevance
            ? 'pointer-events-none opacity-40'
            : ''}"
        >
          <svg
            class="h-4 w-4 fill-none stroke-current stroke-2 transition-transform duration-200 {isReversed
              ? 'rotate-180'
              : ''}"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4"
            />
          </svg>
          <span class="sr-only">
            {isReversed ? "Reversed order" : "Natural order"}
          </span>
        </a>

        <div class="mx-1 h-4 w-px bg-border-subtle" aria-hidden="true"></div>

        <button
          type="button"
          data-testid="open-sort"
          aria-expanded={open === "sort"}
          aria-controls="sort-dropdown"
          onclick={() => toggle("sort")}
          class="flex h-8 items-center gap-2 rounded-lg px-2.5 text-xs font-medium text-ink transition-all hover:bg-surface-hover hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          <span class="font-semibold text-accent">{sortLabel}</span>

          <svg
            class="h-3.5 w-3.5 text-ink-muted transition-transform duration-200 {open ===
            'sort'
              ? 'rotate-180'
              : ''}"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            aria-hidden="true"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </button>
      </div>
    {/if}
  </div>

  <!-- Filter Dropdown Panel (Positioned relative to full toolbar width) -->
  {#if open === "filters"}
    <div
      bind:this={filterDropdown}
      id="filter-dropdown"
      data-testid="filter-dropdown"
      class="absolute left-0 top-full z-30 mt-2 w-full min-w-[320px] max-w-full sm:w-auto"
    >
      <div
        class="rounded-xl border border-border-subtle bg-surface-raised shadow-2xl"
      >
        <FilterPanel
          {genres}
          {tags}
          {current}
          {base}
          {showSearch}
          {extraParams}
        />
      </div>
    </div>
  {/if}

  <!-- Sort Dropdown Menu -->
  {#if open === "sort"}
    <div
      bind:this={sortDropdown}
      id="sort-dropdown"
      data-testid="sort-dropdown"
      class="absolute right-0 top-full z-30 mt-2 w-48"
    >
      <ul
        class="overflow-hidden rounded-xl border border-border-subtle bg-surface-raised p-1.5 shadow-xl"
      >
        {#each SORT_MENU_VALUES as value (value)}
          <li>
            <a
              href={sortHref(params, value, base)}
              aria-current={current.sort === value ? "true" : undefined}
              class="flex items-center justify-between rounded-lg px-3 py-2 text-xs font-medium transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {current.sort ===
              value
                ? 'bg-accent/10 font-semibold text-accent'
                : 'text-ink'}"
            >
              <span>{SORT_LABELS[value]}</span>
              {#if current.sort === value}
                <svg
                  class="h-3.5 w-3.5 fill-current text-accent"
                  viewBox="0 0 20 20"
                  aria-hidden="true"
                >
                  <path
                    fill-rule="evenodd"
                    d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                    clip-rule="evenodd"
                  />
                </svg>
              {/if}
            </a>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>