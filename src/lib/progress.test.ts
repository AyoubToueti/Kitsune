import { describe, it, expect, vi } from "vitest";

import { createProgressRecorder } from "./progress";

describe("createProgressRecorder", () => {
  it("saves the anime and episode", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const recorder = createProgressRecorder(save);

    recorder.record(21, 3);
    await vi.waitFor(() => expect(save).toHaveBeenCalledWith(21, 3));
  });

  /// Resolving the stream, switching files and re-selecting one all reach the
  /// play path, so the same episode must not be written every time.
  it("writes an episode only once", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const recorder = createProgressRecorder(save);

    recorder.record(21, 3);
    recorder.record(21, 3);
    recorder.record(21, 3);
    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(1));
  });

  /// The same number belongs to a different work on another page, so the pair
  /// is the key rather than the episode alone.
  it("treats the same episode of another work separately", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const recorder = createProgressRecorder(save);

    recorder.record(21, 3);
    recorder.record(22, 3);

    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(2));
  });

  it("does not await the save", () => {
    // A promise that never settles: `record` must return regardless.
    const save = vi.fn().mockReturnValue(new Promise(() => {}));
    const recorder = createProgressRecorder(save);

    expect(() => recorder.record(21, 3)).not.toThrow();
  });

  /// Progress is a side effect of watching; a failed write must not throw into
  /// the play path.
  it("swallows a failed save", async () => {
    const save = vi.fn().mockRejectedValue(new Error("offline"));
    const recorder = createProgressRecorder(save);

    expect(() => recorder.record(21, 3)).not.toThrow();
    await vi.waitFor(() => expect(save).toHaveBeenCalled());
  });

  /// A transient failure should not suppress the episode for the whole visit.
  it("retries an episode after a failure", async () => {
    const save = vi
      .fn()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValue(undefined);
    const recorder = createProgressRecorder(save);

    recorder.record(21, 3);
    // Let the rejection settle and clear the key.
    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(1));

    recorder.record(21, 3);
    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(2));
  });
});