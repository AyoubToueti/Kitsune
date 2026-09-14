import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";

import ListSection from "./ListSection.svelte";
import type { Anime } from "$lib/types";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
  };
}

function pending(): Promise<Anime[]> {
  return new Promise(() => {});
}

const base = { title: "Top airing", filter: "topAiring" as const };

describe("ListSection", () => {
  it("shows its heading while loading", () => {
    render(ListSection, { props: { ...base, load: pending } });

    // The heading is kept during loading so the grid does not reflow.
    expect(
      screen.getByRole("heading", { name: "Top airing" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("renders the loaded entries", async () => {
    const load = vi.fn().mockResolvedValue([anime(1, "A"), anime(2, "B")]);

    render(ListSection, { props: { ...base, load } });

    expect(await screen.findByText("A")).toBeInTheDocument();
    expect(screen.getByText("B")).toBeInTheDocument();
  });

  it("fetches once and ignores a later load prop", async () => {
    const first = vi.fn().mockResolvedValue([anime(1, "First")]);
    const second = vi.fn().mockResolvedValue([anime(2, "Second")]);

    const { rerender } = render(ListSection, {
      props: { ...base, load: first },
    });
    await screen.findByText("First");

    await rerender({ ...base, load: second });

    expect(first).toHaveBeenCalledTimes(1);
    expect(second).not.toHaveBeenCalled();
  });

  it("surfaces a failure without losing the heading", async () => {
    const load = vi.fn().mockRejectedValue("provider returned HTTP 429");

    render(ListSection, { props: { ...base, load } });

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

    render(ListSection, { props: { ...base, load } });
    await screen.findByText(/429/);

    screen.getByRole("button", { name: "Try again" }).click();

    expect(await screen.findByText("Recovered")).toBeInTheDocument();
    expect(load).toHaveBeenCalledTimes(2);
  });

  it("renders nothing for an empty result, rather than an empty block", async () => {
    const load = vi.fn().mockResolvedValue([]);

    render(ListSection, { props: { ...base, load } });

    await vi.waitFor(() => {
      expect(screen.queryByRole("heading")).toBeNull();
    });
  });

  it("passes the filter through to the View more link", async () => {
    const load = vi.fn().mockResolvedValue([anime(1, "A")]);

    render(ListSection, {
      props: { title: "Latest completed", filter: "latestCompleted", load },
    });
    await screen.findByText("A");

    expect(screen.getByRole("link", { name: /view more/i })).toHaveAttribute(
      "href",
      "/browse/latestCompleted",
    );
  });
});