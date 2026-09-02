import { describe, expect, it } from "vitest";
import {
  nextBlankColumnName,
  uniqueColumnName,
} from "../PipelineColumnNames";
import { stepsFromRendered } from "./pipelineChainEdits";
import {
  isOrderingOnlySelect,
  mintColumnId,
} from "./pipelineSteps";
import type { Column, FrameObject } from "./types";

const column = (id: string, name: string): Column => ({
  id,
  name,
  dataType: "string",
  formula: null,
});

describe("pipeline step drafts", () => {
  it("distinguishes placement bookkeeping from a real column choice", () => {
    const source = [column("amount", "amount"), column("memo", "memo")];
    const placed = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [{ outputColumnId: "calculated", formula: "1" }],
        },
        {
          kind: "select",
          columnIds: ["amount", "calculated", "memo"],
        },
      ],
      {
        columns: [
          column("amount", "amount"),
          column("calculated", "Calculated"),
          column("memo", "memo"),
        ],
      } as FrameObject,
      source
    );
    expect(isOrderingOnlySelect(source, placed, 1)).toBe(true);
    expect(placed[1]).toMatchObject({ kind: "select", mode: "placement" });

    if (placed[1].kind !== "select") return;
    const hidden = [placed[0], { ...placed[1], columnIds: ["amount", "calculated"] }];
    expect(isOrderingOnlySelect(source, hidden, 1)).toBe(false);
  });

  it("increments blank column names and suffixes explicit collisions", () => {
    expect(nextBlankColumnName(["Column 1", "Amount", "Column 4"])).toBe("Column 5");
    expect(uniqueColumnName("amount", ["amount", "amount_2"])).toBe("amount_3");
    expect(uniqueColumnName("amount_2", ["amount_2"])).toBe("amount_3");
    expect(uniqueColumnName("Column 1", ["Column 1", "Column 2"])).toBe("Column 3");
  });

  it("mints readable immutable column ids", () => {
    expect(mintColumnId("Net Revenue ($)")).toMatch(
      /^net_revenue~[0-9a-hjkmnp-tv-z]{6}$/
    );
    expect(mintColumnId("   ")).toMatch(/^column~[0-9a-hjkmnp-tv-z]{6}$/);
  });
});
