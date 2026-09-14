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
    categoryLabel,
    groupTagsByCategory,
    yearOptions,
  } from "$lib/filter";
  import type { BrowseQuery, MediaTag } from "$lib/types";

  let {
    genres,
    tags = [],
    current,
  }: {
    /** Genre names to offer, from the provider. */
    genres: string[];
    /**
     * Tags to offer, each with its grouping category.
     *
     * Optional so the panel still renders when the catalogue request fails: a
     * missing tag list is a smaller panel, not a broken page.
     */
    tags?: MediaTag[];
    /** The filters currently applied, so the controls reflect them. */
    current: BrowseQuery;
  } = $props();

  /**
   * What a tag chip is currently doing.
   *
   * Absent from the map means "off", so the map only ever holds the tags the
   * user has actually touched. That keeps the common case (nothing selected)
   * allocation-free and makes "is this tag on?" a single lookup.
   */
  type TagState = "include" | "exclude";

  const years = yearOptions();
  const selectedGenres = $derived(new Set(current.genres ?? []));

  const tagGroups = $derived(groupTagsByCategory(tags));

  /**
   * The chip states, seeded from the URL.
   *
   * Seeded once rather than derived: the user's clicks are the source of truth
   * while the form is open, and the URL is re-read on the next page load after
   * a submit. Deriving would fight the clicks.
   */
  function initialTagStates(): Record<string, TagState> {
    const states: Record<string, TagState> = {};
    for (const name of current.tags ?? []) states[name] = "include";
    for (const name of current.excludedTags ?? []) states[name] = "exclude";
    return states;
  }

  let tagStates = $state<Record<string, TagState>>(initialTagStates());

  /**
   * Advance one tag through off -> include -> exclude -> off.
   *
   * Replaced rather than mutated so the `$state` proxy sees the change; a
   * delete on the proxy would not notify anything reading the key.
   */
  function cycleTag(name: string) {
    const updated = { ...tagStates };

    if (updated[name] === undefined) {
      updated[name] = "include";
    } else if (updated[name] === "include") {
      updated[name] = "exclude";
    } else {
      delete updated[name];
    }

    tagStates = updated;
  }

  /**
   * What the user typed into the tag search box.
   *
   * Lives outside the form's data: the box carries no `name`, so it narrows
   * what is visible without ever being submitted as a filter.
   */
  let tagFilter = $state("");

  /**
   * The groups to render, narrowed by the search box.
   *
   * A group whose tags all fail to match is dropped entirely -- an empty
   * heading would just be noise. With no search term this is the full
   * catalogue, so the common case costs nothing.
   */
  const visibleGroups = $derived.by(() => {
    const needle = tagFilter.trim().toLowerCase();
    if (needle === "") return tagGroups;

    return tagGroups
      .map((group) => ({
        category: group.category,
        tags: group.tags.filter((tag) =>
          tag.name.toLowerCase().includes(needle),
        ),
      }))
      .filter((group) => group.tags.length > 0);
  });

  /** Whether a search term excluded everything, so we can say so. */
  const nothingMatched = $derived(
    tagFilter.trim() !== "" && visibleGroups.length === 0,
  );

  const hasCatalogue = $derived(genres.length > 0 || tagGroups.length > 0);

  /** Shared styling for the select and search controls. */
  const selectClass =
    "rounded-lg border border-border-subtle bg-surface-hover px-3 py-1.5 text-sm text-ink focus:ring-2 focus:ring-accent focus:outline-none";

  /** Shared styling for a genre chip, which stays a plain checkbox. */
  const genreChipClass =
    "cursor-pointer rounded-full border border-border-subtle px-3 py-1 text-xs transition-colors has-checked:border-accent has-checked:bg-accent has-checked:text-white hover:text-ink";

  /**
   * Styling for a tag chip, which carries its own state.
   *
   * Three visual states rather than two: a rejected tag has to be as legible
   * as a required one, or the user cannot tell which direction they picked.
   */
  function tagChipClass(state: TagState | undefined): string {
    const base =
      "cursor-pointer rounded-full border px-3 py-1 text-xs transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent";

    if (state === "include") {
      return `${base} border-accent bg-accent text-white`;
    }
    if (state === "exclude") {
      return `${base} border-danger bg-danger text-white`;
    }
    return `${base} border-border-subtle text-ink hover:text-accent`;
  }

  /** Shared styling for a group heading inside the catalogue. */
  const groupHeadingClass = "mb-2 text-xs font-medium text-ink-muted";
</script>

<!-- A plain GET form. The browser builds the query string, the page reads it
     back, and the filters survive a reload with no state store. Tag cycling
     needs script, but the submission itself does not: the hidden inputs are
     real form fields, so the resulting URL is still the whole truth. -->
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

  {#if hasCatalogue}
    <fieldset class="mt-4 border-t border-border-subtle pt-4">
      <legend class="mb-2 text-xs text-ink-faint">Genres &amp; tags</legend>

      {#if tagGroups.length}
        <!-- Deliberately carries no `name`: this narrows what is shown and must
             not be submitted as a filter of its own. -->
        <label class="mb-3 flex flex-col gap-1 text-xs text-ink-faint">
          Filter tags
          <input
            type="search"
            bind:value={tagFilter}
            placeholder="Type to narrow the list"
            autocomplete="off"
            class={selectClass}
          />
        </label>
      {/if}

      <div class="flex flex-col gap-3">
        {#if genres.length}
          <div>
            <p class={groupHeadingClass}>Genres</p>

            <div class="flex flex-wrap gap-2">
              {#each genres as genre (genre)}
                <!-- Genres stay plain checkboxes: a genre is only ever required
                     or absent, so there is no third state to cycle through. -->
                <label class={genreChipClass}>
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
          </div>
        {/if}

        {#each visibleGroups as group (group.category)}
          <div>
            <p class={groupHeadingClass}>{categoryLabel(group.category)}</p>

            <div class="flex flex-wrap gap-2">
              {#each group.tags as tag (tag.name)}
                <button
                  type="button"
                  data-testid="tag-chip"
                  data-state={tagStates[tag.name] ?? "off"}
                  title={tag.description ?? undefined}
                  onclick={() => cycleTag(tag.name)}
                  class={tagChipClass(tagStates[tag.name])}
                >
                  {tag.name}
                </button>

                <!-- The chip is a button, so the submission is carried by these
                     hidden fields. Exactly one exists per active tag, which is
                     what keeps the URL unambiguous about the direction. -->
                {#if tagStates[tag.name] === "include"}
                  <input type="hidden" name="tag" value={tag.name} />
                {:else if tagStates[tag.name] === "exclude"}
                  <input type="hidden" name="exclude_tag" value={tag.name} />
                {/if}
              {/each}
            </div>
          </div>
        {/each}

        {#if nothingMatched}
          <p class="text-xs text-ink-faint">No tags match “{tagFilter}”.</p>
        {/if}
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