// The shared internet speed-test state, for both surfaces that offer one.
//
// The Settings diagnostics panel and the watch modal's footer both run the same
// test and want the same progress/result/error. Rather than duplicate the
// subscription and the run-guard in each, they share this composable; each
// renders it differently.
//
// State is per-instance, so two surfaces mounted at once (the modal over the
// panel) each track their own run. The BACKEND is the single source of truth on
// "one at a time": it rejects a second concurrent test, and the second surface
// simply shows that error.

import { onSpeedTestProgress, runSpeedTest } from "$lib/api/diagnostics";
import type { SpeedTestProgress, SpeedTestResult } from "$lib/types";

import type { UnlistenFn } from "@tauri-apps/api/event";

/** The reactive surface a caller reads and drives. */
export interface SpeedTest {
  /** True while a test is running. */
  readonly running: boolean;
  /** The latest progress update, or `null` before the first one. */
  readonly progress: SpeedTestProgress | null;
  /** The result of the last completed test, or `null`. */
  readonly result: SpeedTestResult | null;
  /** The last failure, or `null`. Cleared when a new run starts. */
  readonly error: string | null;
  /** Start a test. A no-op while one is already running. */
  run: () => void;
  /** Subscribe to progress and return a teardown. Call from `onMount`. */
  start: () => () => void;
}

/**
 * Create a speed-test controller.
 *
 * `start()` installs the progress listener and returns the teardown; call it
 * from the component's `onMount`. `run()` begins a measurement.
 */
export function createSpeedTest(): SpeedTest {
  let running = $state(false);
  let progress = $state<SpeedTestProgress | null>(null);
  let result = $state<SpeedTestResult | null>(null);
  let error = $state<string | null>(null);

  /** True once `start()` has been called, so a second call is a no-op. */
  let started = false;

  /** A message from an unknown rejection. */
  function describe(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
  }

  async function run(): Promise<void> {
    if (running) return;
    running = true;
    error = null;
    result = null;
    progress = null;
    try {
      result = await runSpeedTest();
    } catch (err) {
      error = describe(err);
    } finally {
      running = false;
    }
  }

  /**
   * Install the progress listener.
   *
   * `listen` resolves asynchronously, so the teardown cannot simply close over
   * its result. A `cancelled` flag handles the race: if the component unmounts
   * before the promise resolves, the listener is unlistened the moment it
   * arrives rather than left attached.
   */
  function start(): () => void {
    if (started) return () => {};
    started = true;

    let cancelled = false;
    let unlisten: UnlistenFn | null = null;

    void onSpeedTestProgress((update) => {
      if (!cancelled) progress = update;
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // A missing event bus (e.g. in a test) is not fatal: the test can still
        // run, it just shows no live progress.
      });

    return () => {
      cancelled = true;
      unlisten?.();
      unlisten = null;
      started = false;
    };
  }

  return {
    get running() {
      return running;
    },
    get progress() {
      return progress;
    },
    get result() {
      return result;
    },
    get error() {
      return error;
    },
    run: () => void run(),
    start,
  };
}