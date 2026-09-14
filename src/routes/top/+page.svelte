<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime, errorMessage } from "$lib/api/anime";
  import Pagination from "$lib/components/Pagination.svelte";
  import TopListSkeleton from "$lib/components/TopListSkeleton.svelte";
  import TopList from "$lib/components/TopList.svelte";
  import { filterHref } from "$lib/filter";
  import { clampPage } from "$lib/pagination";
  import type { AnimePage } from "$lib/types";

  /**
   * Rows per page.
   *
   * 100 rows in one response is over AniList's perPage cap of 50, so the list
   * is paged rather than fetched whole.
   */
  const PER_PAGE = 25;

  const params = $derived(page.url.searchParams);
  const requestedPage = $derived(
    Math.max(1, Math.floor(Number(params.get("page") ?? "1")) || 1),
  );

  let result = $state<AnimePage | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    const requested = requestedPage;

    // Guards against a stale response overwriting a newer one when the user
    // pages quickly.
    let cancelled = false;
    loading = true;
    error = null;

    // Score order, matching the preview on the home page so the ranking does
    // not change meaning between the two.
    browseAnime({ sort: "score" }, requested, PER_PAGE)
      .then((found) => {
        if (cancelled) return;
        result = found;
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

  /** Rank of the first row on the current page, so numbering continues. */
  const startRank = $derived(
    result === null ? 1 : (result.pageInfo.currentPage - 1) * PER_PAGE + 1,
  );

  function hrefFor(target: number): string {
    const last = result?.pageInfo.lastPage ?? 1;
    return filterHref(params, clampPage(target, last), "/top");
  }
</script>

<h1 class="mb-4 text-lg font-semibold tracking-tight">Top rated anime</h1>

{#if loading}
  <div class="rounded-xl border border-border-subtle py-1">
    <TopListSkeleton count={PER_PAGE} />
  </div>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load the ranking.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if result}
  <div class="rounded-xl border border-border-subtle py-1">
    <TopList anime={result.items} {startRank} />
  </div>

  <Pagination
    current={result.pageInfo.currentPage}
    last={result.pageInfo.lastPage}
    {hrefFor}
  />
{/if}