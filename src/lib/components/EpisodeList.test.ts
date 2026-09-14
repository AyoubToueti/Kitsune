import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import type { StreamingEpisode } from "$lib/types";

const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: openUrlMock,
}));

import EpisodeList from "./EpisodeList.svelte";

function episode(overrides: Partial<StreamingEpisode> = {}): StreamingEpisode {
  return {
    url: "https://crunchyroll.example/ep1",
    title: "Episode 1",
    site: "Crunchyroll",
    thumbnail: "https://example.test/ep1.jpg",
    ...overrides,
  };
}

describe("EpisodeList", () => {
  it("renders one card per episode", () => {
    render(EpisodeList, {
      props: {
        episodes: [
          episode({ title: "Episode 1" }),
          episode({ title: "Episode 2", url: "https://crunchyroll.example/ep2" }),
        ],
      },
    });

    expect(screen.getAllByRole("button")).toHaveLength(2);
  });

  it("renders the thumbnail for each episode", () => {
    render(EpisodeList, {
      props: { episodes: [episode({ thumbnail: "https://example.test/thumb.jpg" })] },
    });

    // The image is decorative; the button carries the accessible name.
    expect(screen.getByRole("button")).toHaveAccessibleName("Episode 1");
    expect(document.querySelector("img")?.getAttribute("src")).toBe(
      "https://example.test/thumb.jpg",
    );
  });

  it("falls back to the site when the title is absent", () => {
    render(EpisodeList, {
      props: { episodes: [episode({ title: undefined, site: "Crunchyroll" })] },
    });

    expect(screen.getByRole("button")).toHaveAccessibleName("Crunchyroll");
  });

  it("falls back to the url when title and site are absent", () => {
    render(EpisodeList, {
      props: {
        episodes: [episode({ title: undefined, site: undefined, url: "https://x.test/1" })],
      },
    });

    expect(screen.getByRole("button")).toHaveAccessibleName("https://x.test/1");
  });

  it("clicking opens the episode url", async () => {
    render(EpisodeList, {
      props: { episodes: [episode({ url: "https://example.test/watch/1" })] },
    });

    await fireEvent.click(screen.getByRole("button"));

    expect(openUrlMock).toHaveBeenCalledWith("https://example.test/watch/1");
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
    expect(screen.getByRole("button")).toHaveAccessibleName("Episode 1");
    expect(document.querySelector("img")).toBeNull();
  });
});