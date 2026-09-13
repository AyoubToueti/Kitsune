import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";

import Shelf from "./Shelf.svelte";
import type { Anime } from "$lib/types";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
  };
}

/** A promise that never settles, to hold the shelf in its loading state. */
function pending(): Promise<Anime[]> {
  return new Promise(() => {});
}

describe("Shelf", () => {
  it("shows its heading while loading", () => {
    render(Shelf, { props: { title: "Top airing", load: pending } });

    // The heading is present immediately so the page does not shift as rows
    // arrive.
    expect(
      screen.getByRole("heading", { name: "Top airing" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("renders the loaded titles", async () => {
    const load = vi.fn().mockResolvedValue([anime(1, "A"), anime(2, "B")]);

    render(Shelf, { props: { title: "Top airing", load } });

    expect(await screen.findByText("A")).toBeInTheDocument();
    expect(screen.getByText("B")).toBeInTheDocument();
  });

  it("fetches once and ignores a later load prop, since shelves are one-shot", async () => {
    // `onMount` runs once. An `$effect` would treat a changed `load` as a new
    // dependency and re-fetch, so passing a second function here is what pins
    // the difference. Without the re-render this test would pass either way.
    const first = vi.fn().mockResolvedValue([anime(1, "First")]);
    const second = vi.fn().mockResolvedValue([anime(2, "Second")]);

    const { rerender } = render(Shelf, {
      props: { title: "Top airing", load: first },
    });
    await screen.findByText("First");

    await rerender({ title: "Top airing", load: second });

    expect(first).toHaveBeenCalledTimes(1);
    expect(second).not.toHaveBeenCalled();
  });

  it("surfaces a failure without losing the heading", async () => {
    const load = vi.fn().mockRejectedValue("provider returned HTTP 429");

    render(Shelf, { props: { title: "Top airing", load } });

    expect(
      screen.getByRole("heading", { name: "Top airing" }),
    ).toBeInTheDocument();
    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("offers a retry that re-requests", async () => {
    const load = vi
      .fn()
      .mockRejectedValueOnce("provider returned HTTP 429")
      .mockResolvedValueOnce([anime(1, "Recovered")]);

    render(Shelf, { props: { title: "Top airing", load } });
    await screen.findByText(/429/);

    screen.getByRole("button", { name: "Try again" }).click();

    expect(await screen.findByText("Recovered")).toBeInTheDocument();
    expect(load).toHaveBeenCalledTimes(2);
  });

  it("renders nothing for an empty result, rather than an empty heading", async () => {
    const load = vi.fn().mockResolvedValue([]);

    render(Shelf, { props: { title: "Top airing", load } });
    await screen.findByText(/loading/i).catch(() => {});

    // PosterRow suppresses itself for an empty list, so the heading goes too.
    await vi.waitFor(() => {
      expect(screen.queryByRole("heading")).toBeNull();
    });
  });

  it("passes the numbered flag through to the row", async () => {
    const load = vi.fn().mockResolvedValue([anime(1, "A"), anime(2, "B")]);

    render(Shelf, { props: { title: "Top 10", load, numbered: true } });
    await screen.findByText("A");

    expect(screen.getAllByTestId("rank-badge")).toHaveLength(2);
  });
});