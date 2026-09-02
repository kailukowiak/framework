import { invoke } from "@tauri-apps/api/core";

/**
 * The one place a client-side failure becomes visible outside the browser
 * devtools it happened in. Every path here ends in `console.error`/`warn`
 * plus a best-effort forward to the Rust shell's `log_frontend_event`
 * command, which writes into the same log as document open/save (see
 * `src-tauri/src/lib.rs`) -- the terminal running `npm run tauri dev`, or
 * Console.app for a bundled build.
 *
 * This module talks to `@tauri-apps/api/core` directly rather than through
 * `src/lib/invoke.ts`. That wrapper reports a failed command through this
 * module, so if this module reported *its own* forwarding failures the same
 * way, a broken `log_frontend_event` call would recurse into itself
 * forever. The rule that prevents that: nothing inside `forwardToBackend`'s
 * own failure path may call back into `reportError`/`reportWarning`. A
 * failed forward is swallowed, once, with nothing left to report it to.
 *
 * This also makes forwarding safe when there is no Tauri backend at all
 * (vitest, a browser tab with no Tauri runtime): `invoke` is declared
 * `async`, so even `window.__TAURI_INTERNALS__` being undefined surfaces as
 * an ordinary rejection here, not a thrown exception.
 */

type ReportedLevel = "info" | "warn" | "error";

function forwardToBackend(
  level: ReportedLevel,
  message: string,
  context?: string
): void {
  void invoke("log_frontend_event", { level, message, context }).catch(() => {
    // Nowhere left to report a broken logger to -- see the module comment.
  });
}

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Logs and forwards an error-level event. */
export function reportError(message: string, context?: string): void {
  console.error(context ? `${message} (${context})` : message);
  forwardToBackend("error", message, context);
}

/** Logs and forwards a warn-level event. */
export function reportWarning(message: string, context?: string): void {
  console.warn(context ? `${message} (${context})` : message);
  forwardToBackend("warn", message, context);
}

/**
 * Reports a rejected `invoke` call. Only the argument *names* are included,
 * never their values -- command arguments routinely carry document content.
 */
export function reportCommandFailure(
  command: string,
  error: unknown,
  argKeys?: string[]
): void {
  const context = argKeys?.length ? `args: ${argKeys.join(", ")}` : undefined;
  reportError(`command failed: ${command}: ${describeError(error)}`, context);
}

/**
 * A curried helper for the sites that deliberately swallow a rejection --
 * `.catch(reportIgnoredFailure("history menu state"))` instead of
 * `.catch(() => {})`. The user was never going to see these; this just
 * keeps them from vanishing with no trace at all.
 */
export function reportIgnoredFailure(what: string) {
  return (error: unknown): void => {
    reportWarning(`ignored failure: ${what}: ${describeError(error)}`);
  };
}

/** Registers `window` listeners so an uncaught exception or unhandled
 * promise rejection is reported instead of only blanking the window. */
export function installGlobalErrorReporting(): void {
  window.addEventListener("error", (event) => {
    const location = event.filename
      ? `${event.filename}:${event.lineno}:${event.colno}`
      : undefined;
    reportError(describeError(event.error ?? event.message), location);
  });
  window.addEventListener("unhandledrejection", (event) => {
    reportError(describeError(event.reason), "unhandled rejection");
  });
}

/** Reports a React render-time exception caught by `ErrorBoundary`. */
export function reportComponentError(
  error: unknown,
  componentStack: string | null
): void {
  reportError(
    `component crashed: ${describeError(error)}`,
    componentStack ?? undefined
  );
}
