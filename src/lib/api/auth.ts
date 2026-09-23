// Typed wrappers over the auth commands in `src-tauri/src/auth/commands.rs`.
//
// Separate from `./anime` for the same reason `./player` is: these answer a
// question about the reader's account rather than about a work. Mixing them
// would put a credential's lifecycle behind the metadata cache, which is the
// last place it belongs.
//
// The token itself never crosses this boundary. The backend stores it and
// attaches it to requests; the frontend only ever learns WHETHER someone is
// signed in, which is all any UI needs.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { ListEntry, ListStatus } from "$lib/types";

/** Command names, centralised so a rename cannot drift. */
export const AUTH_COMMANDS = {
  beginLogin: "begin_login",
  status: "auth_status",
  logout: "logout",
  getListEntry: "get_list_entry",
  setListEntry: "set_list_entry",
} as const;

/**
 * Event the backend emits after the token changes.
 *
 * Fired when a redirect applies a token and when the reader signs out, so a
 * component can react without polling. The payload is a boolean, never the
 * token.
 */
export const AUTH_CHANGED_EVENT = "auth-changed";

/**
 * The URL to open so the reader can authorise Kitsune.
 *
 * Returned rather than opened by the backend because launching a browser is a
 * UI action: the frontend already holds the opener plugin, and keeping it there
 * means the backend never needs to know how a URL gets shown.
 */
export async function beginLogin(): Promise<string> {
  return invoke<string>(AUTH_COMMANDS.beginLogin);
}

/**
 * Whether a token is stored.
 *
 * Deliberately not cached: a sign-out in another window, or a redirect that
 * lands while this component is mounted, would leave a cached `true` claiming
 * the reader is signed in when they are not.
 */
export async function authStatus(): Promise<boolean> {
  return invoke<boolean>(AUTH_COMMANDS.status);
}

/** Forget the stored token. */
export async function logout(): Promise<void> {
  return invoke<void>(AUTH_COMMANDS.logout);
}

/**
 * Subscribe to sign-in and sign-out.
 *
 * `listen` resolves with an unlisten function rather than returning one, so the
 * caller cannot forget to await it and leak the listener. The callback receives
 * only the signed-in flag; a malformed payload is ignored rather than thrown
 * into Tauri's event loop.
 */
export async function onAuthChanged(
  handler: (signedIn: boolean) => void,
): Promise<UnlistenFn> {
  return listen<boolean>(AUTH_CHANGED_EVENT, (event) => {
    if (typeof event.payload !== "boolean") return;
    handler(event.payload);
  });
}

/**
 * Where a work sits on the reader's list, or `null` when it is not on it.
 *
 * Not cached. A status can change from this app at any moment, and a cached
 * entry would leave the menu showing a stale selection -- which reads as the
 * write having silently failed.
 */
export async function getListEntry(
  mediaId: number,
): Promise<ListEntry | null> {
  return invoke<ListEntry | null>(AUTH_COMMANDS.getListEntry, { mediaId });
}

/**
 * Put a work on the reader's list, or move it between lists.
 *
 * `progress` is omitted rather than sent as zero when the caller only wants to
 * change the status: the backend passes `null` on, and AniList leaves a stored
 * progress value alone when the field is absent. Sending zero would silently
 * reset how far the reader had got.
 */
export async function setListEntry(
  mediaId: number,
  status: ListStatus,
  progress?: number,
): Promise<void> {
  return invoke<void>(AUTH_COMMANDS.setListEntry, {
    mediaId,
    status,
    progress,
  });
}