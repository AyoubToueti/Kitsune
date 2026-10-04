// Typed wrappers over the diagnostics commands in
// `src-tauri/src/diagnostics/commands.rs`.
//
// Separate from the other API modules: this is about the app process itself
// rather than a work, an account or the reader's preferences, and none of it is
// cached -- a resource snapshot is only meaningful for the moment it was taken.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { SpeedTestProgress, SpeedTestResult, SystemStats } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const DIAGNOSTICS_COMMANDS = {
  systemStats: "get_system_stats",
  logPath: "get_log_path",
  logError: "log_frontend_error",
  speedTest: "run_speed_test",
} as const;

/**
 * Event the backend emits with each speed-test progress update.
 *
 * Kept in sync with `speedtest::PROGRESS_EVENT` in the Rust backend.
 */
export const SPEEDTEST_PROGRESS_EVENT = "speedtest-progress";

/**
 * A fresh snapshot of the app process's CPU and memory.
 *
 * Never fails: the backend answers a zeroed snapshot when the process cannot
 * be found, so the panel has nothing to special-case.
 */
export async function getSystemStats(): Promise<SystemStats> {
  return invoke<SystemStats>(DIAGNOSTICS_COMMANDS.systemStats);
}

/** Absolute path of the active log file, for a "reveal in file manager" button. */
export async function getLogPath(): Promise<string> {
  return invoke<string>(DIAGNOSTICS_COMMANDS.logPath);
}

/**
 * Forward an error the webview caught into the backend log.
 *
 * `stack` is sent as `null` when absent, matching the Rust `Option<String>`.
 * The caller does not await a result: this runs on the error path, where a
 * failure to log must never raise a second error.
 */
export async function logFrontendError(
  level: "warn" | "error",
  message: string,
  stack?: string,
): Promise<void> {
  return invoke<void>(DIAGNOSTICS_COMMANDS.logError, {
    level,
    message,
    stack: stack ?? null,
  });
}

/**
 * Measure latency and download speed against Cloudflare.
 *
 * Resolves with the final result once the whole test finishes (a few seconds).
 * Rejects with a message when the endpoint is unreachable or another test is
 * already running. Progress is not returned here -- subscribe with
 * [`onSpeedTestProgress`] to show a live readout.
 */
export async function runSpeedTest(): Promise<SpeedTestResult> {
  return invoke<SpeedTestResult>(DIAGNOSTICS_COMMANDS.speedTest);
}

/**
 * Subscribe to speed-test progress, calling back with each update.
 *
 * `listen` resolves with an unlisten function, so the caller cannot forget to
 * await it and leak the listener. A malformed payload is dropped rather than
 * thrown into Tauri's event loop.
 */
export async function onSpeedTestProgress(
  handler: (progress: SpeedTestProgress) => void,
): Promise<UnlistenFn> {
  return listen<SpeedTestProgress>(SPEEDTEST_PROGRESS_EVENT, (event) => {
    const progress = event.payload;
    if (progress == null || typeof progress !== "object") return;
    handler(progress);
  });
}