import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

const SKIP_KEY = "framework.skippedUpdateVersion";

/**
 * Tauri can only replace a running application it installed itself, which on
 * Linux means an AppImage. A `.deb` or `.rpm` belongs to apt or dnf, and
 * overwriting files a package manager owns would leave its database lying
 * about what is on disk — so the plugin refuses, correctly. That refusal is
 * not a fault the person reading it can act on the way a network error is, so
 * it is classified apart and worded as guidance rather than failure.
 */
export function classifyUpdateFailure(message: string): "unsupported" | "failed" {
  return /appimage|not\s+supported|unsupported|no\s+updater/i.test(message)
    ? "unsupported"
    : "failed";
}

/** The version someone chose to pass over, so opening a window does not ask
 *  again about the same release. Choosing Check for Updates deliberately is a
 *  different act and ignores this. */
export function skippedUpdateVersion(): string | null {
  try {
    return window.localStorage.getItem(SKIP_KEY);
  } catch {
    return null;
  }
}

export function skipUpdateVersion(version: string): void {
  try {
    window.localStorage.setItem(SKIP_KEY, version);
  } catch {
    /* Storage being unavailable costs a redundant prompt, never correctness. */
  }
}

export function clearSkippedUpdateVersion(): void {
  try {
    window.localStorage.removeItem(SKIP_KEY);
  } catch {
    /* As above. */
  }
}

export type UpdateOutcome =
  | { kind: "available"; version: string; notes: string | null; update: Update }
  | { kind: "up-to-date" }
  | { kind: "unsupported" }
  | { kind: "failed"; message: string };

/** Asks the endpoint what the newest release is. Never throws: every failure
 *  is an outcome the caller can render, because a background check that
 *  raises is a background check that shows someone an error they did not ask
 *  for. */
export async function checkForUpdate(): Promise<UpdateOutcome> {
  try {
    const update = await check();
    if (!update) return { kind: "up-to-date" };
    return {
      kind: "available",
      version: update.version,
      notes: update.body ?? null,
      update,
    };
  } catch (reason) {
    const message = String(reason);
    return classifyUpdateFailure(message) === "unsupported"
      ? { kind: "unsupported" }
      : { kind: "failed", message };
  }
}

/** Downloads and installs, reporting bytes so a long download does not look
 *  like a hang, then relaunches into the new version. */
export async function installUpdate(
  update: Update,
  onProgress: (received: number, total: number | null) => void
): Promise<void> {
  let received = 0;
  let total: number | null = null;
  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        total = event.data.contentLength ?? null;
        onProgress(0, total);
        break;
      case "Progress":
        received += event.data.chunkLength;
        onProgress(received, total);
        break;
      case "Finished":
        onProgress(total ?? received, total);
        break;
    }
  });
  await relaunch();
}

const LAST_CHECK_KEY = "framework.lastUpdateCheck";
const BACKGROUND_INTERVAL_MS = 6 * 60 * 60 * 1000;

/**
 * Whether a check that nobody asked for is due. Two things make this worth
 * throttling rather than checking on every window: documents open in separate
 * windows, each with its own webview, so an unthrottled check would ask three
 * times for three open documents; and someone who opens FrameWork ten times in
 * an afternoon does not need ten round trips to GitHub to hear the same
 * answer. Choosing Check for Updates bypasses this entirely — asking outright
 * deserves a real answer, however recently one was fetched.
 */
export function shouldCheckInBackground(
  now: number,
  intervalMs: number = BACKGROUND_INTERVAL_MS
): boolean {
  try {
    const last = Number(window.localStorage.getItem(LAST_CHECK_KEY));
    return !Number.isFinite(last) || last <= 0 || now - last >= intervalMs;
  } catch {
    return true;
  }
}

export function recordUpdateCheck(now: number): void {
  try {
    window.localStorage.setItem(LAST_CHECK_KEY, String(now));
  } catch {
    /* Losing the timestamp costs an extra check, never correctness. */
  }
}
