import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";

import type { Anime, ContinueWatchingItem } from "$lib/types";

const setListEntryMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
vi.mock("$lib/api/auth", () => ({
  setListEntry: setListEntryMock,
}));

import ContinueWatchingCard from "./ContinueWatchingCard.svelte";

function anime(overrides: Partial<Anime> = {}): Anime {
  return {
    id: 21,
    provider: "anilist",
    title: { romaji: "One Piece" },
    genres: [],
    streamingEpisodes: [],
    relations: [],
    recommendations: [],
    ...overrides,
  };
}

function entry(overrides: Partial<ContinueWatchingItem> = {}): ContinueWatchingItem {
  return { anime: anime(), progress: 0, ...overrides };
}

beforeEach(() => {
  setListEntryMock.mockClear();
});

describe("ContinueWatchingCard", () => {
  it("shows the title and the resume episode", () => {
    render(ContinueWatchingCard, { props: { entry: entry({ progress: 7 }) } });

    expect(screen.getByText("One Piece")).toBeInTheDocument();
    expect(screen.getByText("Ep. 7")).toBeInTheDocument();
  });

  it("resumes at the next episode after the one watched", () => {
    render(ContinueWatchingCard, { props: { entry: entry({ progress: 3 }) } });

    // Progress 3 -> index 2 for the watch page's `?ep=`.
    expect(screen.getByTestId("resume")).toHaveAttribute(
      "href",
      "/anime/21?ep=2",
    );
  });

  it("starts from the first episode when nothing has been watched", () => {
    render(ContinueWatchingCard, { props: { entry: entry({ progress: 0 }) } });

    expect(screen.getByTestId("resume")).toHaveAttribute(
      "href",
      "/anime/21?ep=0",
    );
    expect(screen.getByText("Ep. 1")).toBeInTheDocument();
  });

  it("shows watched percentage and episode position when the total is known", () => {
    render(ContinueWatchingCard, {
      props: {
        entry: entry({
          progress: 7,
          anime: anime({ episodeCount: 10 }),
        }),
      },
    });

    expect(screen.getByText("70% completed")).toBeInTheDocument();
    expect(screen.getByText("Ep 7 / 10")).toBeInTheDocument();
  });

  it("omits the percentage when the provider gave no episode count", () => {
    render(ContinueWatchingCard, { props: { entry: entry({ progress: 4 }) } });

    // No episode count -> an honest "In progress" rather than a fake number.
    expect(screen.getByText("In progress")).toBeInTheDocument();
    expect(screen.getByText("Ep 4")).toBeInTheDocument();
  });

  it("bumps the stored progress by one when +1 is pressed", async () => {
    render(ContinueWatchingCard, { props: { entry: entry({ progress: 4 }) } });

    await fireEvent.click(screen.getByTestId("skip-episode"));

    // Kept on the list as "current", advanced by exactly one.
    expect(setListEntryMock).toHaveBeenCalledWith(21, "current", 5);
  });

  it("does not fire two writes when +1 is pressed twice quickly", async () => {
    let release: () => void = () => {};
    setListEntryMock.mockImplementation(
      () => new Promise<void>((r) => (release = r)),
    );

    render(ContinueWatchingCard, { props: { entry: entry({ progress: 4 }) } });

    const button = screen.getByTestId("skip-episode");
    await fireEvent.click(button);
    await fireEvent.click(button);

    // The second press is ignored while the first is in flight.
    expect(setListEntryMock).toHaveBeenCalledTimes(1);

    release();
  });

  it("renders the always-visible info trigger", () => {
    render(ContinueWatchingCard, { props: { entry: entry() } });

    const button = screen.getByTestId("info-trigger");
    expect(button.className).not.toMatch(/opacity-0/);
  });

  it("slides the action bar in on hover via a group class", () => {
    render(ContinueWatchingCard, { props: { entry: entry() } });

    const bar = screen.getByTestId("resume").parentElement!;
    expect(bar.className).toMatch(/translate-y-full/);
    expect(bar.className).toMatch(/group-hover:translate-y-0/);
  });
});