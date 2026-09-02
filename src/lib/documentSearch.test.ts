import { describe, expect, it } from "vitest";
import {
  MAX_FIND_HITS,
  groupFindHits,
  searchDocument,
  type FindHit,
} from "./documentSearch";
import type { DocumentView } from "./types";

const cell = (display: string) => ({ display, value: null, error: null });

/**
 * Two frames and a Scratchwork block, each holding the word "north" in a
 * different place, so one query exercises every kind of hit at once.
 */
const view = (): DocumentView =>
  ({
    objects: [
      {
        kind: "frame",
        id: "sales",
        name: "Northern Sales",
        columns: [
          { id: "region", name: "Region" },
          { id: "margin", name: "Margin" },
        ],
        rows: [{ id: "row-1" }, { id: "row-2" }],
      },
      {
        kind: "block",
        id: "block",
        name: "Notes",
        lines: [{ id: "line-1", name: "north_total", source: "sum(Margin)" }],
      },
      { kind: "value", id: "rate", name: "Northbound rate" },
    ],
    views: [
      { id: "view-sales", objectId: "sales" },
      { id: "view-block", objectId: "block" },
      { id: "view-rate", objectId: "rate" },
    ],
    computedFrames: {
      sales: {
        formulas: { margin: "Region == 'North'" },
        steps: [{ kind: "filter", predicates: ["Region != 'south'"], matchAll: true }],
        rows: {
          "row-1": { region: cell("North"), margin: cell("0.4") },
          "row-2": { region: cell("South"), margin: cell("0.1") },
        },
      },
    },
    computedBlocks: {
      block: { source: "", lines: [{ id: "line-1", name: "north_total", text: "sum(Margin)" }] },
    },
  }) as unknown as DocumentView;

const kinds = (hits: FindHit[]) => hits.map((hit) => hit.kind);

describe("searchDocument", () => {
  it("finds every kind of match, case-insensitively", () => {
    const hits = searchDocument(view(), view(), "NORTH");
    expect(kinds(hits)).toEqual(["object", "formula", "cell", "line", "object"]);

    const [frameName, formula, cellHit, line, valueName] = hits;
    expect(frameName).toMatchObject({ objectId: "sales", label: "Northern Sales", snippet: "Frame" });
    expect(line).toMatchObject({ objectId: "block", label: "north_total", snippet: "sum(Margin)" });
    expect(valueName).toMatchObject({ objectId: "rate", snippet: "Value" });
    expect(formula).toMatchObject({ columnId: "margin", snippet: "Region == 'North'" });
    expect(cellHit).toMatchObject({
      objectId: "sales",
      rowId: "row-1",
      columnId: "region",
      label: "Region",
      snippet: "North",
      viewId: "view-sales",
    });
  });

  it("matches a column name and a chain step's own text", () => {
    expect(kinds(searchDocument(view(), view(), "margin"))).toEqual([
      "column",
      "line",
    ]);
    expect(searchDocument(view(), view(), "south")).toMatchObject([
      { kind: "formula", snippet: "Region != 'south'" },
      { kind: "cell", snippet: "South" },
    ]);
  });

  it("answers an empty query with nothing rather than with everything", () => {
    expect(searchDocument(view(), view(), "   ")).toEqual([]);
  });

  it("leaves a paged frame's rows to the engine", () => {
    const paged = view();
    paged.computedFrames.sales.paged = true;
    expect(kinds(searchDocument(paged, paged, "NORTH"))).toEqual([
      "object",
      "formula",
      "line",
      "object",
    ]);
  });

  it("stops at the cap rather than returning every cell of a large frame", () => {
    const many = view();
    const frame = many.objects[0];
    if (frame.kind !== "frame") throw new Error("fixture shape changed");
    frame.rows = Array.from({ length: 500 }, (_, index) => ({ id: `r${index}` }) as never);
    many.computedFrames.sales.rows = Object.fromEntries(
      frame.rows.map((row) => [row.id, { region: cell("North") }])
    ) as never;
    expect(searchDocument(many, many, "north")).toHaveLength(MAX_FIND_HITS);
  });
});

describe("groupFindHits", () => {
  it("groups under the owning object, in the order the hits arrived", () => {
    const document = view();
    const groups = groupFindHits(document, searchDocument(document, document, "north"));
    expect(groups.map((group) => group.name)).toEqual([
      "Northern Sales",
      "Notes",
      "Northbound rate",
    ]);
    expect(groups[0].hits.map((hit) => hit.kind)).toEqual(["object", "formula", "cell"]);
  });
});
