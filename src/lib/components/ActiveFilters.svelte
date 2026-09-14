<script lang="ts">
  import { activeFilters, clearFiltersHref, removeFilterHref } from "$lib/filter";
  import type { BrowseQuery } from "$lib/types";

  let {
    params,
    current,
    base = "/filter",
  }: {
    /**
     * The live URL parameters.
     *
     * Passed in rather than read from `$app/state` so the component stays
     * testable without SvelteKit's runtime, and so it can serve any route.
     */
    params: URLSearchParams;
    /** The parsed filters, which decide which pills appear. */
    current: BrowseQuery;
    /** Which route the removal links should point back at. */
    base?: string;
  } = $props();

  const filters = $derived(activeFilters(current));

  /**
   * Styling per pill.
   *
   * An exclusion gets the same red as the chip it came from: the bar and the
   * catalogue describe the same state, so they should not disagree about how
   * to depict it.
   */
  function pillClass(excluded: boolean): string {
    const base =
      "inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent";

    return excluded
      ? `${base} bg-danger text-white hover:bg-danger/80`
      : `${base} bg-accent text-white hover:bg-accent-hover`;
  }
</script>

<!-- Nothing applied: render nothing rather than an empty bar. -->
{#if filters.length > 0}
  <div
    data-testid="active-filters"
    class="flex flex-wrap items-center gap-2 rounded-xl border border-border-subtle px-3 py-2"
  >
    <span class="text-xs text-ink-faint">Active</span>

    {#each filters as filter (`${filter.param}:${filter.value}`)}
      <!-- A link, not a button: removing a filter is navigation, so the back
           button undoes it and the result is shareable. -->
      <a
        href={removeFilterHref(params, filter.param, filter.value, base)}
        class={pillClass(filter.excluded)}
        aria-label="Remove {filter.label} filter"
      >
        {filter.label}
        <span aria-hidden="true">×</span>
      </a>
    {/each}

    {#if filters.length > 1}
      <a
        href={clearFiltersHref(params, current, base)}
        data-testid="clear-all"
        class="ml-1 text-xs text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        Clear all
      </a>
    {/if}
  </div>
{/if}