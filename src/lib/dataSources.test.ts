import { describe, expect, it } from "vitest";
import { dataSourceKind, groupDataSources } from "./dataSources";
import type { ComputedFrame, DocumentView, FrameObject } from "./types";

function frame(name: string, fields: Partial<FrameObject> = {}): FrameObject {
  return {
    kind: "frame",
    id: name.toLowerCase(),
    name,
    columns: [],
    rows: [],
    derivation: null,
    uniqueKeys: [],
    summaries: [],
    ...fields,
  };
}

function computed(fields: Partial<ComputedFrame> = {}): ComputedFrame {
  return {
    fingerprint: "test",
  formulas: {},
    overrideFormulas: {},
    rows: {},
    summaries: {},
    derivation: null,
    editing: { cells: true, rows: true, overrides: true },
    ...fields,
  };
}

function documentOf(
  frames: FrameObject[],
  computedFrames: Record<string, ComputedFrame> = {}
): DocumentView {
  return {
    id: "doc",
    name: "Doc",
    revision: 1,
    objects: frames,
    views: [],
    computedFrames,
    computedResults: {},
    computedBlocks: {},
    computedTexts: {},
    formulaFunctions: [],
    canUndo: false,
    canRedo: false,
  };
}

describe("dataSourceKind", () => {
  // Typed-in data and a copied-in file differ in history, not in behaviour:
  // both are the document's own and neither moves until somebody edits it.
  // One cell, and the detail line is what still tells them apart.
  it("puts data the document owns in one place, however it got there", () => {
    expect(dataSourceKind(frame("Typed"))).toBe("base-static");
    expect(dataSourceKind(frame("Loaded", { sourceFile: "/tmp/l.csv" }))).toBe(
      "base-static"
    );
  });

  it("calls a frame with a file to re-read refreshable", () => {
    expect(
      dataSourceKind(
        frame("Linked", { connector: { kind: "file", sourcePath: "/tmp/l.csv" } })
      )
    ).toBe("base-refreshable");
  });

  // A derived frame can carry an artifact of its own — that is what caching
  // is — so the derivation has to win, or every cached frame would file
  // itself as base data.
  it("calls a cached derived frame derived", () => {
    const cached = frame("Grouped", {
      derivation: { sourceFrameId: "ledger" } as FrameObject["derivation"],
      artifact: {
        id: "a",
        path: "/tmp/a.parquet",
        rowCount: 2,
        format: "parquet",
        sourceName: "Grouped",
      },
    });
    expect(dataSourceKind(cached)).toBe("derived-static");
  });

  // The second axis is inherited, and only the computed side knows it: a
  // frame three steps downstream of a file moves when the file does, though
  // nothing in its own definition mentions a file.
  it("splits derived frames by whether the ground under them moves", () => {
    const derived = frame("Grouped", {
      derivation: { sourceFrameId: "ledger" } as FrameObject["derivation"],
    });
    expect(dataSourceKind(derived, { live: true } as never)).toBe(
      "derived-refreshable"
    );
    expect(dataSourceKind(derived, { live: false } as never)).toBe("derived-static");
  });
});

describe("groupDataSources", () => {
  it("names a derived frame after what it reads", () => {
    const ledger = frame("Ledger", { sourceFile: "/tmp/ledger.csv" });
    const grouped = frame("By period", {
      derivation: { sourceFrameId: ledger.id } as FrameObject["derivation"],
    });
    const groups = groupDataSources(documentOf([ledger, grouped]));
    expect(groups.map((group) => group.kind)).toEqual(["base-static", "derived-static"]);
    expect(groups[1].entries[0].detail).toBe("from Ledger");
  });

  it("shows the file for an import and its full path for the tooltip", () => {
    const ledger = frame("Ledger", {
      connector: { kind: "file", sourcePath: "/data/2026/ledger.csv" },
    });
    const groups = groupDataSources(
      documentOf([ledger], {
        ledger: computed({
          sourceName: "ledger.csv",
          live: true,
          editing: {
            cells: false,
            rows: false,
            overrides: false,
            reason: "It is read from a file, and refreshing replaces it.",
          },
        }),
      })
    );
    const entry = groups[0].entries[0];
    expect(entry.detail).toBe("ledger.csv");
    expect(entry.title).toBe("/data/2026/ledger.csv");
    expect(entry.editable).toBe(false);
    expect(entry.live).toBe(true);
  });

  // Liveness is inherited, so a derived frame can be live while nothing in
  // its own definition reads a file. That is the case the list has to say
  // out loud: its group heading says "Derived" and nothing more.
  it("carries the inherited liveness of a derived frame", () => {
    const grouped = frame("By period", {
      derivation: { sourceFrameId: "ledger" } as FrameObject["derivation"],
    });
    const groups = groupDataSources(
      documentOf([grouped], { "by period": computed({ live: true }) })
    );
    expect(groups[0].entries[0].live).toBe(true);
  });

  it("counts the rows of a frame that has them to count", () => {
    const typed = frame("Assumptions", {
      rows: [
        { id: "r1", cells: {} },
        { id: "r2", cells: {} },
      ],
    });
    const groups = groupDataSources(
      documentOf([typed], { assumptions: computed({ totalRows: 2 }) })
    );
    expect(groups[0].entries[0].detail).toBe("2 rows");
    expect(groups[0].entries[0].editable).toBe(true);
  });

  // Before the first computation there is no answer, and "you may type
  // here" is the wrong thing to guess: offering an edit that is then
  // refused is worse than showing it a moment late.
  it("treats a frame with no metadata yet as not editable", () => {
    const groups = groupDataSources(documentOf([frame("Assumptions")]));
    expect(groups[0].entries[0].editable).toBe(false);
  });

  // Asking a derived frame for a row count means running it. The list says
  // so rather than paying for a subtitle.
  it("does not invent a row count for a frame nobody has read", () => {
    const grouped = frame("By period", {
      derivation: { sourceFrameId: "missing" } as FrameObject["derivation"],
    });
    const groups = groupDataSources(documentOf([grouped]));
    expect(groups[0].entries[0].detail).toBe("from a frame that is gone");
  });

  it("reports staleness so the list can say which numbers are old", () => {
    const grouped = frame("By period", {
      derivation: { sourceFrameId: "ledger" } as FrameObject["derivation"],
    });
    const groups = groupDataSources(
      documentOf([grouped], {
        "by period": computed({
          materialization: { rowCount: 4, stale: true },
          upstreamStale: true,
        }),
      })
    );
    expect(groups[0].entries[0]).toMatchObject({
      cached: true,
      stale: true,
      upstreamStale: true,
    });
  });

  it("leaves out a group with nothing in it", () => {
    const groups = groupDataSources(documentOf([frame("Assumptions")]));
    expect(groups).toHaveLength(1);
    expect(groups[0].title).toBe("Base · Static");
  });
});
