<script lang="ts">
  import {
    FORMAT_LABELS,
    FORMAT_VALUES,
    SCORE_OPTIONS,
    SEASON_LABELS,
    SEASON_VALUES,
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
    base = "/filter",
    showSearch = true,
    extraParams = {},
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
    /**
     * Which route the form submits to.
     *
     * Parameterised so the same panel can serve /filter and /search; submitting
     * to the wrong one would drop the search term or the filters.
     */
    base?: string;
    /**
     * Whether to render the free-text search field.
     *
     * /search hides it. The navbar's search box owns the term there, and two
     * inputs writing different parameters to one URL would lose it.
     */
    showSearch?: boolean;
    /**
     * Parameters to carry through submission without showing a control.
     *
     * /search's `q` is the term the user already typed. It is not a filter, but
     * a GET submit replaces the whole query string, so it has to be re-sent or
     * the search is silently reset by changing any filter.
     */
    extraParams?: Record<string, string>;
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
   * Whether the tag catalogue is expanded.
   *
   * Collapsed by default: 361 chips is a wall of colour, and most filter use
   * does not involve a tag. Open state is not persisted -- the catalogue is a
   * tool you reach for, not a setting.
   */
  let showCatalogue = $state(false);

  /** How many tags are currently doing something, for the toggle's badge. */
  const activeTagCount = $derived(Object.keys(tagStates).length);

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
     and the catalogue toggle need script, but the submission itself does not:
     the hidden inputs are real form fields, so the URL is still the whole
     truth and every control re-seeds from it on the next load. -->
<form
  action={base}
  method="GET"
  data-testid="filter-form"
  class="rounded-xl border border-border-subtle p-4"
>
  <h2 class="mb-3 text-sm font-semibold tracking-tight text-accent">Filter</h2>

  <div class="flex flex-wrap items-end gap-3">
    {#if showSearch}
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
    {/if}

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

  </div>

  <!-- The sort lives in the toolbar now, but it still has to be re-sent: a GET
       submit replaces the whole query string, so omitting it here would reset
       the ordering every time a filter changed. -->
  <input type="hidden" name="sort" value={current.sort} />

  <!-- Carried through submission without a control. A GET submit replaces the
       whole query string, so anything not re-sent here is dropped -- which for
       /search would silently reset the term on the next filter change. -->
  {#each Object.entries(extraParams) as [name, value] (name)}
    <input type="hidden" name={name} value={value} />
  {/each}

  {#if hasCatalogue}
    <div class="mt-4 border-t border-border-subtle pt-4">
      <!-- The catalogue is disclosure, not a control: opening it changes what
           is on screen, never what is submitted. That is why it is a button
           rather than a checkbox with a name. -->
      <button
        type="button"
        data-testid="toggle-catalogue"
        aria-expanded={showCatalogue}
        aria-controls="tag-catalogue"
        onclick={() => (showCatalogue = !showCatalogue)}
        class="flex w-full items-center justify-between text-xs font-medium text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <span class="flex items-center gap-2">
          <span aria-hidden="true">{showCatalogue ? "▾" : "▸"}</span>
          Advanced Genre &amp; Tag Filters
          {#if activeTagCount > 0}
            <!-- Says how many tags are active without the user having to open
                 the catalogue to find out. -->
            <span
              data-testid="active-tag-count"
              class="rounded-full bg-accent px-2 py-0.5 text-white"
            >
              {activeTagCount}
            </span>
          {/if}
        </span>
      </button>

      {#if showCatalogue}
        <div id="tag-catalogue" class="mt-3">
          {#if tagGroups.length}
            <!-- Deliberately carries no `name`: this narrows what is shown and
                 must not be submitted as a filter of its own. -->
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

          <!-- A scroll region rather than letting 361 chips push the results
               off the page. Fixed height so the results stay where the user
               left them when the catalogue opens. -->
          <div
            data-testid="tag-scroll"
            class="flex max-h-80 flex-col gap-3 overflow-y-auto pr-1"
          >
            {#if genres.length}
              <div>
                <p class={groupHeadingClass}>Genres</p>

                <div class="flex flex-wrap gap-2">
                  {#each genres as genre (genre)}
                    <!-- Genres stay plain checkboxes: a genre is only ever
                         required or absent, so there is no third state. -->
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

                  {/each}
                </div>
              </div>
            {/each}

            {#if nothingMatched}
              <p class="text-xs text-ink-faint">No tags match “{tagFilter}”.</p>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- The active tags' form fields, deliberately OUTSIDE the collapsible
       catalogue. The chips are buttons, so the submission is carried by these
       hidden inputs -- and if they lived inside the collapse, changing an
       unrelated filter with the catalogue shut would silently drop every tag
       the user had set. -->
  {#each Object.entries(tagStates) as [name, state] (name)}
    {#if state === "include"}
      <input type="hidden" name="tag" value={name} />
    {:else}
      <input type="hidden" name="exclude_tag" value={name} />
    {/if}
  {/each}

  <div class="mt-4 flex items-center gap-3">
    <button
      type="submit"
      class="rounded-full bg-accent px-5 py-1.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      Filter
    </button>

    <a
      href={base}
      class="text-sm text-ink-muted transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
    >
      Reset
    </a>
  </div>
</form>