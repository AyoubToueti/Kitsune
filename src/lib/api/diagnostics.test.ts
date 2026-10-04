import { describe, it, expect, vi, beforeEach } from "vitest";

// The wrapper module must be mocked before it is imported, so the factory is
// hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const listenMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

import {
  DIAGNOSTICS_COMMANDS,
  SPEEDTEST_PROGRESS_EVENT,
  getLogPath,
  getSystemStats,
  logFrontendError,
  onSpeedTestProgress,
  runSpeedTest,
} from "./diagnostics";
import type { SpeedTestResult, SystemStats } from "$lib/types";

function stats(): SystemStats {
  return {
    cpuPercent: 12.5,
    cpuPercentOfMachine: 3.1,
    cpuCores: 4,
    rssBytes: 100_000_000,
    virtualBytes: 200_000_000,
    threadCount: 8,
    processCount: 6,
    uptimeSeconds: 42,
    systemTotalBytes: 16_000_000_000,
    systemAvailableBytes: 8_000_000_000,
  };
}

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
});

describe("diagnostics command wrappers", () => {
  it("getSystemStats returns the snapshot", async () => {
    invokeMock.mockResolvedValue(stats());

    const result = await getSystemStats();

    expect(invokeMock).toHaveBeenCalledWith(DIAGNOSTICS_COMMANDS.systemStats);
    expect(result.cpuPercent).toBe(12.5);
    expect(result.threadCount).toBe(8);
  });

  it("getLogPath returns the path", async () => {
    invokeMock.mockResolvedValue(
      "/home/me/.local/share/kitsune/logs/kitsune.log",
    );

    const path = await getLogPath();

    expect(invokeMock).toHaveBeenCalledWith(DIAGNOSTICS_COMMANDS.logPath);
    expect(path).toContain("kitsune.log");
  });

  it("logFrontendError sends a null stack when none is given", async () => {
    invokeMock.mockResolvedValue(undefined);

    await logFrontendError("error", "boom");

    expect(invokeMock).toHaveBeenCalledWith(DIAGNOSTICS_COMMANDS.logError, {
      level: "error",
      message: "boom",
      stack: null,
    });
  });

  it("logFrontendError passes a stack through when given", async () => {
    invokeMock.mockResolvedValue(undefined);

    await logFrontendError("warn", "careful", "at foo (app.js:1)");

    expect(invokeMock).toHaveBeenCalledWith(DIAGNOSTICS_COMMANDS.logError, {
      level: "warn",
      message: "careful",
      stack: "at foo (app.js:1)",
    });
  });

  it("runSpeedTest returns the result", async () => {
    const result: SpeedTestResult = {
      latencyMs: 18,
      jitterMs: 2,
      downloadMbps: 42,
      bytesDownloaded: 10_000_000,
      durationMs: 8000,
      verdict: "good",
    };
    invokeMock.mockResolvedValue(result);

    const measured = await runSpeedTest();

    expect(invokeMock).toHaveBeenCalledWith(DIAGNOSTICS_COMMANDS.speedTest);
    expect(measured.downloadMbps).toBe(42);
  });

  it("onSpeedTestProgress forwards a well-formed payload", async () => {
    const unlisten = vi.fn();
    listenMock.mockImplementation(
      async (_event: string, handler: (e: { payload: unknown }) => void) => {
        handler({ payload: { phase: "download", percent: 60 } });
        return unlisten;
      },
    );

    const seen: number[] = [];
    await onSpeedTestProgress((p) => seen.push(p.percent));

    expect(listenMock).toHaveBeenCalledWith(
      SPEEDTEST_PROGRESS_EVENT,
      expect.any(Function),
    );
    expect(seen).toEqual([60]);
  });

  it("onSpeedTestProgress drops a malformed payload", async () => {
    listenMock.mockImplementation(
      async (_event: string, handler: (e: { payload: unknown }) => void) => {
        handler({ payload: null });
        handler({ payload: "nonsense" });
        return vi.fn();
      },
    );

    const seen: unknown[] = [];
    await onSpeedTestProgress((p) => seen.push(p));

    expect(seen).toEqual([]);
  });
});