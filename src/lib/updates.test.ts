// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import {
  classifyUpdateFailure,
  clearSkippedUpdateVersion,
  recordUpdateCheck,
  shouldCheckInBackground,
  skipUpdateVersion,
  skippedUpdateVersion,
} from "./updates";

// The test environment provides no Web Storage, so the suite supplies one —
// same shape and same reason as connectorApproval.test.ts.
const stored = new Map<string, string>();
const localStorage = {
  get length() {
    return stored.size;
  },
  clear: () => stored.clear(),
  getItem: (key: string) => stored.get(key) ?? null,
  key: (index: number) => [...stored.keys()][index] ?? null,
  removeItem: (key: string) => stored.delete(key),
  setItem: (key: string, value: string) => stored.set(key, value),
};

beforeEach(() => {
  stored.clear();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: localStorage,
  });
});

describe("classifyUpdateFailure", () => {
  // The distinction that matters: a `.deb` user is not looking at a fault,
  // they are looking at the wrong question being asked, and telling them
  // "update failed" would send them hunting for a problem that is not there.
  it("reads a package-manager install as unsupported rather than broken", () => {
    expect(
      classifyUpdateFailure("Error: UPDATER_NOT_SUPPORTED: not an AppImage")
    ).toBe("unsupported");
    expect(classifyUpdateFailure("updater is not supported on this build")).toBe(
      "unsupported"
    );
  });

  it("treats anything else as a real failure", () => {
    expect(classifyUpdateFailure("error sending request for url")).toBe("failed");
    expect(classifyUpdateFailure("signature verification failed")).toBe("failed");
  });
});

describe("skipped version", () => {
  it("remembers only the version that was skipped", () => {
    expect(skippedUpdateVersion()).toBeNull();
    skipUpdateVersion("0.1.2");
    expect(skippedUpdateVersion()).toBe("0.1.2");
  });

  it("is replaced rather than accumulated, so a newer release still asks", () => {
    skipUpdateVersion("0.1.2");
    skipUpdateVersion("0.1.3");
    expect(skippedUpdateVersion()).toBe("0.1.3");
  });

  it("can be cleared, so a skip is not permanent", () => {
    skipUpdateVersion("0.1.2");
    clearSkippedUpdateVersion();
    expect(skippedUpdateVersion()).toBeNull();
  });
});

describe("shouldCheckInBackground", () => {
  it("checks when nothing has been recorded", () => {
    expect(shouldCheckInBackground(1_000_000)).toBe(true);
  });

  it("declines a second check inside the interval", () => {
    recordUpdateCheck(1_000_000);
    expect(shouldCheckInBackground(1_000_000 + 60_000, 3_600_000)).toBe(false);
  });

  it("checks again once the interval has passed", () => {
    recordUpdateCheck(1_000_000);
    expect(shouldCheckInBackground(1_000_000 + 3_600_001, 3_600_000)).toBe(true);
  });

  // A corrupt or hand-edited value must not wedge checking off permanently.
  it("checks when the stored timestamp is unusable", () => {
    window.localStorage.setItem("framework.lastUpdateCheck", "not-a-number");
    expect(shouldCheckInBackground(1_000_000)).toBe(true);
  });
});
