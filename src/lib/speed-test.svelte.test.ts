import { describe, it, expect, vi, beforeEach } from "vitest";

// The API module is mocked before import so the composable runs without Tauri.
const runSpeedTestMock = vi.hoisted(() => vi.fn());
const onSpeedTestProgressMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/diagnostics", () => ({
  runSpeedTest: runSpeedTestMock,
  onSpeedTestProgress: onSpeedTestProgressMock,
}));

import { createSpeedTest } from "./speed-test.svelte";
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

beforeEach(() => {
  runSpeedTestMock.mockReset();
  onSpeedTestProgressMock.mockReset();
  // A listener that never fires is enough for the run tests.
  onSpeedTestProgressMock.mockResolvedValue(() => {});
});

describe("createSpeedTest", () => {
  it("runs a test and exposes the result", async () => {
    runSpeedTestMock.mockResolvedValue(result());
    const test = createSpeedTest();

    test.run();
    expect(test.running).toBe(true);

    // Let the promise settle.
    await Promise.resolve();
    await Promise.resolve();

    expect(test.running).toBe(false);
    expect(test.result?.downloadMbps).toBe(42);
    expect(test.error).toBeNull();
  });

  it("captures a failure as an error rather than throwing", async () => {
    runSpeedTestMock.mockRejectedValue(new Error("a speed test is already running"));
    const test = createSpeedTest();

    test.run();
    await Promise.resolve();
    await Promise.resolve();

    expect(test.running).toBe(false);
    expect(test.result).toBeNull();
    expect(test.error).toContain("already running");
  });

  it("ignores a second run while one is in flight", async () => {
    let release: (value: SpeedTestResult) => void = () => {};
    runSpeedTestMock.mockImplementation(
      () => new Promise<SpeedTestResult>((resolve) => (release = resolve)),
    );
    const test = createSpeedTest();

    test.run();
    test.run();
    test.run();

    expect(runSpeedTestMock).toHaveBeenCalledTimes(1);

    release(result());
    await Promise.resolve();
    await Promise.resolve();
  });

  it("clears the previous result when a new run starts", async () => {
    runSpeedTestMock.mockResolvedValueOnce(result({ downloadMbps: 10 }));
    const test = createSpeedTest();

    test.run();
    await Promise.resolve();
    await Promise.resolve();
    expect(test.result?.downloadMbps).toBe(10);

    // A second run clears the result before the new one lands.
    let release: (value: SpeedTestResult) => void = () => {};
    runSpeedTestMock.mockImplementationOnce(
      () => new Promise<SpeedTestResult>((resolve) => (release = resolve)),
    );
    test.run();
    expect(test.result).toBeNull();

    release(result({ downloadMbps: 99 }));
    await Promise.resolve();
    await Promise.resolve();
    expect(test.result?.downloadMbps).toBe(99);
  });

  it("start installs a progress listener and returns a teardown", async () => {
    const unlisten = vi.fn();
    onSpeedTestProgressMock.mockResolvedValue(unlisten);
    const test = createSpeedTest();

    const teardown = test.start();
    await Promise.resolve();

    expect(onSpeedTestProgressMock).toHaveBeenCalledTimes(1);
    teardown();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});