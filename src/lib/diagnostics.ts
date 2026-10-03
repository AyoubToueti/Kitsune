// Capture uncaught errors in the webview and forward them to the backend log.
//
// The Rust side cannot see a JS exception, and a release build has no console,
// so without this a frontend crash leaves no trace. `initErrorCapture` installs
// `window.onerror` and `unhandledrejection` handlers that call the
// `log_frontend_error` command.

import { logFrontendError } from "$lib/api/diagnostics";

/** What to log for one captured error. */
export interface CapturedError {
  level: "warn" | "error";
  message: string;
  stack?: string;
}

/**
 * Turn an `onerror` argument set into the fields the log call needs.
 *
 * Pure, so the awkward shapes (a thrown string, a missing stack, a rejection
 * with no reason) are unit tested without a browser. The source location is
 * folded into the message because the backend has no column for it, and a bare
 * "Script error." is useless without somewhere to look.
 */
export function formatErrorEvent(
  message: string | undefined,
  source: string | undefined,
  lineno: number | undefined,
  colno: number | undefined,
  error: unknown,
): CapturedError {
  const base =
    (typeof message === "string" && message.trim()) ||
    (error instanceof Error && error.message) ||
    String(error ?? "Unknown error");

  let location = "";
  if (source) {
    const line = lineno != null ? `:${lineno}` : "";
    const column = colno != null ? `:${colno}` : "";
    location = ` (${source}${line}${column})`;
  }

  const stack = error instanceof Error ? error.stack : undefined;

  return { level: "error", message: `${base}${location}`, stack };
}

/** Turn an unhandled rejection reason into the fields the log call needs. */
export function formatRejection(reason: unknown): CapturedError {
  if (reason instanceof Error) {
    return { level: "error", message: reason.message, stack: reason.stack };
  }
  return { level: "error", message: String(reason ?? "Unhandled rejection") };
}

/**
 * Install the global error handlers.
 *
 * Returns a teardown that restores the previous `window.onerror` and removes
 * the rejection listener, so a test (or HMR) can undo it. The layout owns the
 * single call; it is not meant to be installed more than once per page.
 */
export function initErrorCapture(): () => void {
  const previousOnerror = window.onerror;

  window.onerror = (message, source, lineno, colno, error) => {
    const captured = formatErrorEvent(
      typeof message === "string" ? message : undefined,
      source,
      lineno,
      colno,
      error,
    );
    void logFrontendError(captured.level, captured.message, captured.stack);
    // Do not claim the error: the console should still show it in development.
    return false;
  };

  const onRejection = (event: PromiseRejectionEvent) => {
    const captured = formatRejection(event.reason);
    void logFrontendError(captured.level, captured.message, captured.stack);
  };
  window.addEventListener("unhandledrejection", onRejection);

  return () => {
    window.onerror = previousOnerror;
    window.removeEventListener("unhandledrejection", onRejection);
  };
}