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
  getListEntry,
  logout,
  onAuthChanged,
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