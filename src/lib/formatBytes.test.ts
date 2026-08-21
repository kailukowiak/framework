import { describe, expect, it } from "vitest";
import { formatBytes } from "./formatBytes";

describe("formatBytes", () => {
  it("counts small sizes in bytes", () => {
    expect(formatBytes(0)).toBe("0 bytes");
    expect(formatBytes(1)).toBe("1 byte");
    expect(formatBytes(999)).toBe("999 bytes");
  });

  it("climbs a unit at a time", () => {
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1024 * 340)).toBe("340 KB");
    expect(formatBytes(1024 * 1024 * 30.7)).toBe("30.7 MB");
    expect(formatBytes(1024 ** 3 * 1.25)).toBe("1.3 GB");
  });

  // A decimal that says nothing is noise, and kilobytes are never worth one.
  it("keeps the decimal only where it distinguishes anything", () => {
    expect(formatBytes(1024 * 512.4)).toBe("512 KB");
    expect(formatBytes(1024 * 1024 * 512.4)).toBe("512 MB");
  });

  it("does not report a negative or unreadable size", () => {
    expect(formatBytes(-1)).toBe("0 bytes");
    expect(formatBytes(Number.NaN)).toBe("0 bytes");
  });
});
