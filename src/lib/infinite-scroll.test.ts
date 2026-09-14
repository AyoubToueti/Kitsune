import { describe, it, expect, vi } from "vitest";

import { harness, settle } from "../test/infinite-scroll-harness.svelte";
import type { Anime, AnimePage } from "./types";

function anime(id: number): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: `Title ${id}` },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
  };
}

function page(ids: number[], currentPage: number, lastPage: number): AnimePage {
  return {
    items: ids.map(anime),
    pageInfo: {
      total: lastPage * ids.length,
      currentPage,
      lastPage,
      hasNextPage: currentPage < lastPage,
    },
  };
}

describe("createInfiniteScroll", () => {
  it("loads the first page on creation", async () => {
    const fetchPage = vi.fn().mockResolvedValue(page([1, 2], 1, 3));
    const h = harness(fetchPage);

    await settle();

    expect(fetchPage).toHaveBeenCalledWith(1);
    expect(h.scroll.items.map((a) => a.id)).toEqual([1, 2]);
    expect(h.scroll.loading).toBe(false);
    expect(h.scroll.hasMore).toBe(true);

    h.destroy();
  });

  it("appends the next page rather than replacing", async () => {
    const fetchPage = vi
      .fn()
      .mockResolvedValueOnce(page([1, 2], 1, 3))
      .mockResolvedValueOnce(page([3, 4], 2, 3));
    const h = harness(fetchPage);

    await settle();
    h.scroll.loadMore();
    await settle();

    // Replacing would make the list jump as the reader scrolled.
    expect(h.scroll.items.map((a) => a.id)).toEqual([1, 2, 3, 4]);

    h.destroy();
  });

  it("stops when the last page is reached", async () => {
    const fetchPage = vi.fn().mockResolvedValue(page([1], 1, 1));
    const h = harness(fetchPage);

    await settle();
    h.scroll.loadMore();
    await settle();

    expect(h.scroll.hasMore).toBe(false);
    expect(fetchPage).toHaveBeenCalledTimes(1);

    h.destroy();
  });

  it("ignores a second request while one is in flight", async () => {
    let release: (value: AnimePage) => void = () => {};
    const fetchPage = vi.fn(
      () => new Promise<AnimePage>((resolve) => (release = resolve)),
    );
    const h = harness(fetchPage);

    // The sentinel can fire repeatedly while the reader sits near the bottom.
    h.scroll.loadMore();
    h.scroll.loadMore();
    h.scroll.loadMore();

    expect(fetchPage).toHaveBeenCalledTimes(1);

    release(page([1], 1, 1));
    await settle();

    h.destroy();
  });

  it("resets and refetches when the query changes", async () => {
    const fetchPage = vi
      .fn()
      .mockResolvedValueOnce(page([1, 2], 1, 5))
      .mockResolvedValueOnce(page([9], 1, 1));
    const h = harness(fetchPage);

    await settle();
    expect(h.scroll.items).toHaveLength(2);

    h.setKey("different query");
    await settle();

    // The old results must be gone: keeping them would mix two queries.
    expect(h.scroll.items.map((a) => a.id)).toEqual([9]);
    expect(fetchPage).toHaveBeenLastCalledWith(1);

    h.destroy();
  });

  /// A slow request for the previous filter must not append to the new list.
  /// Every other route here guards this with a local `cancelled` flag; the
  /// composable does it with a generation counter.
  it("discards a response belonging to a superseded query", async () => {
    let releaseFirst: (value: AnimePage) => void = () => {};
    const fetchPage = vi
      .fn()
      .mockImplementationOnce(
        () => new Promise<AnimePage>((resolve) => (releaseFirst = resolve)),
      )
      .mockResolvedValueOnce(page([9], 1, 1));
    const h = harness(fetchPage);

    // The first request is still pending when the query changes.
    h.setKey("different query");
    await settle();
    expect(h.scroll.items.map((a) => a.id)).toEqual([9]);

    // Now let the stale request land. Its results belong to the old query.
    releaseFirst(page([1, 2, 3], 1, 9));
    await settle();

    expect(h.scroll.items.map((a) => a.id)).toEqual([9]);
    // And it must not have moved the page counter for the new query.
    expect(h.scroll.hasMore).toBe(false);

    h.destroy();
  });

  it("surfaces the failure and stops loading more", async () => {
    const fetchPage = vi.fn().mockRejectedValue("provider returned HTTP 429");
    const h = harness(fetchPage);

    await settle();

    expect(h.scroll.error).toContain("429");
    expect(h.scroll.loading).toBe(false);

    // A failed request leaves the page count where it was, so without the error
    // guard the sentinel would retry forever.
    h.scroll.loadMore();
    await settle();
    expect(fetchPage).toHaveBeenCalledTimes(1);

    h.destroy();
  });

  it("retries the same page after a failure", async () => {
    const fetchPage = vi
      .fn()
      .mockRejectedValueOnce("boom")
      .mockResolvedValueOnce(page([1], 1, 1));
    const h = harness(fetchPage);

    await settle();
    expect(h.scroll.error).not.toBeNull();

    h.scroll.retry();
    await settle();

    // The retry re-requests page 1, not page 2.
    expect(fetchPage).toHaveBeenLastCalledWith(1);
    expect(h.scroll.items.map((a) => a.id)).toEqual([1]);
    expect(h.scroll.error).toBeNull();

    h.destroy();
  });

  it("offers a no-op sentinel without IntersectionObserver", () => {
    const h = harness(vi.fn().mockResolvedValue(page([], 1, 1)));

    // jsdom has no observer, so the action must degrade rather than throw.
    expect(() =>
      h.scroll.sentinel(document.createElement("div")),
    ).not.toThrow();

    h.destroy();
  });
});