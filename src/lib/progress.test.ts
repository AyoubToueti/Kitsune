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

  /// A release played with no episode selected has no number, but the work
  /// still belongs on the list -- so the saver is called with `undefined` so it
  /// can mark the work Current without touching a stored progress.
  it("records a work with no episode number", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const recorder = createProgressRecorder(save);

    recorder.record(21);

    await vi.waitFor(() => expect(save).toHaveBeenCalledWith(21, undefined));
  });

  /// The no-number case is still deduplicated, keyed on the work.
  it("writes a number-less work only once", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const recorder = createProgressRecorder(save);

    recorder.record(21);
    recorder.record(21);

    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(1));
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

  /// A failed write used to vanish into an empty catch, so a work watched but
  /// never recorded left no clue. The reporter is how it surfaces.
  it("reports a failed save to onError", async () => {
    const failure = new Error("offline");
    const save = vi.fn().mockRejectedValue(failure);
    const onError = vi.fn();
    const recorder = createProgressRecorder(save, onError);

    recorder.record(21, 3);

    await vi.waitFor(() =>
      expect(onError).toHaveBeenCalledWith(21, 3, failure),
    );
  });

  /// The no-episode case has no number to report, but it is still a write that
  /// failed and must not pass silently.
  it("reports a number-less failure too", async () => {
    const save = vi.fn().mockRejectedValue(new Error("offline"));
    const onError = vi.fn();
    const recorder = createProgressRecorder(save, onError);

    recorder.record(21);

    await vi.waitFor(() =>
      expect(onError).toHaveBeenCalledWith(21, undefined, expect.anything()),
    );
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