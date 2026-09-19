import { describe, it, expect, vi, beforeEach } from "vitest";

// Both Tauri modules must be mocked before the wrapper is imported, so the
// factories are hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const listenMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

import {
  PROBE_RESULT_EVENT,
  RELEASE_COMMANDS,
  downloadTorrent,
  onProbeResult,
  probeReleases,
  searchReleases,
} from "./releases";
import type { ProbeOutcome, Release } from "$lib/types";

function release(overrides: Partial<Release> = {}): Release {
  return {
    title: "[Group] Show - 01 [1080p]",
    indexer: "nyaa",
    magnetUri: "magnet:?xt=urn:btih:abc",
    infoHash: "abc",
    resolution: "1080p",
    source: "webdl",
    remux: false,
    trusted: false,
    parsed: { title: "Show", absoluteEpisode: 1 },
    score: 0,
    ...overrides,
  };
}

function outcome(index: number): ProbeOutcome {
  return {
    index,
    badge: "green",
    combinedScore: 1000,
    probe: {
      infoHash: "abc",
      metadata: { resolved: true, durationMs: 5 },
      totalDurationMs: 5,
    },
  };
}

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
});

describe("release command wrappers", () => {
  it("searchReleases passes every title form and the episode", async () => {
    invokeMock.mockResolvedValue([release()]);

    const found = await searchReleases(["Show", "Shou"], 5);

    expect(invokeMock).toHaveBeenCalledWith(RELEASE_COMMANDS.search, {
      titles: ["Show", "Shou"],
      episode: 5,
        absoluteEpisode: undefined,
      });
      expect(found).toHaveLength(1);
    });
  
    it("searchReleases passes the absolute episode when the numbers differ", async () => {
      invokeMock.mockResolvedValue([release()]);
  
      await searchReleases(["Show Season 2"], 1, 13);
  
      expect(invokeMock).toHaveBeenCalledWith(RELEASE_COMMANDS.search, {
        titles: ["Show Season 2"],
        episode: 1,
        absoluteEpisode: 13,
      });
    await downloadTorrent("https://nyaa.si/download/1.torrent", "/tmp/1.torrent");

    expect(invokeMock).toHaveBeenCalledWith(RELEASE_COMMANDS.download, {
      url: "https://nyaa.si/download/1.torrent",
      path: "/tmp/1.torrent",
    });
  });

  it("probeReleases sends the releases and returns the outcomes", async () => {
    invokeMock.mockResolvedValue([outcome(0)]);

    const releases = [release()];
    const results = await probeReleases(releases);

    expect(invokeMock).toHaveBeenCalledWith(RELEASE_COMMANDS.probe, { releases });
    expect(results[0].badge).toBe("green");
  });

  it("onProbeResult subscribes to the probe event", async () => {
    listenMock.mockResolvedValue(() => {});

    await onProbeResult(() => {});

    expect(listenMock).toHaveBeenCalledWith(
      PROBE_RESULT_EVENT,
      expect.any(Function),
    );
  });

  it("onProbeResult forwards a well-formed payload to the handler", async () => {
    listenMock.mockResolvedValue(() => {});
    const handler = vi.fn();

    await onProbeResult(handler);

    // Replay the listener the wrapper registered, as Tauri would on an event.
    const listener = listenMock.mock.calls[0][1] as (event: {
      payload: ProbeOutcome;
    }) => void;
    listener({ payload: outcome(2) });

    expect(handler).toHaveBeenCalledWith(outcome(2));
  });

  it("onProbeResult drops a payload with no index", async () => {
    // The danger is not a missing event but a malformed one: applying it to
    // release 0 by accident would put a stranger's verdict on the wrong row.
    listenMock.mockResolvedValue(() => {});
    const handler = vi.fn();

    await onProbeResult(handler);

    const listener = listenMock.mock.calls[0][1] as (event: {
      payload: unknown;
    }) => void;
    listener({ payload: { badge: "green" } });
    listener({ payload: null });
    listener({ payload: { index: "0" } });

    expect(handler).not.toHaveBeenCalled();
  });
});