import { describe, expect, it } from "vitest";
import { transformationForTarget } from "./targetedTransformation";

describe("transformationForTarget", () => {
  it("reads a named line by its own name", () => {
    expect(transformationForTarget("`Profit` = `Revenue` - `Cost`", "Margin")).toEqual({
      name: "Profit",
      formula: "`Revenue` - `Cost`",
    });
  });

  it("reads a line without an assignment as the target column's expression", () => {
    expect(transformationForTarget("10*2", "Revenue")).toEqual({
      name: "Revenue",
      formula: "10*2",
    });
    expect(transformationForTarget("= `Cost` >= 10", "Revenue")).toEqual({
      name: "Revenue",
      formula: "`Cost` >= 10",
    });
    expect(transformationForTarget('`Note` != "a=b"', "Flag")).toEqual({
      name: "Flag",
      formula: '`Note` != "a=b"',
    });
  });

  it("still refuses a half-written assignment or an empty line", () => {
    expect(transformationForTarget("`Margin` =", "Margin")).toBeNull();
    expect(transformationForTarget("1 + = 2", "Margin")).toBeNull();
    expect(transformationForTarget("  ", "Margin")).toBeNull();
    expect(transformationForTarget("=", "Margin")).toBeNull();
  });
});
