<script lang="ts">
  import {
    FILTER_SORT_VALUES,
    FORMAT_LABELS,
    FORMAT_VALUES,
    SCORE_OPTIONS,
    SEASON_LABELS,
    SEASON_VALUES,
    SORT_LABELS,
    STATUS_LABELS,
    STATUS_VALUES,
    yearOptions,
  } from "$lib/filter";
  import type { BrowseQuery } from "$lib/types";

  let {
    genres,
    current,
  }: {
    /** Genre names to offer, from the provider. */
    genres: string[];
    /** The filters currently applied, so the controls reflect them. */
    current: BrowseQuery;
  } = $props();

  const years = yearOptions();
  const selectedGenres = $derived(new Set(current.genres ?? []));

  /** Shared styling for the select controls. */
  const selectClass =
    "rounded-lg border border-border-subtle bg-surface-hover px-3 py-1.5 text-sm text-ink focus:ring-2 focus:ring-accent focus:outline-none";
</script>

<!-- A plain GET form rather than a JS submit handler. The browser builds the
     query string, the page reads it back, and the whole thing works without
     JavaScript and is testable without SvelteKit's runtime. -->
<form
  action="/filter"
  method="GET"
  data-testid="filter-form"
  class="rounded-xl border border-border-subtle p-4"
>
  <h2 class="mb-3 text-sm font-semibold tracking-tight text-accent">Filter</h2>

  <div class="flex flex-wrap items-end gap-3">
    <label class="flex flex-col gap-1 text-xs text-ink-faint">
      Search
      <input
        type="search"
        name="search"
        value={current.search ?? ""}
        placeholder="Any"
        autocomplete="off"
        class={selectClass}
      />
    </label>

    <label class="flex flex-col gap-1 text-xs text-ink-faint">
      Type
      <select name="format" class={selectClass}>
        <option value="">All</option>
        {#each FORMAT_VALUES as value (value)}
          <option value={value} selected={current.format === value}>
            {FORMAT_LABELS[value]}
          </option>
        {/each}
      </select>
    </label>

    <label class="flex flex-col gap-1 text-xs text-ink-faint">
      Status
      <select name="status" class={selectClass}>
        <option value="">All</option>
        {#each STATUS_VALUES as value (value)}
          <option value={value} selected={current.status === value}>
            {STATUS_LABELS[value]}
          </option>
        {/each}
      </select>
    </label>

    <label class="flex flex-col gap-1 text-xs text-ink-faint">
      Score
      <select name="score" class={selectClass}>
        <option value="">All</option>
        {#each SCORE_OPTIONS as value (value)}
          <option value={value} selected={current.minScore === value}>
            {value}+
          </option>
        {/each}
      </select>
    </label>

    <label class="flex flex-col gap-1 text-xs text-ink-faint">
      Season
      <select name="season" class={selectClass}>
        <option value="">All</option>
        {#each SEASON_VALUES as value (value)}
          <option value={value} selected={current.season === value}>
            {SEASON_LABELS[value]}
          </option>
        {/each}
      </select>
    </label>

    <label class="flex flex-col gap-1 text-xs text-ink-faint">
      Year
      <select name="year" class={selectClass}>
        <option value="">Any</option>
        {#each years as year (year)}
          <option value={year} selected={current.seasonYear === year}>
            {year}
          </option>
        {/each}
      </select>
    </label>

    <label class="flex flex-col gap-1 text-xs text-ink-faint">
      Sort
      <select name="sort" class={selectClass}>
        {#each FILTER_SORT_VALUES as value (value)}
          <option value={value} selected={current.sort === value}>
            {SORT_LABELS[value]}
          </option>
        {/each}
      </select>
    </label>
  </div>

  {#if genres.length}
    <fieldset class="mt-4">
      <legend class="mb-2 text-xs text-ink-faint">Genre</legend>

      <div class="flex flex-wrap gap-2">
        {#each genres as genre (genre)}
          <!-- Checkboxes rather than a multi-select: every genre is visible at
               once, and the browser sends repeated `genre` parameters. -->
          <label
            class="cursor-pointer rounded-full border border-border-subtle px-3 py-1 text-xs transition-colors has-checked:border-accent has-checked:bg-accent has-checked:text-white hover:text-ink"
          >
            <input
              type="checkbox"
              name="genre"
              value={genre}
              checked={selectedGenres.has(genre)}
              class="sr-only"
            />
            {genre}
          </label>
        {/each}
      </div>
    </fieldset>
  {/if}

  <div class="mt-4 flex items-center gap-3">
    <button
      type="submit"
      class="rounded-full bg-accent px-5 py-1.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      Filter
    </button>

    <a
      href="/filter"
      class="text-sm text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      Reset
    </a>
  </div>
</form>