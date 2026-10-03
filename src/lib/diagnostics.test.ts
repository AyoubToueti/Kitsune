import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the API module so the pure helpers can be tested without Tauri, and so
// `initErrorCapture`'s forwarding can be asserted.
const logFrontendErrorMock = vi.hoisted(() => vi.fn());
vi.mock("$lib/api/diagnostics", () => ({
  logFrontendError: logFrontendErrorMock,
}));

import {
  formatErrorEvent,
  formatRejection,
  initErrorCapture,
} from "./diagnostics";

beforeEach(() => {
  logFrontendErrorMock.mockReset();
  logFrontendErrorMock.mockResolvedValue(undefined);
});

describe("formatErrorEvent", () => {
  it("uses the message and folds in the source location", () => {
    const captured = formatErrorEvent("boom", "app.js", 10, 5, undefined);
    expect(captured.level).toBe("error");
    expect(captured.message).toBe("boom (app.js:10:5)");
  });

  it("falls back to the error's own message and keeps its stack", () => {
    const err = new Error("from error");
    const captured = formatErrorEvent(
      undefined,
      undefined,
      undefined,
      undefined,
      err,
    );
    expect(captured.message).toBe("from error");
    expect(captured.stack).toBe(err.stack);
  });

  it("omits the location when there is no source", () => {
    const captured = formatErrorEvent("boom", undefined, 10, 5, undefined);
    expect(captured.message).toBe("boom");
  });

  it("handles a non-string, non-error message", () => {
    const captured = formatErrorEvent(
      undefined,
      undefined,
      undefined,
      undefined,
      { code: 1 },
    );
    expect(captured.message).toContain("object");
  });
});

describe("formatRejection", () => {
  it("reads an Error reason", () => {
    const err = new Error("nope");
    const captured = formatRejection(err);
    expect(captured.message).toBe("nope");
    expect(captured.stack).toBe(err.stack);
  });

  it("stringifies a non-Error reason", () => {
    expect(formatRejection("just a string").message).toBe("just a string");
  });

  it("defaults an empty reason", () => {
    expect(formatRejection(undefined).message).toBe("Unhandled rejection");
  });
});

describe("initErrorCapture", () => {
  it("forwards a window error and returns a teardown", () => {
    const original = window.onerror;
    const teardown = initErrorCapture();

    window.onerror?.("boom", "app.js", 1, 2, undefined);

    expect(logFrontendErrorMock).toHaveBeenCalledTimes(1);
    expect(logFrontendErrorMock.mock.calls[0][0]).toBe("error");
    expect(String(logFrontendErrorMock.mock.calls[0][1])).toContain("boom");

    teardown();
    expect(window.onerror).toBe(original);
  });
});