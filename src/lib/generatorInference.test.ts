import { describe, expect, it } from "vitest";
import {
  inferDateGeneratorPattern,
  inferNumericGeneratorPattern,
} from "./generatorInference";

describe("numeric generator inference", () => {
  it("reads the start and step from a selected ascending run", () => {
    expect(inferNumericGeneratorPattern(["1", "2", "3"])).toEqual({
      kind: "number",
      start: 1,
      step: 1,
    });
  });

  it("ignores blank cells that the frame will retain as nulls", () => {
    expect(inferNumericGeneratorPattern(["10", "", "8", "6", ""])).toEqual({
      kind: "number",
      start: 10,
      step: -2,
    });
  });

  it("refuses a run that is not an arithmetic series", () => {
    expect(inferNumericGeneratorPattern(["1", "2", "4"])).toBeNull();
  });
});

describe("date generator inference", () => {
  it("recognizes a daily date run", () => {
    expect(inferDateGeneratorPattern(["2026-01-01", "2026-01-02", "2026-01-03"]))
      .toEqual({ kind: "date", start: "2026-01-01", step: 1, unit: "d" });
  });

  it("recognizes calendar months, including a month-end run", () => {
    expect(inferDateGeneratorPattern(["2026-01-31", "2026-02-28", "2026-03-31"]))
      .toEqual({ kind: "date", start: "2026-01-31", step: 1, unit: "mo" });
  });
});
