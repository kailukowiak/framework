import { describe, expect, it } from "vitest";
import type { Column } from "./lib/types";
import { columnQuickActions } from "./ColumnQuickActions";

const column = (name: string, dataType: Column["dataType"]): Column => ({
  id: "column",
  name,
  dataType,
  formula: null,
});

describe("column quick actions", () => {
  it("writes text cleanup as visible formulas against the chosen column", () => {
    expect(columnQuickActions(column("Customer name", "string")).map((item) => item.formula))
      .toEqual([
        "`Customer name`.str.strip_chars(None)",
        "`Customer name`.str.to_uppercase()",
        "`Customer name`.str.to_lowercase()",
      ]);
  });

  it("offers explicit missing-value defaults only when they are unambiguous", () => {
    expect(columnQuickActions(column("Amount", "number"))[0].formula).toBe(
      "`Amount`.fill_null(0)"
    );
    expect(columnQuickActions(column("Paid", "boolean"))[0].formula).toBe(
      "`Paid`.fill_null(False)"
    );
    expect(columnQuickActions(column("When", "date"))).toEqual([]);
  });
});
