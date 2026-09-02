import { describe, expect, it } from "vitest";
import { parseNamedTransformation } from "../PipelineColumnNames";
import {
  outputColumnIdForName,
  parsePivotCommand,
  parseSortCommand,
  parseUnpivotCommand,
  reorderColumnIds,
} from "./pipelineStepCommands";

describe("pipeline step commands", () => {
  it("reads named transformations and forgives a redundant spreadsheet equals", () => {
    expect(
      parseNamedTransformation("`Currency Lower` = `currency`.str.to_lowercase()")
    ).toEqual({
      name: "Currency Lower",
      formula: "`currency`.str.to_lowercase()",
    });
    expect(parseNamedTransformation("`amount` == 10")).toEqual({
      name: "amount",
      formula: "10",
    });
    expect(parseNamedTransformation("`amount` = = `Revenue` - `Cost`")).toEqual({
      name: "amount",
      formula: "`Revenue` - `Cost`",
    });
    expect(parseNamedTransformation("`flag` = `Revenue` == `Cost`")).toEqual({
      name: "flag",
      formula: "`Revenue` == `Cost`",
    });
  });

  it("binds a same-name transformation to the existing column id", () => {
    const visible = [{ id: "memo", name: "Memo" }];
    expect(outputColumnIdForName(visible, "new-id", "Memo")).toBe("memo");
    expect(outputColumnIdForName(visible, "new-id", "Clean memo")).toBe("new-id");
  });

  it("reads ordered sort commands from the bar", () => {
    expect(
      parseSortCommand("`posted_date` desc, `line_no` asc", [
        { id: "date", name: "posted_date" },
        { id: "line", name: "line_no" },
      ])
    ).toMatchObject([
      { columnId: "date", descending: true },
      { columnId: "line", descending: false },
    ]);
  });

  it("places a dragged column on either side of its drop target", () => {
    expect(reorderColumnIds(["a", "b", "c", "d"], "a", "c", false)).toEqual([
      "b",
      "a",
      "c",
      "d",
    ]);
    expect(reorderColumnIds(["a", "b", "c", "d"], "a", "c", true)).toEqual([
      "b",
      "c",
      "a",
      "d",
    ]);
  });

  it("reads pivot and unpivot named commands", () => {
    expect(
      parsePivotCommand("columns=`period`, values=`amount`, aggregate=sum", [
        { id: "period", name: "period" },
        { id: "amount", name: "amount" },
      ])
    ).toEqual({
      namesColumnId: "period",
      valuesColumnId: "amount",
      aggregate: "sum",
    });
    expect(
      parseUnpivotCommand(
        'columns=`Jan`, starts_with("Q"), names=`Period`, values=`Amount`'
      )
    ).toEqual({
      columns: '`Jan`, starts_with("Q")',
      nameColumnName: "Period",
      valueColumnName: "Amount",
    });
  });
});
