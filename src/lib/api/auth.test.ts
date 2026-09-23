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
  logout,
  onAuthChanged,
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