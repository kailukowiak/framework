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

function pointerDown(target: HTMLElement, button = 0) {
  return {
    target,
    button,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as never;
}

/** The click a real press is followed by, which the grid also listens for. */
function clickAfterPress(target: HTMLElement): boolean {
  return target.dispatchEvent(
    new MouseEvent("click", { bubbles: true, cancelable: true })
  );
}

/** A wrangle-style session scoped to the sales frame, with no cell anchor. */
const formulaSession: ActiveFormulaEditor = {
  ...active,
  id: "pipeline:sales:step",
  label: "Amount",
  kind: "formula",
  completion: { ...active.completion, frameId: "sales" },
};

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

    expect(configured.insertReference).toHaveBeenCalledWith("`Sales`.`Revenue`");
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
      "`Sales`.`Revenue`.head(2).last()"
    );
  });

  it("lets a click on another frame's cell end the session and pass through", () => {
    document.body.innerHTML = `
      <div data-frame-id="ledger"><table><tbody>
        <tr data-row-index="0"><td data-column-id="je">JE-1</td></tr>
      </tbody></table></div>`;
    const cell = document.querySelector<HTMLElement>("td")!;
    const configured = { ...options(), getActive: () => formulaSession };
    const event = pointerDown(cell);
    canvasFormulaPointerHandler(configured)(event);

    expect(configured.insertReference).not.toHaveBeenCalled();
    expect(configured.clear).toHaveBeenCalled();
    expect((event as { preventDefault: ReturnType<typeof vi.fn> }).preventDefault)
      .not.toHaveBeenCalled();
  });

  it("still explains a same-frame column the session cannot read", () => {
    document.body.innerHTML = `
      <div data-frame-id="sales"><table><tbody>
        <tr data-row-index="0"><td data-column-id="later-output">7</td></tr>
      </tbody></table></div>`;
    const cell = document.querySelector<HTMLElement>("td")!;
    const configured = { ...options(), getActive: () => formulaSession };
    canvasFormulaPointerHandler(configured)(pointerDown(cell));

    expect(configured.clear).not.toHaveBeenCalled();
    expect(configured.onNotice).toHaveBeenCalledWith(
      expect.stringContaining("Amount")
    );
  });

  // A cell renders its own onClick — that is how an ordinary click selects
  // it — and a cancelled pointerdown does not suppress the click that
  // follows. Left alone it moved the keyboard into the grid right after the
  // reference landed in the formula.
  it("keeps the click that follows a pick from also selecting the cell", () => {
    document.body.innerHTML = `
      <div data-frame-id="sales">
        <table><thead><tr><th data-column-id="revenue">
          <button class="column-select"><span>Revenue</span></button>
        </th></tr></thead></table>
      </div>`;
    const target = document.querySelector("span")!;
    canvasFormulaPointerHandler(options())(pointerDown(target));

    expect(clickAfterPress(target)).toBe(false);
    // And only that one click: the next gesture is the person's again.
    expect(clickAfterPress(target)).toBe(true);
  });

  it("explains a column added in the same step instead of selecting it", () => {
    document.body.innerHTML = `
      <div data-frame-id="sales"><table><tbody>
        <tr data-row-index="0"><td data-column-id="previous">118000</td></tr>
      </tbody></table></div>`;
    const cell = document.querySelector<HTMLElement>("td")!;
    const writingChange: ActiveFormulaEditor = {
      ...formulaSession,
      label: "Change",
      completion: {
        references: active.completion.references,
        // A source frame that has grown a chain publishes no completion
        // frame id; the anchor is what says which grid is its own.
        anchorFrameId: "sales",
        targetColumnId: "change",
        scope: {
          steps: [
            {
              kind: "withColumns",
              columns: [
                {
                  outputColumnId: "previous",
                  name: "Previous revenue",
                  formula: "`Revenue`.shift(1)",
                },
                { outputColumnId: "change", name: "Change", formula: "" },
              ],
            },
          ],
          stepIndex: 0,
        },
      },
    };
    const configured = { ...options(), getActive: () => writingChange };
    canvasFormulaPointerHandler(configured)(pointerDown(cell));

    expect(configured.clear).not.toHaveBeenCalled();
    expect(configured.insertReference).not.toHaveBeenCalled();
    expect(configured.onNotice).toHaveBeenCalledWith(
      "Previous revenue is added in this same step, so Change cannot read it yet. Add Change as a new step to read it."
    );
  });

  it("ends the session when a primary click lands on nothing pickable", () => {
    document.body.innerHTML = `<div class="canvas-viewport"></div>`;
    const canvas = document.querySelector<HTMLElement>("div")!;
    const configured = options();
    canvasFormulaPointerHandler(configured)(pointerDown(canvas));

    expect(configured.clear).toHaveBeenCalled();
    expect(configured.disengage).not.toHaveBeenCalled();
  });

  it("keeps the session for clicks inside its editing surfaces", () => {
    document.body.innerHTML = `<aside class="inspector"><button>step</button></aside>`;
    const button = document.querySelector<HTMLElement>("button")!;
    const configured = options();
    canvasFormulaPointerHandler(configured)(pointerDown(button));

    expect(configured.clear).not.toHaveBeenCalled();
    expect(configured.disengage).not.toHaveBeenCalled();
  });

  it("only disarms pick mode for a non-primary press", () => {
    document.body.innerHTML = `<div class="canvas-viewport"></div>`;
    const canvas = document.querySelector<HTMLElement>("div")!;
    const configured = options();
    canvasFormulaPointerHandler(configured)(pointerDown(canvas, 2));

    expect(configured.clear).not.toHaveBeenCalled();
    expect(configured.disengage).toHaveBeenCalled();
  });
});
