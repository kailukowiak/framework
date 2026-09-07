import { describe, expect, it } from "vitest";
import { defaultPlotSpec } from "./canvasCards";
import type { Column, FrameObject } from "./types";

const column = (id: string, name: string, dataType: Column["dataType"]): Column =>
  ({ id, name, dataType, formula: null }) as Column;

const frame = (columns: Column[]) =>
  ({
    kind: "frame",
    id: "frame",
    name: "Monthly sales",
    columns,
    rows: [],
    uniqueKeys: [],
    summaries: [],
  }) as unknown as FrameObject;

const xEncoding = (spec: Record<string, unknown>) =>
  (spec.encoding as Record<string, Record<string, unknown>>).x;

describe("the first plot a frame gets", () => {
  // A declared sort is part of what the table is: a bar chart of a
  // month-sorted table used to draw the months in descending revenue.
  it("draws categories in the order the rows arrive", () => {
    const spec = defaultPlotSpec(
      frame([
        column("month", "Month", "string"),
        column("revenue", "Revenue", "currency"),
      ])
    );
    expect(xEncoding(spec).sort).toBe(null);
    expect(xEncoding(spec).type).toBe("nominal");
  });

  it("keeps the row order for a counted single column too", () => {
    const spec = defaultPlotSpec(frame([column("month", "Month", "string")]));
    expect(xEncoding(spec).sort).toBe(null);
  });

  it("leaves an axis that orders itself by value alone", () => {
    const temporal = defaultPlotSpec(
      frame([
        column("day", "Day", "date"),
        column("revenue", "Revenue", "currency"),
      ])
    );
    expect(xEncoding(temporal).sort).toBe(undefined);
    expect(xEncoding(temporal).type).toBe("temporal");

    const scatter = defaultPlotSpec(
      frame([
        column("cost", "Cost", "number"),
        column("revenue", "Revenue", "currency"),
      ])
    );
    expect(xEncoding(scatter).sort).toBe(undefined);
    expect(xEncoding(scatter).type).toBe("quantitative");
  });
});
