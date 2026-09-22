import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/svelte";

import type { Episode } from "$lib/episodes";

const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: openUrlMock,
}));

import EpisodeList from "./EpisodeList.svelte";

function episode(overrides: Partial<Episode> = {}): Episode {
  return {
    url: "https://crunchyroll.example/ep1",
    title: "Episode 1",
    site: "Crunchyroll",
    thumbnail: "https://example.test/ep1.jpg",
    ...overrides,
  };
}

/** The episode cards only; the jump form's Go button is not one of these. */
function episodeCards() {
  return within(screen.getByTestId("episode-list")).getAllByRole("button");
}

describe("aired count label", () => {
  it("shows the count when episodes are still to air", () => {
    render(EpisodeList, {
      props: {
        episodes: [episode({ title: "Episode 1" })],
        airedCount: 1,
        totalCount: 12,
      },
    });

    expect(screen.getByText("1 of 12 aired")).toBeInTheDocument();
  });

  it("hides the count for a finished work", () => {
    render(EpisodeList, {
      props: {
        episodes: [episode({ title: "Episode 1" })],
        airedCount: 12,
        totalCount: 12,
      },
    });

    expect(screen.queryByText(/aired/)).toBeNull();
  });

  it("hides the count when either number is absent", () => {
    render(EpisodeList, {
      props: { episodes: [episode({ title: "Episode 1" })], airedCount: 1 },
    });

    expect(screen.queryByText(/aired/)).toBeNull();
  });
});

