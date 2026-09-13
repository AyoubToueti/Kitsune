<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime, errorMessage } from "$lib/api/anime";
  import AnimeGrid from "$lib/components/AnimeGrid.svelte";
  import Pagination from "$lib/components/Pagination.svelte";
  import { clampPage } from "$lib/pagination";
  import type { AnimePage } from "$lib/types";

  /** AniList caps perPage at 50; the backend clamps anything larger. */
  const PER_PAGE = 30;

  // Reactive: the navbar submits a GET to /search, so the query string changes
  // without the component unmounting.
  const query = $derived((page.url.searchParams.get("q") ?? "").trim());

  /**
   * The requested page, sanitised to a positive integer.
   *
   * `Number("abc")` is NaN and `Number("0")` is 0; both mean "first page" here.
   * A page past the end is left to the provider, which returns an empty page
   * that the UI reports honestly rather than silently rewinding.
   */
  const requestedPage = $derived(
    Math.max(1, Math.floor(Number(page.url.searchParams.get("page") ?? "1")) || 1),
  );

  let result = $state<AnimePage | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  // Distinguishes "nothing yet" from "searched and found nothing", which need
  // different copy.
  let searched = $state("");

  $effect(() => {
    const q = query;
    const requested = requestedPage;

    if (q === "") {
      result = null;
      error = null;
      loading = false;
      searched = "";
      return;
    }

    // Guards against a stale response overwriting a newer one when the user
    // searches again quickly.
    let cancelled = false;
    loading = true;
    error = null;

    // Relevance first: a search should return the closest matches, not the most
    // popular titles that happen to contain the words.
    browseAnime({ search: q, sort: "searchMatch" }, requested, PER_PAGE)
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
   * Build a page link that keeps the search term.
   *
   * The target is clamped against the last page we know about, so the control
   * cannot link further into nowhere.
   */
  function hrefFor(target: number): string {
    const last = result?.pageInfo.lastPage ?? 1;
    const safe = clampPage(target, last);
    return `/search?q=${encodeURIComponent(searched)}&page=${safe}`;
  }
</script>

{#if query === ""}
  <p class="py-16 text-center text-ink-muted">Type something in the search box above.</p>
{:else if loading}
  <p class="py-16 text-center text-ink-muted">Searching…</p>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not run that search.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if result && result.items.length === 0}
  <p class="py-16 text-center text-ink-muted">
    No results for “{searched}”.
  </p>
{:else if result}
  <h1 class="mb-4 text-lg font-semibold tracking-tight">
    Results for “{searched}”
  </h1>

  <AnimeGrid anime={result.items} />
  <Pagination
    current={result.pageInfo.currentPage}
    last={result.pageInfo.lastPage}
    {hrefFor}
  />
{/if}