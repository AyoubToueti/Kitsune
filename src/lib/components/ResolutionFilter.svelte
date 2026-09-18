<script lang="ts">
  import type { ResolutionCount } from "$lib/resolution";
  import type { Resolution } from "$lib/types";

  let {
    available,
    selected = [],
    onToggle,
  }: {
    /**
     * The resolutions to offer, highest first.
     *
     * Comes from the search results themselves, so a chip can never be offered
     * for a resolution the list does not contain -- a control that only ever
     * empties the list is worse than no control.
     */
    available: ResolutionCount[];
    /** The resolutions currently kept. Empty means "no filter". */
    selected?: Resolution[];
    /** Called with the resolution whose chip was clicked. */
    onToggle?: (resolution: Resolution) => void;
  } = $props();

  /** Whether one resolution's chip is currently on. */
  function isSelected(resolution: Resolution): boolean {
    return selected.includes(resolution);
  }

  /**
   * Chip styling.
   *
   * A selected chip uses the accent, matching the "included" pills in
   * `ActiveFilters`; an unselected one stays muted so the active filter reads at
   * a glance without counting.
   */
  function chipClass(resolution: Resolution): string {
    const base =
      "inline-flex items-center gap-1 rounded-full border px-3 py-1 text-xs transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent";

    return isSelected(resolution)
      ? `${base} border-accent bg-accent text-white hover:bg-accent-hover`
      : `${base} border-border-subtle text-ink-muted hover:border-accent hover:text-ink`;
  }
</script>

<!-- A single chip cannot change what is shown, so it is not worth the space.
     The list is only worth filtering when there is a choice to make. -->
{#if available.length > 1}
  <div
    data-testid="resolution-filter"
    role="group"
    aria-label="Filter by resolution"
    class="flex flex-wrap items-center gap-2"
  >
    {#each available as { resolution, count } (resolution)}
      <button
        type="button"
        onclick={() => onToggle?.(resolution)}
        aria-pressed={isSelected(resolution)}
        data-resolution={resolution}
        class={chipClass(resolution)}
      >
        {resolution}
        <span class="text-[0.65rem] opacity-70">{count}</span>
      </button>
    {/each}
  </div>
{/if}