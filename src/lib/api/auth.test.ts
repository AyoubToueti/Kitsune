import { describe, it, expect, vi, beforeEach } from "vitest";

// Both Tauri modules must be mocked before the wrapper is imported, so the
// factories are hoisted by Vitest.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const listenMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

import {
  AUTH_CHANGED_EVENT,
  AUTH_COMMANDS,
  authStatus,
  beginLogin,
  deleteListEntry,
  getContinueWatching,
  getLastPlayed,
  getListEntry,
  getUserList,
  LIST_CHANGED_EVENT,
  logout,
  onAuthChanged,
  onListChanged,
  recordLastPlayed,
  setListEntry,
} from "./auth";

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
});

describe("auth command wrappers", () => {
  it("beginLogin returns the authorize url", async () => {
    invokeMock.mockResolvedValue("https://anilist.co/api/v2/oauth/authorize?x=1");

    const url = await beginLogin();

    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.beginLogin);
    expect(url).toContain("anilist.co");
  });

  it("authStatus returns the stored flag", async () => {
    invokeMock.mockResolvedValue(true);

    await expect(authStatus()).resolves.toBe(true);
    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.status);
  });

  /// Deliberately not cached: a sign-out elsewhere would otherwise leave a
  /// cached `true` claiming the reader is signed in.
  it("authStatus asks the backend every time", async () => {
    invokeMock.mockResolvedValue(true);

    await authStatus();
    await authStatus();

    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("logout calls the command", async () => {
    invokeMock.mockResolvedValue(undefined);

    await logout();

    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.logout);
  });
});

describe("list entry wrappers", () => {
  it("getListEntry passes the media id", async () => {
    invokeMock.mockResolvedValue({ status: "current", progress: 3 });

    const entry = await getListEntry(21);

    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.getListEntry, {
      mediaId: 21,
    });
    expect(entry?.status).toBe("current");
  });

  /// A work that is not on the list is a legitimate answer, not an error.
  it("getListEntry returns null for a work off the list", async () => {
    invokeMock.mockResolvedValue(null);

    await expect(getListEntry(21)).resolves.toBeNull();
  });

  it("setListEntry passes the status", async () => {
    invokeMock.mockResolvedValue(undefined);

    await setListEntry(21, "paused");

    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.setListEntry, {
      mediaId: 21,
      status: "paused",
      progress: undefined,
    });
  });

  /// Progress is omitted rather than sent as zero. The backend passes null on,
  /// and AniList leaves a stored value alone when the field is absent -- so
  /// sending zero would reset how far the reader had got.
  it("setListEntry omits progress when it is not given", async () => {
    invokeMock.mockResolvedValue(undefined);

    await setListEntry(21, "current");

    const [, payload] = invokeMock.mock.calls[0];
    expect(payload.progress).toBeUndefined();
  });

  it("setListEntry forwards progress when it is given", async () => {
    invokeMock.mockResolvedValue(undefined);

    await setListEntry(21, "current", 7);

    const [, payload] = invokeMock.mock.calls[0];
    expect(payload.progress).toBe(7);
  });

  /// Deliberately not cached: a status can change at any moment, and a stale
  /// entry would leave the menu showing a selection that is no longer true.
  it("getListEntry asks the backend every time", async () => {
    invokeMock.mockResolvedValue(null);

    await getListEntry(21);
    await getListEntry(21);

    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("getContinueWatching forwards the limit", async () => {
    invokeMock.mockResolvedValue([]);

    await getContinueWatching(6);

    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.continueWatching, {
      limit: 6,
    });
  });

  /// The home page calls this with no argument, so the default has to be a
  /// real number rather than `undefined`, which the command cannot decode.
  it("getContinueWatching defaults the limit", async () => {
    invokeMock.mockResolvedValue([]);

    await getContinueWatching();

    const [, payload] = invokeMock.mock.calls[0];
    expect(typeof payload.limit).toBe("number");
  });

  it("getUserList returns the entries", async () => {
    const entries = [{ anime: { id: 1 }, status: "current", progress: 3, entryId: 9 }];
    invokeMock.mockResolvedValue(entries);

    await expect(getUserList()).resolves.toBe(entries);
    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.userList);
  });

  /// Removing must send the LIST ENTRY id, not the media id: the backend keys
  /// the delete on the entry, and a media id would target the wrong thing.
  it("deleteListEntry forwards the entry id", async () => {
    invokeMock.mockResolvedValue(undefined);

    await deleteListEntry(9);

    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.deleteListEntry, {
      entryId: 9,
    });
  });

  it("recordLastPlayed forwards the work and episode", async () => {
    invokeMock.mockResolvedValue(undefined);

    await recordLastPlayed(21, 3);

    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.recordLastPlayed, {
      animeId: 21,
      episode: 3,
    });
  });

  it("recordLastPlayed allows a missing episode", async () => {
    invokeMock.mockResolvedValue(undefined);

    await recordLastPlayed(21);

    const [, payload] = invokeMock.mock.calls[0];
    expect(payload.episode).toBeUndefined();
  });

  it("getLastPlayed returns the record", async () => {
    const record = { animeId: 21, episode: 3, at: 1_700_000_000 };
    invokeMock.mockResolvedValue(record);

    await expect(getLastPlayed()).resolves.toBe(record);
    expect(invokeMock).toHaveBeenCalledWith(AUTH_COMMANDS.lastPlayed);
  });
});

describe("onAuthChanged", () => {
  it("subscribes to the auth event", async () => {
    listenMock.mockResolvedValue(() => {});

    await onAuthChanged(() => {});

    expect(listenMock).toHaveBeenCalledWith(
      AUTH_CHANGED_EVENT,
      expect.any(Function),
    );
  });

  it("forwards the signed-in flag", async () => {
    let fire: ((event: { payload: unknown }) => void) | undefined;
    listenMock.mockImplementation((_event: string, cb: typeof fire) => {
      fire = cb;
      return Promise.resolve(() => {});
    });
    const handler = vi.fn();

    await onAuthChanged(handler);
    fire!({ payload: true });

    expect(handler).toHaveBeenCalledWith(true);
  });

  /// The payload is a boolean, so anything else is dropped rather than passed
  /// on as a truthy value the UI would read as "signed in".
  it("ignores a malformed payload", async () => {
    let fire: ((event: { payload: unknown }) => void) | undefined;
    listenMock.mockImplementation((_event: string, cb: typeof fire) => {
      fire = cb;
      return Promise.resolve(() => {});
    });
    const handler = vi.fn();

    await onAuthChanged(handler);
    fire!({ payload: "yes" });
    fire!({ payload: null });

    expect(handler).not.toHaveBeenCalled();
  });

  it("returns the unlisten function", async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);

    const returned = await onAuthChanged(() => {});

    expect(returned).toBe(unlisten);
  });
});

describe("onListChanged", () => {
  it("subscribes to the list event", async () => {
    listenMock.mockResolvedValue(() => {});

    await onListChanged(() => {});

    expect(listenMock).toHaveBeenCalledWith(
      LIST_CHANGED_EVENT,
      expect.any(Function),
    );
  });

  /// The event carries no payload; it only says "re-read the list".
  it("calls the handler when the event fires", async () => {
    let fire: (() => void) | undefined;
    listenMock.mockImplementation((_event: string, cb: () => void) => {
      fire = cb;
      return Promise.resolve(() => {});
    });
    const handler = vi.fn();

    await onListChanged(handler);
    fire!();

    expect(handler).toHaveBeenCalled();
  });

  it("returns the unlisten function", async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);

    const returned = await onListChanged(() => {});

    expect(returned).toBe(unlisten);
  });
});