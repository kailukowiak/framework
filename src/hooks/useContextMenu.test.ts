import { describe, expect, it } from "vitest";
import type { GridFocus, RenderedGrid } from "../FrameGrid";
import { fixtures, objectNamed } from "../test/support";
import { gridFocusAfterContextClick } from "./useContextMenu";

const document = fixtures.salesWithMargin;
const frame = objectNamed(document, "frame", "Monthly sales");
const view = document.views.find((candidate) => candidate.objectId === frame.id)!;
const rendered = new Map<string, RenderedGrid>([
  [frame.id, { rows: frame.rows, offset: 0, totalRows: frame.rows.length }],
]);

function focus(row: number, column: number): GridFocus {
  return {
    viewId: view.id,
    objectId: frame.id,
    rowId: frame.rows[row].id,
    columnId: frame.columns[column].id,
    mode: "navigate",
    editSeed: null,
    anchor: null,
    span: null,
  };
}

describe("gridFocusAfterContextClick", () => {
  it("focuses the cell right-clicked outside the current selection", () => {
    const current = focus(0, 0);
    const next = gridFocusAfterContextClick(document, current, rendered, {
      frameId: frame.id,
      viewId: view.id,
      rowId: frame.rows[1].id,
      columnId: frame.columns[1].id,
    });

    expect(next).toMatchObject({
      rowId: frame.rows[1].id,
      columnId: frame.columns[1].id,
      anchor: null,
      span: null,
    });
  });

  it("preserves a range when the right-click lands inside it", () => {
    const current = {
      ...focus(1, 1),
      anchor: {
        rowId: frame.rows[0].id,
        columnId: frame.columns[0].id,
      },
    };

    expect(
      gridFocusAfterContextClick(document, current, rendered, {
        frameId: frame.id,
        viewId: view.id,
        rowId: frame.rows[0].id,
        columnId: frame.columns[1].id,
      })
    ).toBe(current);
  });
});
