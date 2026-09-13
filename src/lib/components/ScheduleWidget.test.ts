import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

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

/** An entry airing `hoursFromNow` ahead, so it lands on a known local day. */
function soon(hours: number, id = 1, title = "Title"): ScheduledEpisode {
  const when = new Date();
  when.setHours(when.getHours() + hours);
  return {
    anime: anime(id, title),
    airingAt: Math.floor(when.getTime() / 1000),
    episode: 5,
  };
}

/** An entry explicitly on a future day, for exercising the tabs. */
function onDay(offsetDays: number, id = 1, title = "Title"): ScheduledEpisode {
  const when = new Date();
  when.setDate(when.getDate() + offsetDays);
  when.setHours(12, 0, 0, 0);
  return {
    anime: anime(id, title),
    airingAt: Math.floor(when.getTime() / 1000),
    episode: 3,
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
      screen.getByRole("heading", { name: /estimated schedule/i }),
    ).toBeInTheDocument();
  });

  it("links each entry to its detail page", () => {
    render(ScheduleWidget, { props: { entries: [soon(1, 42, "One Piece")] } });

    expect(screen.getByRole("link", { name: /one piece/i })).toHaveAttribute(
      "href",
      "/anime/42",
    );
  });

  it("shows the episode label when known", () => {
    render(ScheduleWidget, { props: { entries: [soon(1)] } });

    expect(screen.getByText(/episode 5/i)).toBeInTheDocument();
  });

  it("omits the episode label when the provider gave none", () => {
    const entry = { ...soon(1), episode: undefined };

    render(ScheduleWidget, { props: { entries: [entry] } });

    expect(screen.queryByText(/^episode /i)).toBeNull();
  });

  it("falls back to Untitled when no title form is present", () => {
    const entry: ScheduledEpisode = {
      anime: { ...anime(1, "x"), title: {} },
      airingAt: soon(1).airingAt,
    };

    render(ScheduleWidget, { props: { entries: [entry] } });

    expect(screen.getByRole("link", { name: /untitled/i })).toBeInTheDocument();
  });

  it("emits a machine-readable datetime on each time", () => {
    render(ScheduleWidget, { props: { entries: [soon(1)] } });

    expect(document.querySelector("time")).toHaveAttribute("datetime");
  });

  // --- day tabs -----------------------------------------------------------

  it("renders one tab per day", () => {
    render(ScheduleWidget, {
      props: { entries: [onDay(0), onDay(1), onDay(2)] },
    });

    expect(screen.getAllByRole("button")).toHaveLength(3);
  });

  it("selects the first day by default", () => {
    render(ScheduleWidget, {
      props: { entries: [onDay(0, 1, "First"), onDay(1, 2, "Second")] },
    });

    const [firstTab] = screen.getAllByRole("button");
    expect(firstTab).toHaveAttribute("aria-pressed", "true");
  });

  it("shows only the selected day's entries", () => {
    render(ScheduleWidget, {
      props: { entries: [onDay(0, 1, "First"), onDay(1, 2, "Second")] },
    });

    expect(screen.getByText("First")).toBeInTheDocument();
    expect(screen.queryByText("Second")).toBeNull();
  });

  it("switching tabs swaps the visible entries", async () => {
    render(ScheduleWidget, {
      props: { entries: [onDay(0, 1, "First"), onDay(1, 2, "Second")] },
    });

    await fireEvent.click(screen.getAllByRole("button")[1]);

    expect(screen.getByText("Second")).toBeInTheDocument();
    expect(screen.queryByText("First")).toBeNull();
  });

  it("marks the newly selected tab as pressed", async () => {
    render(ScheduleWidget, {
      props: { entries: [onDay(0), onDay(1)] },
    });

    const [first, second] = screen.getAllByRole("button");
    await fireEvent.click(second);

    expect(second).toHaveAttribute("aria-pressed", "true");
    expect(first).toHaveAttribute("aria-pressed", "false");
  });

  it("shows the weekday and short date on each tab", () => {
    render(ScheduleWidget, { props: { entries: [onDay(0)] } });

    const [tab] = screen.getAllByRole("button");
    // Two parts: weekday above, short date below.
    expect(tab.textContent).toMatch(/\w{3}/);
    expect(tab.textContent).toMatch(/\d/);
  });
});