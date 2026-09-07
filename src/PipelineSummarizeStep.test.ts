import { describe, expect, it } from "vitest";
import { summarizeGroupColumn } from "./PipelineSummarizeStep";
import type { VisibleColumn } from "./lib/pipelineSteps";

const columns: VisibleColumn[] = [
  { id: "month", name: "Month", dataType: "date" },
  { id: "revenue", name: "Revenue", dataType: "currency" },
  { id: "region", name: "Region", dataType: "string" },
  { id: "channel", name: "Channel", dataType: "categorical" },
];

describe("the column a new group key starts on", () => {
  it("takes the selected column when it names things", () => {
    expect(summarizeGroupColumn(columns, "channel")?.id).toBe("channel");
  });

  it("ignores a selected measure and finds the first grouping column", () => {
    expect(summarizeGroupColumn(columns, "revenue")?.id).toBe("region");
  });

  it("falls back to the first grouping column, never the leading date", () => {
    expect(summarizeGroupColumn(columns)?.id).toBe("region");
  });

  it("takes the first column when nothing groups naturally", () => {
    const measures = columns.slice(0, 2);
    expect(summarizeGroupColumn(measures)?.id).toBe("month");
    expect(summarizeGroupColumn([])).toBe(undefined);
  });
});