describe("EpisodeList", () => {
  beforeEach(() => {
    // jsdom has no layout engine, so smooth scrolling is a no-op stub.
    Element.prototype.scrollIntoView = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders one card per episode", () => {
    render(EpisodeList, {
      props: {
        episodes: [
          episode({ title: "Episode 1" }),
          episode({ title: "Episode 2", url: "https://crunchyroll.example/ep2" }),
        ],
      },
    });

    expect(episodeCards()).toHaveLength(2);
  });

  it("renders the thumbnail for each episode", () => {
    render(EpisodeList, {
      props: { episodes: [episode({ thumbnail: "https://example.test/thumb.jpg" })] },
    });

    // The image is decorative; the button carries the accessible name.
    expect(episodeCards()[0]).toHaveAccessibleName("Episode 1");
    expect(document.querySelector("img")?.getAttribute("src")).toBe(
      "https://example.test/thumb.jpg",
    );
  });

  it("falls back to the site when the title is absent", () => {
    render(EpisodeList, {
      props: { episodes: [episode({ title: undefined, site: "Crunchyroll" })] },
    });

    expect(episodeCards()[0]).toHaveAccessibleName("Crunchyroll");
  });

  it("falls back to the url when title and site are absent", () => {
    render(EpisodeList, {
      props: {
        episodes: [episode({ title: undefined, site: undefined, url: "https://x.test/1" })],
      },
    });

    expect(episodeCards()[0]).toHaveAccessibleName("https://x.test/1");
  });

  it("prefixes a numbered episode with its number", () => {
    render(EpisodeList, {
      props: {
        episodes: [episode({ number: 1, title: "The Journey's End" })],
      },
    });

    // The number is the load-bearing part: the title alone does not say where
    // in the run the reader is.
    expect(episodeCards()[0]).toHaveAccessibleName(
      "Episode 1 - The Journey's End",
    );
  });

  it("prefixes the fallback caption too", () => {
    render(EpisodeList, {
      props: {
        episodes: [episode({ number: 2, title: undefined, site: "Crunchyroll" })],
      },
    });

    expect(episodeCards()[0]).toHaveAccessibleName("Episode 2 - Crunchyroll");
  });

  it("does not repeat the number on a padded entry", () => {
    // A padded catalogue entry is titled `Episode 7`, so prefixing it again
    // would read "Episode 7 - Episode 7".
    render(EpisodeList, {
      props: { episodes: [episode({ number: 7, title: "Episode 7" })] },
    });

    expect(episodeCards()[0]).toHaveAccessibleName("Episode 7");
  });

  it("leaves a title that spells its own number alone", () => {
    render(EpisodeList, {
      props: {
        episodes: [episode({ number: 3, title: "Episode 3 - A Name" })],
      },
    });

    expect(episodeCards()[0]).toHaveAccessibleName("Episode 3 - A Name");
  });

  it("still prefixes a title whose number is part of the name", () => {
    // The trailing number is the episode's name, not an announcement, so the
    // prefix is still needed.
    render(EpisodeList, {
      props: { episodes: [episode({ number: 4, title: "The Journey's End 2" })] },
    });

    expect(episodeCards()[0]).toHaveAccessibleName(
      "Episode 4 - The Journey's End 2",
    );
  });

  it("leaves an unnumbered episode's caption alone", () => {
    render(EpisodeList, {
      props: { episodes: [episode({ title: "The Journey's End" })] },
    });

    expect(episodeCards()[0]).toHaveAccessibleName("The Journey's End");
  });

  it("clicking opens the episode url", async () => {
    render(EpisodeList, {
      props: { episodes: [episode({ url: "https://example.test/watch/1" })] },
    });

    await fireEvent.click(episodeCards()[0]);

    expect(openUrlMock).toHaveBeenCalledWith("https://example.test/watch/1");
  });

  it("calls onSelect with the index instead of opening a url", async () => {
    const onSelect = vi.fn();
    render(EpisodeList, {
      props: {
        episodes: [
          episode({ title: "Episode 1" }),
          episode({ title: "Episode 2", url: "https://x.test/2" }),
        ],
        onSelect,
      },
    });

    await fireEvent.click(episodeCards()[1]);

    // The watch page plays the reader's own file, so the licensed link must
    // not be opened as well.
    expect(onSelect).toHaveBeenCalledWith(1);
    expect(openUrlMock).not.toHaveBeenCalled();
  });

  it("marks the selected episode as current", () => {
    render(EpisodeList, {
      props: {
        episodes: [
          episode({ title: "Episode 1" }),
          episode({ title: "Episode 2", url: "https://x.test/2" }),
        ],
        selected: 1,
      },
    });

    expect(episodeCards()[1]).toHaveAttribute("aria-current", "true");
    expect(episodeCards()[0]).not.toHaveAttribute("aria-current");
  });

  it("renders nothing when there are no episodes", () => {
    const { container } = render(EpisodeList, { props: { episodes: [] } });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /episodes/i })).toBeNull();
  });

  it("renders nothing when the prop is undefined", () => {
    const { container } = render(EpisodeList, { props: {} });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
  });

  it("still renders a card when the thumbnail is missing", () => {
    render(EpisodeList, {
      props: { episodes: [episode({ thumbnail: undefined })] },
    });

    // No image, but the button and its caption survive.
    expect(episodeCards()[0]).toHaveAccessibleName("Episode 1");
    expect(document.querySelector("img")).toBeNull();
  });

  it("wraps the grid in a scrollable container", () => {
    render(EpisodeList, { props: { episodes: [episode()] } });

    const scroller = screen.getByTestId("episode-scroller");
    expect(scroller.className).toContain("overflow-y-auto");
  });

  it("jumps to an episode by number and highlights it", async () => {
    render(EpisodeList, {
      props: {
        episodes: [
          episode({ title: "Episode 1", url: "https://x.test/1" }),
          episode({ title: "Episode 2", url: "https://x.test/2" }),
          episode({ title: "Episode 3", url: "https://x.test/3" }),
        ],
      },
    });

    await fireEvent.input(screen.getByLabelText("Jump to episode number"), {
      target: { value: "3" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Go" }));

    const cards = episodeCards();
    expect(cards[2].getAttribute("data-highlighted")).toBe("true");
    expect(cards[0].getAttribute("data-highlighted")).toBeNull();
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
  });

  it("matches an episode number embedded in the url when the title has none", async () => {
    render(EpisodeList, {
      props: {
        episodes: [
          episode({ title: undefined, site: "Crunchyroll", url: "https://x.test/episode-5" }),
        ],
      },
    });

    await fireEvent.input(screen.getByLabelText("Jump to episode number"), {
      target: { value: "5" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Go" }));

    expect(episodeCards()[0].getAttribute("data-highlighted")).toBe("true");
  });

  it("reports a number that matches no episode", async () => {
    render(EpisodeList, { props: { episodes: [episode({ title: "Episode 1" })] } });

    await fireEvent.input(screen.getByLabelText("Jump to episode number"), {
      target: { value: "99" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Go" }));

    expect(screen.getByRole("status")).toHaveTextContent(/no episode 99/i);
    expect(episodeCards()[0].getAttribute("data-highlighted")).toBeNull();
  });

  it("clears the not-found message once typing resumes", async () => {
    render(EpisodeList, { props: { episodes: [episode({ title: "Episode 1" })] } });

    const input = screen.getByLabelText("Jump to episode number");
    await fireEvent.input(input, { target: { value: "99" } });
    await fireEvent.click(screen.getByRole("button", { name: "Go" }));
    expect(screen.getByRole("status")).toBeInTheDocument();

    await fireEvent.input(input, { target: { value: "9" } });
    expect(screen.queryByRole("status")).toBeNull();
  });
});