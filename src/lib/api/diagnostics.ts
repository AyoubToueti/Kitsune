// Typed wrappers over the diagnostics commands in
// `src-tauri/src/diagnostics/commands.rs`.
//
// Separate from the other API modules: this is about the app process itself
// rather than a work, an account or the reader's preferences, and none of it is
// cached -- a resource snapshot is only meaningful for the moment it was taken.

import { invoke } from "@tauri-apps/api/core";

import type { SystemStats } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const DIAGNOSTICS_COMMANDS = {
  systemStats: "get_system_stats",
  logPath: "get_log_path",
  logError: "log_frontend_error",
} as const;

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