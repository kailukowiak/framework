import { describe, expect, it } from "vitest";
import { scratchworkResultIsLong } from "./ScratchworkResultViewer";

describe("scratchworkResultIsLong", () => {
  it("marks answers whose gutter preview withholds useful content", () => {
    expect(scratchworkResultIsLong("42.00")).toBe(false);
    expect(scratchworkResultIsLong("[1, 2, 3, 4, 5, 6, 7, 8]")).toBe(true);
    expect(scratchworkResultIsLong("first\nsecond")).toBe(true);
  });
});
