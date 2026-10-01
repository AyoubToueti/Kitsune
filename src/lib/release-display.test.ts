import { describe, it, expect } from "vitest";

import {
  badgeClass,
  badgeTitle,
  clarityClass,
  formatSize,
} from "./release-display";
import type { ProbeOutcome } from "./types";

function outcome(overrides: Partial<ProbeOutcome> = {}): ProbeOutcome {
  return {
    index: 0,
    badge: "green",
    combinedScore: 100,
    probe: {
      infoHash: "abc",
      totalDurationMs: 5,
    },
    ...overrides,
  };
}

describe("badgeClass", () => {
  it("maps each verdict to its colour", () => {
    expect(badgeClass("green")).toBe("bg-health-green");
    expect(badgeClass("yellow")).toBe("bg-health-yellow");
    expect(badgeClass("red")).toBe("bg-health-red");
  });

  it("pulses while unprobed", () => {
    expect(badgeClass(undefined)).toContain("animate-pulse");
  });
});

describe("clarityClass", () => {
  it("rings a stated row", () => {
    expect(clarityClass("stated")).toContain("border-accent");
  });

  it("dims an unclear row", () => {
    expect(clarityClass("unclear")).toBe("opacity-60");
  });

  it("leaves the ordinary case unmarked", () => {
    expect(clarityClass("episode")).toBe("");
  });
});

describe("badgeTitle", () => {
  it("describes an unprobed release", () => {
    expect(badgeTitle(undefined)).toBe("Checking swarm health…");
  });

  it("reports the tracker seeders and confirmed metadata", () => {
    const title = badgeTitle(
      outcome({
        probe: {
          infoHash: "abc",
          totalDurationMs: 5,
          scrape: {
            seeders: 42,
            leechers: 1,
            completed: 10,
            trackerUrl: "udp://x",
            durationMs: 3,
          },
          metadata: { resolved: true, durationMs: 4 },
        },
      }),
    );
    expect(title).toContain("42 seeders");
    expect(title).toContain("metadata confirmed");
  });

  it("says when metadata was not found", () => {
    const title = badgeTitle(
      outcome({
        probe: {
          infoHash: "abc",
          totalDurationMs: 5,
          metadata: { resolved: false, durationMs: 4 },
        },
      }),
    );
    expect(title).toContain("metadata not found");
  });
});

describe("formatSize", () => {
  it("formats gigabytes with one decimal", () => {
    expect(formatSize(1_500_000_000)).toBe("1.4 GB");
  });

  it("formats bytes without a decimal", () => {
    expect(formatSize(512)).toBe("512 B");
  });

  it("returns undefined for zero or absent", () => {
    expect(formatSize(0)).toBeUndefined();
    expect(formatSize(undefined)).toBeUndefined();
  });
});