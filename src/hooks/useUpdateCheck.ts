import type { Update } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useRef, useState } from "react";
import { isDesktopShell } from "../lib/applicationShortcuts";
import {
  checkForUpdate,
  installUpdate,
  recordUpdateCheck,
  shouldCheckInBackground,
  skipUpdateVersion,
  skippedUpdateVersion,
} from "../lib/updates";

export type UpdateProgress = { received: number; total: number | null };

export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "available"; version: string; notes: string | null }
  | { kind: "installing"; version: string; notes: string | null }
  | { kind: "up-to-date" }
  | { kind: "unsupported" }
  | { kind: "failed"; message: string };

/**
 * Owns when FrameWork asks about updates and what it does with the answer.
 *
 * The rule that shapes everything here: a background check may only ever
 * produce an update offer. It never reports that there was nothing to say, and
 * never surfaces a network failure — an application that interrupts you to
 * announce it has no news, or that GitHub was briefly unreachable, has made
 * your problem out of its own. Choosing Check for Updates inverts that: it is
 * an unambiguous request for an answer, so every outcome is shown, the
 * six-hour throttle is bypassed, and a previously skipped version is offered
 * again.
 */
export function useUpdateCheck() {
  const [status, setStatus] = useState<UpdateStatus>({ kind: "idle" });
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const pending = useRef<Update | null>(null);

  const runCheck = useCallback(async (explicit: boolean) => {
    // The browser dev server and the e2e shell have no updater plugin behind
    // them. Saying so is only useful to someone who asked.
    if (!isDesktopShell()) {
      if (explicit) setStatus({ kind: "unsupported" });
      return;
    }
    if (!explicit && !shouldCheckInBackground(Date.now())) return;
    if (explicit) setStatus({ kind: "checking" });

    const outcome = await checkForUpdate();
    recordUpdateCheck(Date.now());

    if (outcome.kind === "available") {
      if (!explicit && skippedUpdateVersion() === outcome.version) return;
      pending.current = outcome.update;
      setStatus({
        kind: "available",
        version: outcome.version,
        notes: outcome.notes,
      });
      return;
    }
    if (!explicit) return;
    setStatus(
      outcome.kind === "failed"
        ? { kind: "failed", message: outcome.message }
        : { kind: outcome.kind }
    );
  }, []);

  // One background check per window, on open. The throttle inside runCheck is
  // what keeps several windows from each producing a round trip and a prompt.
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void runCheck(false);
  }, [runCheck]);

  const check = useCallback(() => void runCheck(true), [runCheck]);

  const install = useCallback(() => {
    const update = pending.current;
    if (!update || status.kind !== "available") return;
    setStatus({ kind: "installing", version: status.version, notes: status.notes });
    setProgress({ received: 0, total: null });
    void installUpdate(update, (received, total) =>
      setProgress({ received, total })
    ).catch((reason) => {
      setProgress(null);
      setStatus({ kind: "failed", message: String(reason) });
    });
  }, [status]);

  const skip = useCallback(() => {
    if (status.kind === "available") skipUpdateVersion(status.version);
    setStatus({ kind: "idle" });
    setProgress(null);
  }, [status]);

  const dismiss = useCallback(() => {
    setStatus({ kind: "idle" });
    setProgress(null);
  }, []);

  return { status, progress, check, install, skip, dismiss };
}
