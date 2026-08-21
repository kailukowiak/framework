import { describe, expect, it } from "vitest";
import {
  appendBlankCalculatedColumn,
  appendColumnFilter,
  appendInPlaceColumnTransformation,
  appendOrderedColumnTransformation,
  focusExistingCalculatedColumn,
  columnFilterDraft,
  nextBlankColumnName,
  normalizeCalculatedColumnNames,
  outputColumnIdForName,
  parseNamedTransformation,
  parsePivotCommand,
  parseSortCommand,
  parseUnpivotCommand,
  rearrangePipelineColumns,
  reorderColumnIds,
  hidePipelineColumn,
  isOrderingOnlySelect,
  mintColumnId,
  stepsFromRendered,
  uniqueColumnName,
} from "./PipelineEditor";
import { formatPipelineFormulas } from "./PipelineFormulaFormatting";
import type { Column, RenderedFrameStep, FrameObject } from "./lib/types";

const column = (id: string, name: string): Column => ({
  id,
  name,
  dataType: "string",
  formula: null,
});

describe("stepsFromRendered", () => {
  it("reopens canonical engine formulas in the automatic multiline layout", () => {
    const source = column("date", "Date");
    const calculated = column("month", "Month");
    const frame = {
      kind: "frame",
      id: "calendar",
      name: "Calendar",
      columns: [source, calculated],
      rows: [],
      derivation: null,
      uniqueKeys: [],
      summaries: [],
    } as FrameObject;
    const drafts = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [
            {
              outputColumnId: calculated.id,
              formula: "`Date`.dt.month_start()",
            },
          ],
        },
      ],
      frame,
      [source]
    );
    expect(drafts[0]).toMatchObject({
      columns: [{ formula: "`Date`\n  .dt\n  .month_start()" }],
    });
  });

  it("normalizes formula layout at the save boundary", () => {
    const formatted = formatPipelineFormulas({
      id: "step",
      kind: "filter",
      predicates: [{ id: "condition", formula: "value.is_not_null()" }],
      matchAll: true,
    });
    expect(formatted).toMatchObject({
      predicates: [{ formula: "value\n  .is_not_null()" }],
    });
  });

  it("seeds a clicked string filter with only its example value selected", () => {
    expect(columnFilterDraft(column("memo", "Account name"))).toEqual({
      formula: '`Account name` == "value"',
      focusSelection: { start: 19, end: 24 },
    });
  });

  it("adds header filters as explicit conditions on a trailing filter", () => {
    const first = appendColumnFilter([], column("a", "Account"), 4);
    const next = appendColumnFilter(first, column("b", "Region"), 5);
    expect(next).toHaveLength(1);
    expect(next[0]).toMatchObject({
      kind: "filter",
      matchAll: true,
      predicates: [
        { formula: '`Account` == "value"', focusToken: 4 },
        { formula: '`Region` == "value"', focusToken: 5 },
      ],
    });
  });

  it("focuses the final declaration of an existing calculated column", () => {
    const first = appendInPlaceColumnTransformation(
      [],
      { id: "balance", name: "Balance" },
      "1"
    );
    const steps = appendInPlaceColumnTransformation(
      first,
      { id: "balance", name: "Balance" },
      "`Balance` + 1"
    );
    const focused = focusExistingCalculatedColumn(steps, "balance", 9, 3);
    expect(focused?.[0]).toBe(steps[0]);
    expect(focused?.[1]).toMatchObject({
      columns: [{ focusToken: 9, focusAtEnd: false, anchorRowIndex: 3 }],
    });
  });

  it("can select a new transformation's formula for header equals editing", () => {
    const [step] = appendInPlaceColumnTransformation(
      [],
      { id: "amount", name: "Amount" },
      "`Amount`",
      12,
      false
    );
    expect(step).toMatchObject({
      kind: "withColumns",
      columns: [{ focusToken: 12, focusAtEnd: false }],
    });
  });

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

  it("records a frame-header reorder as an authored rearrange step", () => {
    const next = rearrangePipelineColumns([], ["memo", "account", "amount"]);
    expect(next).toMatchObject([
      {
        kind: "select",
        mode: "rearrange",
        columnIds: ["memo", "account", "amount"],
      },
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

  it("keeps linked pass-through names after a summarize replaces the final schema", () => {
    const sourceColumns = [
      column("source-period", "period"),
      column("source-credit", "credit"),
    ];
    const rendered: RenderedFrameStep[] = [
      {
        kind: "withColumns",
        columns: [
          { outputColumnId: "linked-period", formula: "`period`" },
          { outputColumnId: "linked-credit", formula: "`credit`" },
        ],
      },
      { kind: "select", columnIds: ["linked-period", "linked-credit"] },
      {
        kind: "summarize",
        groupKeys: [{ outputColumnId: "group-period", formula: "`period`" }],
        aggregates: [{ outputColumnId: "credit-sum", formula: "`credit`.sum()" }],
        maintainOrder: true,
      },
    ];
    const summarized = {
      kind: "frame",
      id: "summary",
      name: "Summary",
      columns: [column("group-period", "Period"), column("credit-sum", "Credit Sum")],
      rows: [],
      derivation: { sourceFrameId: "source", join: null },
      uniqueKeys: [],
      summaries: [],
    } as FrameObject;

    const drafts = stepsFromRendered(rendered, summarized, sourceColumns);
    const projection = drafts[0];
    expect(projection.kind).toBe("withColumns");
    if (projection.kind !== "withColumns") return;
    expect(projection.columns.map((item) => item.name)).toEqual(["period", "credit"]);
  });

  it("appends a visible typed-null calculated column at the bottom", () => {
    const existing = stepsFromRendered(
      [{ kind: "filter", predicates: ["true"], matchAll: true }],
      {} as FrameObject,
      []
    );
    const steps = appendBlankCalculatedColumn(existing, 7);
    expect(steps).toHaveLength(2);
    expect(steps[0].kind).toBe("filter");
    expect(steps[1].kind).toBe("withColumns");
    if (steps[1].kind !== "withColumns") return;
    expect(steps[1].columns[0]).toMatchObject({
      fallbackName: "Column 1",
      formula: 'null.cast("number")',
      focusToken: 7,
    });
  });

  it("keeps the grid row that anchored a point-built formula", () => {
    const steps = appendBlankCalculatedColumn([], 8, undefined, [], 0, 12);
    expect(steps[0].kind).toBe("withColumns");
    if (steps[0].kind !== "withColumns") return;
    expect(steps[0].columns[0].anchorRowIndex).toBe(12);
  });

  it("appends a same-id transformation so Polars replaces the column", () => {
    const steps = appendInPlaceColumnTransformation(
      [],
      { id: "memo", name: "Memo" },
      "`Memo`.str.to_uppercase()",
      12
    );
    expect(steps).toMatchObject([
      {
        kind: "withColumns",
        columns: [
          {
            outputColumnId: "memo",
            fallbackName: "Memo",
            formula: "`Memo`.str.to_uppercase()",
            focusToken: 12,
            focusAtEnd: true,
          },
        ],
      },
    ]);
  });

  it("puts a visible sort before a row-position transformation", () => {
    const steps = appendOrderedColumnTransformation(
      [],
      { id: "row", name: "Row" },
      "sequence(1, frame.len() + 1)",
      "date"
    );
    expect(steps).toMatchObject([
      { kind: "sort", keys: [{ columnId: "date", descending: false }] },
      {
        kind: "withColumns",
        columns: [{ outputColumnId: "row", formula: "sequence(1, frame.len() + 1)" }],
      },
    ]);
  });

  it("turns a recurrence wrapper into a visual calculate-down step", () => {
    const steps = appendOrderedColumnTransformation(
      [],
      { id: "balance", name: "Balance" },
      "recur(`Change`, previous() + `Change`, restart_by=[`Account`])",
      "date",
      14
    );
    expect(steps).toMatchObject([
      { kind: "sort", keys: [{ columnId: "date", descending: false }] },
      {
        kind: "recurrence",
        outputColumnId: "balance",
        name: "Balance",
        seed: "`Change`",
        formula: "previous() + `Change`",
        partitionName: "Account",
        focusToken: 14,
      },
    ]);
    expect(focusExistingCalculatedColumn(steps, "balance", 15, 2)?.[1]).toMatchObject({
      kind: "recurrence",
      focusToken: 15,
      focusAtEnd: false,
      anchorRowIndex: 2,
    });
  });

  it("rebuilds a saved recurrence as its two visual fields", () => {
    const drafts = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [
            {
              outputColumnId: "balance",
              formula: "recur(`Change`, previous() + `Change`, restart_by=[`Account`])",
            },
          ],
        },
      ],
      { columns: [column("balance", "Balance")] } as FrameObject,
      [column("change", "Change"), column("account", "Account")]
    );
    expect(drafts).toMatchObject([
      {
        kind: "recurrence",
        outputColumnId: "balance",
        name: "Balance",
        seed: "`Change`",
        formula: "previous() + `Change`",
        partitionName: "Account",
      },
    ]);
  });

  it("places a context-added column immediately after the clicked column", () => {
    const steps = appendBlankCalculatedColumn([], 8, "amount", [
      { id: "account", name: "account" },
      { id: "amount", name: "amount" },
      { id: "memo", name: "memo" },
    ]);
    expect(steps).toHaveLength(2);
    expect(steps[0].kind).toBe("withColumns");
    if (steps[0].kind !== "withColumns") return;
    const outputColumnId = steps[0].columns[0].outputColumnId;
    expect(steps[1]).toMatchObject({
      kind: "select",
      columnIds: ["account", "amount", outputColumnId, "memo"],
    });
  });

  it("reuses the preceding Add columns step when only a projection follows", () => {
    const existing = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [{ outputColumnId: "first", formula: "1" }],
        },
        {
          kind: "select",
          columnIds: ["amount", "first", "memo"],
        },
      ],
      {
        columns: [
          column("amount", "amount"),
          column("first", "First"),
          column("memo", "memo"),
        ],
      } as FrameObject,
      [column("amount", "amount"), column("memo", "memo")]
    );
    const steps = appendBlankCalculatedColumn(existing, 9, "amount", [
      { id: "amount", name: "amount" },
      { id: "first", name: "First" },
      { id: "memo", name: "memo" },
    ]);

    expect(steps).toHaveLength(2);
    expect(steps[0]).toMatchObject({
      kind: "withColumns",
      columns: [{ outputColumnId: "first" }, { focusToken: 9 }],
    });
    if (steps[0].kind !== "withColumns" || steps[1].kind !== "select") return;
    expect(steps[1].columnIds).toEqual([
      "amount",
      steps[0].columns[1].outputColumnId,
      "first",
      "memo",
    ]);
  });

  it("keeps clicked placement when reusing a final Add columns step", () => {
    const existing = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [{ outputColumnId: "first", formula: "1" }],
        },
      ],
      {
        columns: [column("amount", "amount"), column("first", "First")],
      } as FrameObject,
      [column("amount", "amount")]
    );
    const steps = appendBlankCalculatedColumn(existing, 10, "amount", [
      { id: "amount", name: "amount" },
      { id: "first", name: "First" },
    ]);

    expect(steps).toHaveLength(2);
    if (steps[0].kind !== "withColumns" || steps[1].kind !== "select") return;
    expect(steps[0].columns).toHaveLength(2);
    expect(steps[1].columnIds).toEqual([
      "amount",
      steps[0].columns[1].outputColumnId,
      "first",
    ]);
  });

  it("keeps a new Add columns step below a real transformation boundary", () => {
    const existing = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [{ outputColumnId: "first", formula: "1" }],
        },
        { kind: "sort", keys: [{ columnId: "amount", descending: false }] },
        { kind: "select", columnIds: ["amount", "first"] },
      ],
      {
        columns: [column("amount", "amount"), column("first", "First")],
      } as FrameObject,
      [column("amount", "amount")]
    );
    const steps = appendBlankCalculatedColumn(existing, 10, undefined, [
      { id: "amount", name: "amount" },
      { id: "first", name: "First" },
    ]);

    expect(steps).toHaveLength(4);
    expect(steps[3].kind).toBe("withColumns");
  });

  it("does not merge an authored column into a linked frame's hidden projection", () => {
    const hidden = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [{ outputColumnId: "linked", formula: "`amount`" }],
        },
        { kind: "select", columnIds: ["linked"] },
      ],
      { columns: [column("linked", "amount")] } as FrameObject,
      [column("source-amount", "amount")]
    );
    const steps = appendBlankCalculatedColumn(
      hidden,
      11,
      undefined,
      [{ id: "linked", name: "amount" }],
      2
    );

    expect(steps).toHaveLength(3);
    expect(steps[0]).toMatchObject({
      kind: "withColumns",
      columns: [{ outputColumnId: "linked" }],
    });
    expect(steps[2].kind).toBe("withColumns");
  });

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

  it("reads projections back as delete or rearrange decisions", () => {
    const source = [
      column("account", "account"),
      column("amount", "amount"),
      column("memo", "memo"),
    ];
    const frame = { columns: source } as FrameObject;

    expect(
      stepsFromRendered(
        [{ kind: "select", columnIds: ["account", "amount"] }],
        frame,
        source
      )[0]
    ).toMatchObject({
      kind: "select",
      mode: "delete",
      columnIds: ["account", "amount"],
    });
    expect(
      stepsFromRendered(
        [
          {
            kind: "select",
            columnIds: ["memo", "account", "amount"],
          },
        ],
        frame,
        source
      )[0]
    ).toMatchObject({
      kind: "select",
      mode: "rearrange",
      columnIds: ["memo", "account", "amount"],
    });
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

  it("normalizes typed and formula-suggested pipeline names before save", () => {
    const drafts = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [
            { outputColumnId: "first", formula: "1" },
            { outputColumnId: "second", formula: "2" },
          ],
        },
      ],
      {
        columns: [column("first", "amount"), column("second", "amount")],
      } as FrameObject,
      [column("amount", "amount")]
    );
    expect(
      normalizeCalculatedColumnNames(drafts, [column("amount", "amount")], 0)[0]
    ).toMatchObject({
      kind: "withColumns",
      columns: [{ name: "amount_2" }, { name: "amount_3" }],
    });
  });

  it("preserves an intentional same-id, same-name overwrite", () => {
    const drafts = appendInPlaceColumnTransformation(
      [],
      { id: "amount", name: "amount" },
      '`amount`.cast("integer")'
    );
    expect(
      normalizeCalculatedColumnNames(drafts, [column("amount", "amount")], 0)[0]
    ).toMatchObject({
      kind: "withColumns",
      columns: [{ outputColumnId: "amount", fallbackName: "amount" }],
    });
  });

  it("hides a computed output with a final select without removing its formula", () => {
    const drafts = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [
            { outputColumnId: "kept", formula: "`amount`" },
            { outputColumnId: "deleted", formula: "`amount` * 2" },
          ],
        },
      ],
      {
        columns: [column("kept", "Kept"), column("deleted", "Deleted")],
      } as FrameObject,
      [column("amount", "amount")]
    );
    const next = hidePipelineColumn(drafts, "deleted", ["kept", "deleted"]);
    expect(next).not.toBeNull();
    expect(next?.[0]).toMatchObject({
      kind: "withColumns",
      columns: [{ outputColumnId: "kept" }, { outputColumnId: "deleted" }],
    });
    expect(next?.[1]).toMatchObject({ kind: "select", columnIds: ["kept"] });
  });

  it("unchecks a hidden output in an existing final select", () => {
    const drafts = stepsFromRendered(
      [
        {
          kind: "withColumns",
          columns: [{ outputColumnId: "calculated", formula: "`amount` * 2" }],
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
      [column("amount", "amount"), column("memo", "memo")]
    );
    expect(
      hidePipelineColumn(drafts, "calculated", ["amount", "calculated", "memo"])
    ).toMatchObject([
      { kind: "withColumns" },
      { kind: "select", columnIds: ["amount", "memo"] },
    ]);
  });

  it("will not hide an absent column or the frame's last visible column", () => {
    expect(hidePipelineColumn([], "missing", ["amount"])).toBeNull();
    expect(hidePipelineColumn([], "amount", ["amount"])).toBeNull();
  });
});
