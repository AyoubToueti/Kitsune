import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import { flushSync } from "svelte";

import { SCHEDULE_FUTURE_DAYS, SCHEDULE_PAST_DAYS } from "$lib/schedule";
import type { Anime, ScheduledEpisode } from "$lib/types";

const getScheduleMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/anime", async () => {
  const actual =
    await vi.importActual<typeof import("$lib/api/anime")>("$lib/api/anime");
  return { ...actual, getSchedule: getScheduleMock };
});

import ScheduleWidget from "./ScheduleWidget.svelte";

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

function entry(
  id: number,
  title: string,
  when: Date,
  episode?: number,
): ScheduledEpisode {
  return {
    anime: anime(id, title),
    airingAt: Math.floor(when.getTime() / 1000),
    episode,
  };
}

/** A promise that never settles, to hold the widget in its loading state. */
function pending(): Promise<ScheduledEpisode[]> {
  return new Promise(() => {});
}

/** All the day tabs, in order. */
function tabs(): HTMLElement[] {
  return Array.from(
    document.querySelectorAll<HTMLElement>("[data-testid='day-strip'] button"),
  );
}

const EXPECTED_DAYS = SCHEDULE_PAST_DAYS + 1 + SCHEDULE_FUTURE_DAYS;

beforeEach(() => {
  getScheduleMock.mockReset().mockResolvedValue([]);
});

