<script lang="ts">
  import { clampPage, pageWindow } from "$lib/pagination";

  let {
    current,
    last,
    hrefFor,
  }: {
    /** 1-based current page. */
    current: number;
    /** Total pages. May be 0 before the first response arrives. */
    last: number;
    /**
     * Build the href for a page.
     *
     * Passed in rather than computed here so the control works on any route
     * without knowing how that route spells its query string.
     */
    hrefFor: (page: number) => string;
  } = $props();

  const entries = $derived(pageWindow(current, last));
  const total = $derived(Math.max(1, last));
  // Clamped through the shared helper so the control and the window cannot
  // disagree about what an out-of-range page means.
  const active = $derived(clampPage(current, last));
  const atStart = $derived(active <= 1);
  const atEnd = $derived(active >= total);

  /** Shared styling for the numeric page buttons. */
  const pageClass =
    "flex h-9 min-w-9 items-center justify-center rounded-full px-3 text-sm transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent";

  /** The arrow buttons, which grey out at the ends. */
  const arrowClass =
    "flex h-9 w-9 items-center justify-center rounded-full bg-surface-hover text-ink-muted transition-colors hover:text-ink focus:outline-none focus-visible:ring-2 focus-visible:ring-accent";
</script>

<!-- Nothing to page through. Rendering a lone "1" would be noise. -->
{#if last > 1}
  <nav
    aria-label="Pagination"
    data-testid="pagination"
    class="mt-8 flex items-center justify-center gap-2"
  >
    {#if atStart}
      <span
        aria-hidden="true"
        data-testid="prev-disabled"
        class="{arrowClass} opacity-40"
      >
        ‹
      </span>
    {:else}
      <a
        href={hrefFor(active - 1)}
        rel="prev"
        aria-label="Previous page"
        class={arrowClass}
      >
        ‹
      </a>
    {/if}

    {#each entries as entry, i (`${i}-${entry}`)}
      {#if entry === "gap"}
        <span aria-hidden="true" class="px-1 text-ink-faint">…</span>
      {:else if entry === active}
        <span
          aria-current="page"
          data-testid="current-page"
          class="{pageClass} bg-accent font-medium text-white"
        >
          {entry}
        </span>
      {:else}
        <a
          href={hrefFor(entry)}
          aria-label="Page {entry}"
          class="{pageClass} bg-surface-hover text-ink-muted hover:text-ink"
        >
          {entry}
        </a>
      {/if}
    {/each}

    {#if atEnd}
      <span
        aria-hidden="true"
        data-testid="next-disabled"
        class="{arrowClass} opacity-40"
      >
        ›
      </span>
    {:else}
      <a
        href={hrefFor(active + 1)}
        rel="next"
        aria-label="Next page"
        class={arrowClass}
      >
        ›
      </a>
    {/if}

    {#if !atEnd}
      <a
        href={hrefFor(total)}
        aria-label="Last page"
        class={arrowClass}
      >
        »
      </a>
    {/if}
  </nav>
{/if}