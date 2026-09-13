import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";

import ScheduleWidget from "./ScheduleWidget.svelte";
import type { Anime, ScheduledEpisode } from "$lib/types";

function anime(id: number, title: string): Anime {
  return {
    id,
    provider: "anilist",
    title: { romaji: title },
    genres: [],
    streamingEpisodes: [],
  };
}

/**
 * An entry airing `hoursFromNow` in the future, so it lands on the current
 * local day regardless of when the suite runs.
 */
function soon(hours: number, id = 1, title = "Title"): ScheduledEpisode {
  const when = new Date();
  when.setHours(when.getHours() + hours);
  return {
    anime: anime(id, title),
    airingAt: Math.floor(when.getTime() / 1000),
    episode: 5,
  };
}

describe("ScheduleWidget", () => {
  it("renders nothing for an empty list", () => {
    render(ScheduleWidget, { props: { entries: [] } });

    expect(screen.queryByRole("heading")).toBeNull();
  });

  it("shows the section heading when there is data", () => {
    render(ScheduleWidget, { props: { entries: [soon(1)] } });

    expect(
      screen.getByRole("heading", { name: /airing soon/i }),
    ).toBeInTheDocument();
  });

  it("links each entry to its detail page", () => {
    render(ScheduleWidget, { props: { entries: [soon(1, 42, "One Piece")] } });

    expect(screen.getByRole("link", { name: "One Piece" })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("shows the episode number when known", () => {
    render(ScheduleWidget, { props: { entries: [soon(1)] } });

    expect(screen.getByText(/EP 5/)).toBeInTheDocument();
  });

  it("omits the episode label when the provider gave none", () => {
    const entry = { ...soon(1), episode: undefined };

    render(ScheduleWidget, { props: { entries: [entry] } });

    expect(screen.queryByText(/^EP /)).toBeNull();
  });

  it("falls back to Untitled when no title form is present", () => {
    const entry: ScheduledEpisode = {
      anime: { ...anime(1, "x"), title: {} },
      airingAt: soon(1).airingAt,
    };

    render(ScheduleWidget, { props: { entries: [entry] } });

    expect(screen.getByRole("link", { name: "Untitled" })).toBeInTheDocument();
  });

  it("groups entries under a day heading", () => {
    render(ScheduleWidget, { props: { entries: [soon(1), soon(2)] } });

    // Both land on the same local day, so there is one day heading.
    expect(screen.getByText("Today")).toBeInTheDocument();
  });

  it("emits a machine-readable datetime on each time", () => {
    render(ScheduleWidget, { props: { entries: [soon(1)] } });

    const time = document.querySelector("time");
    expect(time).toHaveAttribute("datetime");
  });
});