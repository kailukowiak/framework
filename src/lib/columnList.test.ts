import { describe, expect, it } from "vitest";
import { meltedColumnIds } from "./columnList";

const visible = [
  { id: "region", name: "Region" },
  { id: "q1", name: "Q1" },
  { id: "q2", name: "Q2" },
  { id: "notes", name: "Notes, kept" },
];

describe("meltedColumnIds", () => {
  it("resolves backticked names in written order", () => {
    expect(meltedColumnIds("`Q2`, `Q1`", visible)).toEqual(["q2", "q1"]);
  });

  it("keeps a comma inside a backticked name with its name", () => {
    expect(meltedColumnIds("`Notes, kept`", visible)).toEqual(["notes"]);
  });

  it("melts nothing for a piece it cannot read", () => {
    expect(meltedColumnIds("`Q1` + 1, `Q2`", visible)).toEqual(["q2"]);
    expect(meltedColumnIds("`Q1", visible)).toEqual([]);
  });

  it("sweeps a pattern selector over the visible names", () => {
    expect(meltedColumnIds('starts_with("Q")', visible)).toEqual(["q1", "q2"]);
    expect(meltedColumnIds('contains("otes")', visible)).toEqual(["notes"]);
  });

  it("matches patterns exactly, capitals included, like the core", () => {
    expect(meltedColumnIds('starts_with("q")', visible)).toEqual([]);
  });

  it("skips what a sweep would add twice", () => {
    expect(meltedColumnIds('`Q2`, starts_with("Q")', visible)).toEqual([
      "q2",
      "q1",
    ]);
  });

  it("melts everything but the excepted columns", () => {
    expect(meltedColumnIds("except(`Region`)", visible)).toEqual([
      "q1",
      "q2",
      "notes",
    ]);
  });

  it("treats a comma inside a selector's pattern as part of the pattern", () => {
    expect(meltedColumnIds('contains(",")', visible)).toEqual(["notes"]);
  });

  it("reads an empty pattern as matching nothing rather than everything", () => {
    expect(meltedColumnIds('starts_with("")', visible)).toEqual([]);
  });
});
