import { describe, it, expect } from "vitest";

import { healthLabel, streamWarning } from "./stream-health";
import type { TorrentProgress } from "./types";

function progress(overrides: Partial<TorrentProgress> = {}): TorrentProgress {
  return {
    state: "live",
    progressBytes: 1_000,
    totalBytes: 2_000,
    fileProgress: [500],
    finished: false,
    error: null,
    downloadMbps: 4,
    uploadMbps: 0,
    etaSeconds: 60,
    peersLive: 10,
    peersConnecting: 0,
    peersQueued: 0,
    peersSeen: 20,
    ...overrides,
  };
}

describe("streamWarning", () => {
  it("says nothing when the download is healthy", () => {
    expect(streamWarning(progress(), 0)).toBeNull();
  });

  it("says nothing before the first snapshot", () => {
    expect(streamWarning(null, 30)).toBeNull();
  });

  it("surfaces the backend error message", () => {
    const warning = streamWarning(
      progress({ state: "error", error: "tracker unreachable" }),
      0,
    );
    expect(warning?.level).toBe("error");
    expect(warning?.message).toContain("tracker unreachable");
  });

  it("reports a stall when bytes stop arriving", () => {
    const warning = streamWarning(progress(), 12);
    expect(warning?.level).toBe("error");
    expect(warning?.message).toMatch(/stalled/i);
  });

  it("does not call it stalled before the threshold", () => {
    expect(streamWarning(progress(), 5)).toBeNull();
  });

  it("reports a paused download as info, not a stall", () => {
    // No bytes arrive while paused, so the stall check would otherwise fire.
    const warning = streamWarning(progress({ state: "paused" }), 30);
    expect(warning?.level).toBe("info");
    expect(warning?.message).toMatch(/paused/i);
  });

  it("reports searching while peers are connecting", () => {
    const warning = streamWarning(
      progress({ peersLive: 0, peersConnecting: 2 }),
      0,
    );
    expect(warning?.level).toBe("info");
    expect(warning?.message).toMatch(/looking for peers/i);
  });

  it("warns when few peers and low speed", () => {
    const warning = streamWarning(
      progress({ peersLive: 2, downloadMbps: 0.2 }),
      0,
    );
    expect(warning?.level).toBe("warn");
    expect(warning?.message).toMatch(/slow download/i);
  });

  it("does not warn on few peers if the speed is fine", () => {
    expect(
      streamWarning(progress({ peersLive: 2, downloadMbps: 3 }), 0),
    ).toBeNull();
  });

  it("prefers the stall over the slow warning", () => {
    const warning = streamWarning(
      progress({ peersLive: 2, downloadMbps: 0.1 }),
      20,
    );
    expect(warning?.message).toMatch(/stalled/i);
  });

  it("reports a fully buffered episode as a green notice", () => {
    // A complete file is the one case that overrides a stall: once it is
    // buffered no more bytes arrive, so the stale counter would otherwise
    // eventually report it as stalled.
    const warning = streamWarning(progress(), 999, 1);
    expect(warning?.level).toBe("ok");
    expect(warning?.message).toMatch(/fully buffered/i);
  });

  it("shows the green notice even if the torrent state says error", () => {
    // The episode is playable regardless of what the torrent is doing now.
    const warning = streamWarning(
      progress({ state: "error", error: "tracker unreachable" }),
      0,
      1,
    );
    expect(warning?.level).toBe("ok");
  });

  it("does not report fully buffered below 100%", () => {
    const warning = streamWarning(progress(), 0, 0.99);
    expect(warning).toBeNull();
  });
});

describe("healthLabel", () => {
  it("is Waiting before any snapshot", () => {
    expect(healthLabel(null)).toBe("Waiting");
  });

  it("is Healthy with plenty of peers", () => {
    expect(healthLabel(progress({ peersLive: 10 }))).toBe("Healthy");
  });

  it("is Slow with few peers AND a slow download", () => {
    // Few peers alone is not "slow" -- a good connection still delivers.
    expect(healthLabel(progress({ peersLive: 2, downloadMbps: 0.2 }))).toBe(
      "Slow",
    );
  });

  it("is Healthy with few peers when the speed is good", () => {
    expect(healthLabel(progress({ peersLive: 2, downloadMbps: 4 }))).toBe(
      "Healthy",
    );
  });

  it("is Connecting while reaching out", () => {
    expect(healthLabel(progress({ peersLive: 0, peersConnecting: 3 }))).toBe(
      "Connecting",
    );
  });

  it("is Searching when nothing is known", () => {
    expect(
      healthLabel(progress({ peersLive: 0, peersConnecting: 0, peersQueued: 0 })),
    ).toBe("Searching");
  });

  it("is Paused while the download is paused", () => {
    expect(healthLabel(progress({ state: "paused" }))).toBe("Paused");
  });
});