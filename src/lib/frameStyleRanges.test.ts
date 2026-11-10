import { describe, expect, it } from "vitest";
import { effectiveFrameCellStyle } from "../FrameGrid";
import { emptyStyle } from "./conditionalFormatting";
import { rectangleHasFormatting } from "./frameStyleRanges";
import type { FrameObject, FrameStyle } from "./types";

const frameWith = (styles: FrameStyle[]): FrameObject =>
  ({
    kind: "frame",
    id: "f",
    name: "F",
    columns: [],
    rows: [],
    display: { styles },
  }) as unknown as FrameObject;

const style = (patch: object) => ({ ...emptyStyle(), ...patch });
const range = (rowIds: string[], columnIds: string[], patch: object): FrameStyle => ({
  target: { kind: "range", rowIds, columnIds },
  style: style(patch),
});

describe("range resolution order", () => {
  it("sits above frame, column and row", () => {
    const frame = frameWith([
      { target: { kind: "frame" }, style: style({ fillColor: "#111111" }) },
      { target: { kind: "column", columnId: "c" }, style: style({ fillColor: "#222222" }) },
      { target: { kind: "row", rowId: "r" }, style: style({ fillColor: "#333333" }) },
      range(["r"], ["c"], { fillColor: "#444444" }),
    ]);
    expect(effectiveFrameCellStyle(frame, "r", "c").fillColor).toBe("#444444");
  });

  it("sits below a cell format and below rule answers", () => {
    const frame = frameWith([
      range(["r"], ["c"], { bold: true, fillColor: "#444444" }),
      { target: { kind: "cell", rowId: "r", columnId: "c" }, style: style({ fillColor: "#555555" }) },
    ]);
    const style_ = effectiveFrameCellStyle(frame, "r", "c");
    expect(style_.fillColor).toBe("#555555");
    expect(style_.bold).toBe(true);
  });

  it("lets a later range win only the properties it carries", () => {
    const frame = frameWith([
      range(["r"], ["c"], { bold: true, fillColor: "#aaaaaa" }),
      range(["r"], ["c"], { fillColor: "#bbbbbb" }),
    ]);
    const got = effectiveFrameCellStyle(frame, "r", "c");
    expect(got.fillColor).toBe("#bbbbbb");
    expect(got.bold).toBe(true);
  });

  it("applies only inside its rows and columns", () => {
    const frame = frameWith([range(["r"], ["c"], { bold: true })]);
    expect(effectiveFrameCellStyle(frame, "r", "other").bold).toBeNull();
    expect(effectiveFrameCellStyle(frame, "other", "c").bold).toBeNull();
  });

  it("never reaches the row gutter or a column header", () => {
    const frame = frameWith([range(["r"], ["c"], { bold: true })]);
    expect(effectiveFrameCellStyle(frame, "r", undefined).bold).toBeNull();
    expect(effectiveFrameCellStyle(frame, undefined, "c").bold).toBeNull();
  });
});

describe("rectangleHasFormatting", () => {
  it("sees cell formats and overlapping ranges, not row formats", () => {
    const styles: FrameStyle[] = [
      { target: { kind: "row", rowId: "r" }, style: style({ bold: true }) },
    ];
    expect(rectangleHasFormatting(styles, ["r"], ["c"])).toBe(false);
    expect(
      rectangleHasFormatting([...styles, range(["r", "s"], ["c"], { bold: true })], ["r"], ["c"])
    ).toBe(true);
    expect(
      rectangleHasFormatting(
        [{ target: { kind: "cell", rowId: "r", columnId: "c" }, style: style({ bold: true }) }],
        ["r"],
        ["c"]
      )
    ).toBe(true);
  });
});
