import { describe, it, expect, vi, beforeEach } from "vitest";

const getUserListMock = vi.hoisted(() => vi.fn());
const onListChangedMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/auth", () => ({
  getUserList: getUserListMock,
  onListChanged: onListChangedMock,
}));

import { flushSync } from "svelte";

import { progressFor, progressLoaded, stopWatchProgress } from "./watch-progress.svelte";
import type { UserListEntry } from "./types";

function entry(id: number, progress: number): UserListEntry {
  return {
    anime: {
      id,
      provider: "anilist",
      title: { romaji: `T${id}` },
      genres: [],
      streamingEpisodes: [],
      relations: [],
      recommendations: [],
    },
    status: "current",
    progress,
    entryId: id,
  };
}

/** Let the fired-and-forgotten load promise resolve, then flush the runes. */
async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
}

beforeEach(() => {
  stopWatchProgress();
  getUserListMock.mockReset().mockResolvedValue([]);
  onListChangedMock.mockReset().mockResolvedValue(() => {});
});

describe("watch progress store", () => {
  it("answers 0 for a work with no progress", async () => {
    getUserListMock.mockResolvedValue([entry(1, 0)]);

    // Reading starts the store and triggers the load.
    expect(progressFor(1)).toBe(0);
    await settle();

    expect(progressFor(1)).toBe(0);
  });

  it("answers the stored progress after the load", async () => {
    getUserListMock.mockResolvedValue([entry(1, 5), entry(2, 12)]);

    progressFor(1);
    await settle();

    expect(progressFor(1)).toBe(5);
    expect(progressFor(2)).toBe(12);
  });

  it("answers 0 for a work that is not on the list", async () => {
    getUserListMock.mockResolvedValue([entry(1, 5)]);

    progressFor(1);
    await settle();

    expect(progressFor(99)).toBe(0);
  });

  it("keeps a planning entry at 0 out of the table", async () => {
    // A work on the list but never started is the same as absent for resume.
    getUserListMock.mockResolvedValue([entry(1, 0)]);

    progressFor(1);
    await settle();

    expect(progressFor(1)).toBe(0);
  });

  it("falls back to 0 when the list read fails", async () => {
    getUserListMock.mockRejectedValue(new Error("signed out"));

    expect(progressFor(1)).toBe(0);
    await settle();

    expect(progressFor(1)).toBe(0);
    expect(progressLoaded()).toBe(true);
  });

  it("refreshes when the list changes", async () => {
    getUserListMock.mockResolvedValue([entry(1, 3)]);

    let fire: (() => void) | undefined;
    onListChangedMock.mockImplementation((cb: () => void) => {
      fire = cb;
      return Promise.resolve(() => {});
    });

    progressFor(1);
    await settle();
    expect(progressFor(1)).toBe(3);

    getUserListMock.mockResolvedValue([entry(1, 4)]);
    fire!();
    await settle();

    expect(progressFor(1)).toBe(4);
  });
});