import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import type { StreamingEpisode } from "$lib/types";

const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: openUrlMock,
}));

import StreamingLinks from "./StreamingLinks.svelte";

function episode(overrides: Partial<StreamingEpisode> = {}): StreamingEpisode {
  return {
    url: "https://crunchyroll.example/test",
    site: "Crunchyroll",
    ...overrides,
  };
}

describe("StreamingLinks", () => {
  it("renders one entry per episode", () => {
    render(StreamingLinks, {
      props: {
        episodes: [
          episode({ site: "Crunchyroll" }),
          episode({ site: "Netflix", url: "https://netflix.example/test" }),
        ],
      },
    });

    expect(screen.getByText("Crunchyroll")).toBeInTheDocument();
    expect(screen.getByText("Netflix")).toBeInTheDocument();
  });

  it("falls back to title when site is absent", () => {
    render(StreamingLinks, {
      props: {
        episodes: [episode({ site: undefined, title: "Episode 1" })],
      },
    });

    expect(screen.getByText("Episode 1")).toBeInTheDocument();
  });

  it("falls back to url when both site and title are absent", () => {
    render(StreamingLinks, {
      props: {
        episodes: [episode({ site: undefined, title: undefined })],
      },
    });

    expect(screen.getByText("https://crunchyroll.example/test")).toBeInTheDocument();
  });

  it("clicking calls openUrl with the episode url", async () => {
    render(StreamingLinks, {
      props: {
        episodes: [episode({ url: "https://example.test/watch" })],
      },
    });

    await fireEvent.click(screen.getByRole("button"));

    expect(openUrlMock).toHaveBeenCalledWith("https://example.test/watch");
  });

  it("renders nothing when list is empty", () => {
    const { container } = render(StreamingLinks, {
      props: { episodes: [] },
    });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
  });

  it("renders nothing when episodes prop is undefined", () => {
    const { container } = render(StreamingLinks, {
      props: {},
    });

    expect(container.querySelector("ul")).not.toBeInTheDocument();
  });
});