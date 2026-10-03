import { describe, it, expect, vi, beforeEach } from "vitest";

// The wrapper module must be mocked before it is imported, so the factory is
// hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import {
  DIAGNOSTICS_COMMANDS,
  getLogPath,
  getSystemStats,
  logFrontendError,
} from "./diagnostics";
import type { SystemStats } from "$lib/types";

function stats(): SystemStats {
  return {
    cpuPercent: 12.5,
    rssBytes: 100_000_000,
    virtualBytes: 200_000_000,
    threadCount: 8,
    uptimeSeconds: 42,
    systemTotalBytes: 16_000_000_000,
    systemAvailableBytes: 8_000_000_000,
  };
}

beforeEach(() => {
  invokeMock.mockReset();
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
});