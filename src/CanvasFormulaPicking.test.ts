// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { canvasFormulaPointerHandler } from "./CanvasFormulaPicking";
import type { ActiveFormulaEditor } from "./lib/activeFormulaEditor";
import type { DocumentView } from "./lib/types";

const active: ActiveFormulaEditor = {
  id: "scratchwork:checks",
  label: "Checks",
  kind: "scratchwork",
  draft: "",
  selection: { start: 0, end: 0 },
  focused: true,
  canCommit: true,
  completion: {
    references: [
      {
        id: "revenue",
        frameId: "sales",
        label: "Sales.Revenue",
        token: "`Sales`.`Revenue`",
        kind: "column",
        detail: "Integer column",
      },
    ],
  },
};

const documentView = {
  objects: [
    {
      id: "sales",
      kind: "frame",
      derivation: null,
      steps: [],
      display: { steps: [] },
      columns: [{ id: "revenue", name: "Revenue" }],
    },
  ],
  computedFrames: { sales: { live: false, editing: { rows: true } } },
} as unknown as DocumentView;

afterEach(() => {
  document.body.replaceChildren();
});

function options() {
  return {
    document: documentView,
    getActive: () => active,
    insertReference: vi.fn(),
    clear: vi.fn(),
    disengage: vi.fn(),
    onNotice: vi.fn(),
    onRecurrence: vi.fn(),
  };
}

function pointerDown(target: HTMLElement) {
  return {
    target,
    button: 0,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as never;
}

describe("canvas formula pointing", () => {
  it("inserts a whole column from its semantic header button", () => {
    document.body.innerHTML = `
      <div data-frame-id="sales">
        <table><thead><tr><th data-column-id="revenue">
          <button class="column-select"><span>Revenue</span></button>
        </th></tr></thead></table>
      </div>`;
    const target = document.querySelector("span")!;
    const configured = options();
    canvasFormulaPointerHandler(configured)(pointerDown(target));

    expect(configured.insertReference).toHaveBeenCalledWith(
      "`Sales`.`Revenue`",
      true
    );
  });

  it("explains a same-column drag instead of inserting its first cell", () => {
    document.body.innerHTML = `
      <div data-frame-id="sales">
        <table><tbody>
          <tr data-row-index="0"><td data-column-id="revenue">1</td></tr>
          <tr data-row-index="1"><td data-column-id="revenue">2</td></tr>
          <tr data-row-index="2"><td data-column-id="revenue">3</td></tr>
        </tbody></table>
      </div>`;
    const cells = [...document.querySelectorAll<HTMLElement>("td")];
    const configured = options();
    canvasFormulaPointerHandler(configured)(pointerDown(cells[0]));
    cells[2].dispatchEvent(new MouseEvent("pointerup", { bubbles: true }));

    expect(configured.insertReference).not.toHaveBeenCalled();
    expect(configured.onNotice).toHaveBeenCalledWith(expect.stringContaining("Wrangle"));
    expect(document.querySelector(".formula-pick-range-preview")).toBeNull();
  });

  it("still inserts one stable cell when the pointer is released in place", () => {
    document.body.innerHTML = `
      <div data-frame-id="sales"><table><tbody>
        <tr data-row-index="1"><td data-column-id="revenue">2</td></tr>
      </tbody></table></div>`;
    const cell = document.querySelector<HTMLElement>("td")!;
    const configured = options();
    canvasFormulaPointerHandler(configured)(pointerDown(cell));
    cell.dispatchEvent(new MouseEvent("pointerup", { bubbles: true }));

    expect(configured.insertReference).toHaveBeenCalledWith(
      "`Sales`.`Revenue`.head(2).last()",
      true
    );
  });
});
