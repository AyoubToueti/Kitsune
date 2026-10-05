// Typed wrappers over the settings commands in `src-tauri/src/settings/mod.rs`.
//
// Separate from `./player` and `./auth`: this is the reader's own configuration,
// stored on their machine, rather than anything about a work or an account. The
// backend owns the file; the frontend only reads and writes the whole object,
// so there is no partial-update surface to keep in step.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { Settings } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const SETTINGS_COMMANDS = {
  get: "get_settings",
  set: "set_settings",
  setDefaultPlayer: "set_default_player",
} as const;

/**
 * Event the backend emits after the settings are written.
 *
 * Lets any surface showing a setting -- the watch page's player picker, say --
 * update without a reload. Kept in sync with `SETTINGS_CHANGED_EVENT` in
 * `src-tauri/src/settings/mod.rs`.
 */
export const SETTINGS_CHANGED_EVENT = "settings-changed";

/**
 * The reader's stored settings.
 *
 * Never fails: the backend answers the built-in defaults when no file exists or
 * the file is unreadable, so a caller does not have to handle "no settings".
 */
export async function getSettings(): Promise<Settings> {
  return invoke<Settings>(SETTINGS_COMMANDS.get);
}

/**
 * Replace the stored settings.
 *
 * Resolves to what was actually persisted, so a caller can pick up any value
 * the backend clamped (the auto-launch threshold) without a second read.
 */
export async function setSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>(SETTINGS_COMMANDS.set, { settings });
}

/**
 * Remember `program` as the default player and stop asking which to use.
 *
 * The picker's "Always" action. A focused command rather than a whole-object
 * `setSettings` call, so the picker does not have to fetch and round-trip every
 * other preference to change one. Resolves to the stored settings, and the
 * backend emits `SETTINGS_CHANGED_EVENT` as for any other write.
 */
export async function setDefaultPlayer(
  program: string,
  extraArgs: string[] = [],
): Promise<Settings> {
  return invoke<Settings>(SETTINGS_COMMANDS.setDefaultPlayer, {
    program,
    extraArgs,
  });
}

/**
 * Subscribe to settings changes, calling back with the new settings.
 *
 * `listen` resolves with an unlisten function rather than returning one, so the
 * caller cannot forget to await it and leak the listener. A malformed payload
 * is dropped rather than thrown into Tauri's event loop.
 */
export async function onSettingsChanged(
  handler: (settings: Settings) => void,
): Promise<UnlistenFn> {
  return listen<Settings>(SETTINGS_CHANGED_EVENT, (event) => {
    const settings = event.payload;
    if (settings == null || typeof settings !== "object") return;
    handler(settings);
  });
}