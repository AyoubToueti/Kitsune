import { describe, it, expect } from "vitest";

import { compactResult, speedSummary, verdictClass } from "./speed-test";
import type { SpeedTestResult } from "./types";

function result(overrides: Partial<SpeedTestResult> = {}): SpeedTestResult {
  return {
    latencyMs: 18,
    jitterMs: 2,
    downloadMbps: 42,
    bytesDownloaded: 10_000_000,
    durationMs: 8000,
    verdict: "good",
    ...overrides,
  };
}

describe("speedSummary", () => {
  it("reads as good for a fast connection", () => {
    const summary = speedSummary(result({ verdict: "good", downloadMbps: 42 }));
    expect(summary.level).toBe("good");
    expect(summary.message).toContain("42 Mbps");
    expect(summary.message.toLowerCase()).toContain("stream");
  });

  it("reads as ok for a middling connection", () => {
    const summary = speedSummary(result({ verdict: "ok", downloadMbps: 12 }));
    expect(summary.level).toBe("ok");
    expect(summary.message).toContain("12 Mbps");
  });

  it("warns for a slow connection", () => {
    const summary = speedSummary(result({ verdict: "poor", downloadMbps: 3 }));
    expect(summary.level).toBe("poor");
    expect(summary.message.toLowerCase()).toContain("stutter");
  });
});

describe("compactResult", () => {
  it("formats a short one-line result", () => {
    expect(compactResult(result({ downloadMbps: 42, latencyMs: 18 }))).toBe(
      "↓ 42 Mbps · 18 ms",
    );
  });

  it("rounds the numbers", () => {
    expect(compactResult(result({ downloadMbps: 41.6, latencyMs: 18.4 }))).toBe(
      "↓ 42 Mbps · 18 ms",
    );
  });
});

describe("verdictClass", () => {
  it("maps each verdict to a colour class", () => {
    expect(verdictClass("good")).toBe("text-health-green");
    expect(verdictClass("ok")).toBe("text-health-yellow");
    expect(verdictClass("poor")).toBe("text-danger");
  });
});