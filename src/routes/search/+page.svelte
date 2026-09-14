<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime, errorMessage, getGenres, getTags } from "$lib/api/anime";
  import ActiveFilters from "$lib/components/ActiveFilters.svelte";
  import AnimeGrid from "$lib/components/AnimeGrid.svelte";
  import FilterBar from "$lib/components/FilterBar.svelte";
  import Pagination from "$lib/components/Pagination.svelte";
  import { filterHref, parseBrowseQuery } from "$lib/filter";
  import { clampPage } from "$lib/pagination";
  import type { AnimePage, MediaTag } from "$lib/types";

  /** AniList caps perPage at 50; the backend clamps anything larger. */
  const PER_PAGE = 30;

  const params = $derived(page.url.searchParams);

  /**
   * The search term.
   *
   * `q`, not `search`: the navbar's search box owns this parameter, and the
   * filter panel reads `search`. Keeping them distinct means a filtered search
   * cannot accidentally overwrite the term the user typed.
   */
  const term = $derived((params.get("q") ?? "").trim());

  /** The filters applied on top of the search, read from the same URL. */
  const filters = $derived(parseBrowseQuery(params));

  /**
   * The requested page, sanitised to a positive integer.
   *
   * `Number("abc")` is NaN and `Number("0")` is 0; both mean "first page" here.
   */
  const requestedPage = $derived(
    Math.max(1, Math.floor(Number(params.get("page") ?? "1")) || 1),
  );

  let result = $state<AnimePage | null>(null);
  let genres = $state<string[]>([]);
  let tags = $state<MediaTag[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  // Distinguishes "nothing yet" from "searched and found nothing", which need
  // different copy.
  let searched = $state("");

  getGenres()
    .then((found) => {
      genres = found;
    })
    .catch(() => {
      // Same reasoning as the filter page: a missing list is a smaller panel,
      // not a broken page.
      genres = [];
    });

  getTags()
    .then((found) => {
      tags = found;
    })
    .catch(() => {
      tags = [];
    });

  $effect(() => {
    const q = term;
    const current = filters;
    const requested = requestedPage;

    if (q === "") {
      result = null;
      error = null;
      loading = false;
      searched = "";
      return;
    }

    // Guards against a stale response overwriting a newer one when the user
    // searches or re-filters quickly.
    let cancelled = false;
    loading = true;
    error = null;

    // Relevance first, always: a text search should return the closest matches,
    // not the most popular titles that happen to contain the words. The panel
    // therefore does not offer a sort control here.
    browseAnime({ ...current, search: q, sort: "searchMatch" }, requested, PER_PAGE)
      .then((found) => {
        if (cancelled) return;
        result = found;
        searched = q;
      })
      .catch((err) => {
        if (cancelled) return;
        error = errorMessage(err);
      })
      .finally(() => {
        if (cancelled) return;
        loading = false;
      });

    return () => {
      cancelled = true;
    };
  });

  /**
   * Build a page link that keeps both the term and the filters.
   *
   * Routed through `filterHref` so the repeated parameters (genre, tag,
   * exclude_tag) survive paging rather than only the first of each.
   */
  function hrefFor(target: number): string {
    const last = result?.pageInfo.lastPage ?? 1;
    return filterHref(params, clampPage(target, last), "/search");
  }
</script>

<!-- Echoes the term only when there are results to head. The empty state names
     the term itself, and two elements carrying it would read as a stutter. -->
<h1 class="mb-4 text-lg font-semibold tracking-tight">
  {#if result && result.items.length > 0}
    Results for “{searched}”
  {:else}
    Search anime
  {/if}
</h1>

<!-- The term is not shown as a field: the navbar's search box already owns it.
     It rides along as a hidden parameter so changing a filter does not wipe it. -->
<FilterBar
  {genres}
  {tags}
  current={filters}
  base="/search"
  showSearch={false}
  showSort={false}
  extraParams={{ q: term }}
/>

<div class="mt-4">
  <ActiveFilters {params} current={filters} base="/search" />
</div>

<div class="mt-8">
  {#if term === ""}
    <p class="py-16 text-center text-ink-muted">
      Type something in the search box above.
    </p>
  {:else if loading}
    <p class="py-16 text-center text-ink-muted">Searching…</p>
  {:else if error}
    <div class="py-16 text-center">
      <p class="text-ink">Could not run that search.</p>
      <p class="mt-2 text-sm text-ink-faint">{error}</p>
    </div>
  {:else if result && result.items.length === 0}
    <p class="py-16 text-center text-ink-muted">No results for “{searched}”.</p>
  {:else if result}
    <AnimeGrid anime={result.items} />
    <Pagination
      current={result.pageInfo.currentPage}
      last={result.pageInfo.lastPage}
      {hrefFor}
    />
  {/if}
</div>