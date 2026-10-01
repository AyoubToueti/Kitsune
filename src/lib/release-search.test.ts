import { describe, it, expect, vi, beforeEach } from "vitest";

import { harness, settle, type Harness } from "../test/release-search-harness.svelte";

const searchReleasesMock = vi.hoisted(() => vi.fn());
const probeReleasesMock = vi.hoisted(() => vi.fn());
const onProbeResultMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/releases", () => ({
  searchReleases: searchReleasesMock,
  probeReleases: probeReleasesMock,
  onProbeResult: onProbeResultMock,
}));

import type { ProbeOutcome, Release } from "$lib/types";
import type { ReleaseRequest } from "$lib/release-search.svelte";

function release(title: string): Release {
  return {
    title,
    indexer: "nyaa",
    magnetUri: "magnet:?xt=urn:btih:abc",
    resolution: "1080p",
    source: "webdl",
    remux: false,
    trusted: false,
    parsed: {},
    score: 10,
  };
}

let current: Harness | null = null;

beforeEach(() => {
  searchReleasesMock.mockReset();
  probeReleasesMock.mockReset();
  onProbeResultMock.mockReset();
  // Default: probing is a no-op that never emits, so search tests are not
  // entangled with the probe effect.
  onProbeResultMock.mockResolvedValue(() => {});
  probeReleasesMock.mockResolvedValue([]);
});

function start(
  initial: ReleaseRequest | null = { titles: ["Show"], episode: 3 },
): Harness {
  current = harness(initial);
  return current;
}

describe("createReleaseSearch", () => {
  it("searches with the titles and episode from the request", async () => {
    searchReleasesMock.mockResolvedValue([release("Show - 03")]);
    const h = start();

    await settle();

    expect(searchReleasesMock).toHaveBeenCalledWith(["Show"], 3, undefined);
    expect(h.search.releases).toHaveLength(1);
    h.destroy();
  });

  it("does not search when there is no request", async () => {
    const h = start(null);
    await settle();

    expect(searchReleasesMock).not.toHaveBeenCalled();
    expect(h.search.releases).toEqual([]);
    h.destroy();
  });

  it("does not search when the titles are empty", async () => {
    const h = start({ titles: [] });
    await settle();

    expect(searchReleasesMock).not.toHaveBeenCalled();
    h.destroy();
  });

  it("reports a search failure as a message", async () => {
    searchReleasesMock.mockRejectedValue("indexer unreachable");
    const h = start();

    await settle();

    expect(h.search.error).toBe("indexer unreachable");
    expect(h.search.releases).toEqual([]);
    h.destroy();
  });

  it("re-searches when the request changes", async () => {
    searchReleasesMock.mockResolvedValue([]);
    const h = start();

    await settle();
    searchReleasesMock.mockClear();

    h.setRequest({ titles: ["Show"], episode: 4 });
    await settle();

    expect(searchReleasesMock).toHaveBeenCalledWith(["Show"], 4, undefined);
    h.destroy();
  });

  it("probes the releases a search returned", async () => {
    searchReleasesMock.mockResolvedValue([release("A"), release("B")]);
    const h = start();

    await settle();

    expect(probeReleasesMock).toHaveBeenCalledTimes(1);
    expect(probeReleasesMock.mock.calls[0][0]).toHaveLength(2);
    h.destroy();
  });

  it("records probe verdicts by index", async () => {
    searchReleasesMock.mockResolvedValue([release("A"), release("B")]);
    let emit: ((o: ProbeOutcome) => void) | null = null;
    onProbeResultMock.mockImplementation(async (handler) => {
      emit = handler;
      return () => {};
    });

    const h = start();
    await settle();

    emit!({
      index: 1,
      badge: "green",
      combinedScore: 50,
      probe: { infoHash: "b", totalDurationMs: 5 },
    });

    expect(h.search.badgeFor(1)?.badge).toBe("green");
    expect(h.search.badgeFor(0)).toBeUndefined();
    h.destroy();
  });

  it("ignores a verdict whose index is out of bounds", async () => {
    searchReleasesMock.mockResolvedValue([release("A")]);
    let emit: ((o: ProbeOutcome) => void) | null = null;
    onProbeResultMock.mockImplementation(async (handler) => {
      emit = handler;
      return () => {};
    });

    const h = start();
    await settle();

    emit!({
      index: 9,
      badge: "red",
      combinedScore: 1,
      probe: { infoHash: "z", totalDurationMs: 5 },
    });

    // Nothing thrown, and the single slot stays empty.
    expect(h.search.badgeFor(9)).toBeUndefined();
    h.destroy();
  });

  it("drops late verdicts after teardown", async () => {
    searchReleasesMock.mockResolvedValue([release("A")]);
    let emit: ((o: ProbeOutcome) => void) | null = null;
    onProbeResultMock.mockImplementation(async (handler) => {
      emit = handler;
      return () => {};
    });

    const h = start();
    await settle();
    h.destroy();

    // The listener is cancelled; calling it must not throw or mutate anything.
    expect(() =>
      emit!({
        index: 0,
        badge: "green",
        combinedScore: 5,
        probe: { infoHash: "a", totalDurationMs: 5 },
      }),
    ).not.toThrow();
  });
});