describe("ScheduleWidget", () => {
  it("renders the whole day strip without waiting for data", () => {
    getScheduleMock.mockReturnValue(pending());

    render(ScheduleWidget);

    // The tabs are pure date arithmetic, so they appear before any response.
    expect(tabs()).toHaveLength(EXPECTED_DAYS);
  });

  it("renders at least 25 days", () => {
    render(ScheduleWidget);

    expect(tabs().length).toBeGreaterThanOrEqual(25);
  });

  it("selects today by default", () => {
    render(ScheduleWidget);

    const pressed = tabs().filter(
      (tab) => tab.getAttribute("aria-pressed") === "true",
    );
    expect(pressed).toHaveLength(1);
    // Today sits after the past days.
    expect(tabs().indexOf(pressed[0])).toBe(SCHEDULE_PAST_DAYS);
  });

  it("fetches only the selected day, not the whole window", async () => {
    render(ScheduleWidget);

    await waitFor(() => expect(getScheduleMock).toHaveBeenCalledTimes(1));

    const [from, to] = getScheduleMock.mock.calls[0];
    // A one-day window, which keeps the request inside AniList's perPage cap.
    expect(to - from).toBe(24 * 60 * 60 - 1);
  });

  it("shows a loading state while the day is in flight", () => {
    getScheduleMock.mockReturnValue(pending());

    render(ScheduleWidget);

    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("lists the day's broadcasts", async () => {
    const when = new Date();
    when.setHours(21, 30, 0, 0);
    getScheduleMock.mockResolvedValue([entry(1, "One Piece", when, 1178)]);

    render(ScheduleWidget);

    expect(await screen.findByText("One Piece")).toBeInTheDocument();
    expect(screen.getByText(/episode 1178/i)).toBeInTheDocument();
  });

  it("links each entry to its detail page", async () => {
    getScheduleMock.mockResolvedValue([entry(42, "One Piece", new Date(), 1)]);

    render(ScheduleWidget);

    expect(
      await screen.findByRole("link", { name: /one piece/i }),
    ).toHaveAttribute("href", "/anime/42");
  });

  it("omits the episode label when the provider gave none", async () => {
    getScheduleMock.mockResolvedValue([entry(1, "One Piece", new Date())]);

    render(ScheduleWidget);
    await screen.findByText("One Piece");

    expect(screen.queryByText(/^episode /i)).toBeNull();
  });

  it("says so when a day has nothing scheduled", async () => {
    getScheduleMock.mockResolvedValue([]);

    render(ScheduleWidget);

    expect(await screen.findByText(/nothing scheduled/i)).toBeInTheDocument();
  });

  it("surfaces a failure", async () => {
    getScheduleMock.mockRejectedValue("provider returned HTTP 429");

    render(ScheduleWidget);

    expect(await screen.findByText(/429/)).toBeInTheDocument();
  });

  it("fetches the newly selected day on demand", async () => {
    render(ScheduleWidget);
    await waitFor(() => expect(getScheduleMock).toHaveBeenCalledTimes(1));

    await fireEvent.click(tabs()[SCHEDULE_PAST_DAYS + 1]);

    await waitFor(() => expect(getScheduleMock).toHaveBeenCalledTimes(2));
  });

  it("marks the clicked tab as pressed", async () => {
    render(ScheduleWidget);
    const target = tabs()[SCHEDULE_PAST_DAYS + 2];

    await fireEvent.click(target);

    expect(target).toHaveAttribute("aria-pressed", "true");
  });

  it("does not refetch a day already loaded", async () => {
    render(ScheduleWidget);
    await waitFor(() => expect(getScheduleMock).toHaveBeenCalledTimes(1));

    // Away, then back again.
    await fireEvent.click(tabs()[SCHEDULE_PAST_DAYS + 1]);
    await waitFor(() => expect(getScheduleMock).toHaveBeenCalledTimes(2));
    await fireEvent.click(tabs()[SCHEDULE_PAST_DAYS]);

    // The cache means revisiting costs nothing.
    await waitFor(() => expect(getScheduleMock).toHaveBeenCalledTimes(2));
  });

  it("shows the selected day's own entries", async () => {
    const when = new Date();
    when.setHours(21, 0, 0, 0);

    getScheduleMock
      .mockResolvedValueOnce([entry(1, "Today Show", when, 1)])
      .mockResolvedValueOnce([entry(2, "Tomorrow Show", when, 2)]);

    render(ScheduleWidget);
    await screen.findByText("Today Show");

    await fireEvent.click(tabs()[SCHEDULE_PAST_DAYS + 1]);

    expect(await screen.findByText("Tomorrow Show")).toBeInTheDocument();
    expect(screen.queryByText("Today Show")).toBeNull();
  });

  it("gives each day a distinct tab label", async () => {
    render(ScheduleWidget);

    // Two days in the same month may share a weekday but never a date, so the
    // short date is what keeps the tabs distinguishable.
    const labels = tabs().map((tab) => tab.textContent?.trim() ?? "");
    expect(new Set(labels).size).toBeGreaterThan(20);
  });
});

/** The strip the arrows drive. */
function strip(): HTMLElement {
  return screen.getByTestId("day-strip");
}

function prev(): HTMLButtonElement {
  return screen.getByTestId("schedule-prev") as HTMLButtonElement;
}

function next(): HTMLButtonElement {
  return screen.getByTestId("schedule-next") as HTMLButtonElement;
}

/**
 * Give the strip the scroll geometry jsdom never lays out.
 *
 * Without this `clientWidth` and `scrollWidth` are both 0, so the strip reads
 * as fitting and both arrows stay disabled.
 */
function setWidths(clientWidth: number, scrollWidth: number): void {
  const el = strip();
  Object.defineProperty(el, "clientWidth", { value: clientWidth, configurable: true });
  Object.defineProperty(el, "scrollWidth", { value: scrollWidth, configurable: true });
  // The component re-reads on resize, which is the cheapest way to trigger it.
  // Flushed, because the handler writes `$state` and the assertions that follow
  // read the DOM.
  window.dispatchEvent(new Event("resize"));
  flushSync();
}

/** Fire the strip's scroll listener and flush the arrow state it updates. */
function emitScroll(): void {
  strip().dispatchEvent(new Event("scroll"));
  flushSync();
}

describe("ScheduleWidget day-strip arrows", () => {
  it("renders both arrows", () => {
    render(ScheduleWidget);

    expect(prev()).toBeInTheDocument();
    expect(next()).toBeInTheDocument();
  });

  it("disables both arrows when the strip fits", () => {
    render(ScheduleWidget);

    // No geometry to overflow means there is nowhere to go in either direction.
    setWidths(400, 400);

    expect(prev()).toBeDisabled();
    expect(next()).toBeDisabled();
  });

  it("enables only the right arrow at the start of an overflowing strip", () => {
    render(ScheduleWidget);

    setWidths(400, 1000);

    expect(prev()).toBeDisabled();
    expect(next()).toBeEnabled();
  });

  it("disables the right arrow once the end is reached", () => {
    render(ScheduleWidget);

    setWidths(400, 1000);
    // Scrolled to the very end: nothing further to the right.
    strip().scrollLeft = 600;
    emitScroll();

    expect(next()).toBeDisabled();
    expect(prev()).toBeEnabled();
  });

  it("eases the strip when the right arrow is clicked, rather than jumping", async () => {
    render(ScheduleWidget);
    setWidths(400, 1000);

    await fireEvent.click(next());

    // The glide starts synchronously, which the strip marks with a class so
    // scroll snapping can stand down for its duration. Asserting the glide
    // *began* is deterministic; the landing point is covered by the
    // `createStripDrag` release-glide tests, which own that arithmetic.
    expect(strip().className).toContain("strip-settling");
  });

  it("eases back when the left arrow is clicked", async () => {
    render(ScheduleWidget);
    setWidths(400, 1000);
    strip().scrollLeft = 400;
    emitScroll();

    await fireEvent.click(prev());

    expect(strip().className).toContain("strip-settling");
  });

  it("gives every tab the same width that fills the strip exactly", () => {
    render(ScheduleWidget);
    setWidths(400, 1000);

    const widths = tabs().map((tab) =>
      Number.parseFloat((tab as HTMLElement).style.width),
    );
    // One width for all, and a whole number of them spans the visible width.
    // At 400px the breakpoint table yields 4 tabs: 4 * 94 + 3 * 8 === 400, so
    // no half tab can show.
    expect(new Set(widths).size).toBe(1);
    expect(widths[0] * 4 + 8 * 3).toBeCloseTo(400, 5);
  });

  it("does not scroll when the arrow is disabled", async () => {
    render(ScheduleWidget);
    setWidths(400, 400);

    await fireEvent.click(next());

    expect(strip().scrollLeft).toBe(0);
  });

  it("suppresses text selection while a drag is in progress", async () => {
    render(ScheduleWidget);
    setWidths(400, 1000);

    await fireEvent.pointerDown(strip(), { clientX: 200, button: 0, pointerId: 1 });

    // Dragging must not select the tab labels as the pointer sweeps over them.
    expect(strip().className).toContain("select-none");
  });
});