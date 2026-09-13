<script lang="ts">
  import { page } from "$app/state";

  import { browseAnime, errorMessage } from "$lib/api/anime";
  import AnimeGrid from "$lib/components/AnimeGrid.svelte";
  import Pagination from "$lib/components/Pagination.svelte";
  import { clampPage } from "$lib/pagination";
  import type { AnimePage } from "$lib/types";

  const PER_PAGE = 30;

  // SvelteKit decodes route params, so a "Slice%20of%20Life" path arrives here
  // already readable. Decoding again could corrupt a name containing a literal
  // percent, so it is used as-is.
  const genre = $derived(page.params.name ?? "");

  /** Sanitised to a positive integer; see the search page for the reasoning. */
  const requestedPage = $derived(
    Math.max(1, Math.floor(Number(page.url.searchParams.get("page") ?? "1")) || 1),
  );

  let result = $state<AnimePage | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let loadedFor = $state("");

  $effect(() => {
    const current = genre;
    const requested = requestedPage;

    if (current === "") {
      result = null;
      error = null;
      loading = false;
      loadedFor = "";
      return;
    }

    // Guards against a stale response overwriting a newer one when the user
    // navigates between genres quickly.
    let cancelled = false;
    loading = true;
    error = null;

    browseAnime({ genres: [current], sort: "popularity" }, requested, PER_PAGE)
      .then((found) => {
        if (cancelled) return;
        result = found;
        loadedFor = current;
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
   * Keep the genre in the path while moving between pages.
   *
   * Page 1 is written without a query string, so the canonical URL for a genre
   * stays clean and the first page is not duplicated at `?page=1`.
   */
  function hrefFor(target: number): string {
    const last = result?.pageInfo.lastPage ?? 1;
    const safe = clampPage(target, last);
    const base = `/genre/${encodeURIComponent(loadedFor)}`;
    return safe === 1 ? base : `${base}?page=${safe}`;
  }
</script>

{#if genre === ""}
  <p class="py-16 text-center text-ink-muted">No genre selected.</p>
{:else if loading}
  <p class="py-16 text-center text-ink-muted">Loading…</p>
{:else if error}
  <div class="py-16 text-center">
    <p class="text-ink">Could not load that genre.</p>
    <p class="mt-2 text-sm text-ink-faint">{error}</p>
  </div>
{:else if result && result.items.length === 0}
  <p class="py-16 text-center text-ink-muted">
    Nothing found in “{loadedFor}”.
  </p>
{:else if result}
  <h1 class="mb-4 text-lg font-semibold tracking-tight">{loadedFor}</h1>

  <AnimeGrid anime={result.items} />
  <Pagination
    current={result.pageInfo.currentPage}
    last={result.pageInfo.lastPage}
    {hrefFor}
  />
{/if}