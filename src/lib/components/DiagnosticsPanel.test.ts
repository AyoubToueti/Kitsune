import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";

import type { SystemStats } from "$lib/types";

const getSystemStatsMock = vi.hoisted(() => vi.fn());
const getLogPathMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/diagnostics", () => ({
  getSystemStats: getSystemStatsMock,
  getLogPath: getLogPathMock,
}));

const revealItemInDirMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: revealItemInDirMock,
}));

import DiagnosticsPanel from "./DiagnosticsPanel.svelte";

function stats(overrides: Partial<SystemStats> = {}): SystemStats {
  return {
    cpuPercent: 12.5,
    rssBytes: 150_000_000,
    virtualBytes: 300_000_000,
    threadCount: 8,
    uptimeSeconds: 125,
    systemTotalBytes: 16_000_000_000,
    systemAvailableBytes: 8_000_000_000,
    ...overrides,
  };
}

beforeEach(() => {
  getSystemStatsMock.mockReset().mockResolvedValue(stats());
  getLogPathMock.mockReset().mockResolvedValue("/data/kitsune/logs/kitsune.log");
  revealItemInDirMock.mockReset().mockResolvedValue(undefined);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("DiagnosticsPanel", () => {
  it("renders the CPU, memory, threads and uptime", async () => {
    render(DiagnosticsPanel);

    expect(await screen.findByTestId("diag-cpu")).toHaveTextContent("12.5%");
    expect(screen.getByTestId("diag-memory")).toHaveTextContent("143 MB");
    expect(screen.getByTestId("diag-threads")).toHaveTextContent("8");
    // 125 seconds is "2m 5s".
    expect(screen.getByTestId("diag-uptime")).toHaveTextContent("2m 5s");
  });

  it("formats an uptime over an hour as hours and minutes", async () => {
    getSystemStatsMock.mockResolvedValue(stats({ uptimeSeconds: 3720 }));
    render(DiagnosticsPanel);

    // 3720s = 1h 2m.
    expect(await screen.findByTestId("diag-uptime")).toHaveTextContent("1h 2m");
  });

  it("reveals the log file's folder when asked", async () => {
    render(DiagnosticsPanel);
    await screen.findByTestId("diag-cpu");

    await fireEvent.click(screen.getByRole("button", { name: /reveal log/i }));

    expect(getLogPathMock).toHaveBeenCalled();
    await waitFor(() =>
      expect(revealItemInDirMock).toHaveBeenCalledWith(
        "/data/kitsune/logs/kitsune.log",
      ),
    );
  });

  it("shows an error when the snapshot read fails", async () => {
    getSystemStatsMock.mockRejectedValue(new Error("no such process"));
    render(DiagnosticsPanel);

    expect(await screen.findByText(/no such process/i)).toBeInTheDocument();
  });

  it("polls again after the interval", async () => {
    vi.useFakeTimers();
    render(DiagnosticsPanel);

    // The immediate read on mount.
    await vi.waitFor(() => expect(getSystemStatsMock).toHaveBeenCalledTimes(1));

    await vi.advanceTimersByTimeAsync(2000);
    expect(getSystemStatsMock).toHaveBeenCalledTimes(2);
  });
});