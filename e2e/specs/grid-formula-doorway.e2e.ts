import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import {
  digits,
  frameHeaderNames,
  openContextMenuOn,
  pasteFrameOntoScratchCanvas,
  pointAtColumnCell,
  recordCellTexts,
  savedFrameSteps,
} from "../lib/helpers";

// `=` in a cell is a doorway, not a place to type (GridFormulaKeyboard.ts):
// it opens the column's own line in Wrangle as an unsaved draft,
// `` `Units` = `Units` `` with only the expression selected. Return decides
// what the line was (useColumnTransformRequest.ts): reference-free
// arithmetic typed from one hand-entered cell folds into that cell's value —
// a `setCell`, no step — and anything else is saved as the column's
// formula. GridEqualsDoorway.test.tsx proves what each case emits; this spec
// proves the emitted operation reaches the engine, comes back rendered, and
// is one undo, and that a calculated column leaves the frame's literal
// columns as editable as they were (`owns_its_columns`, which admits a
// chain of Filter / WithColumns / Sort / Comment steps).

/** The record cells of a column, as numbers (the grid formats what it draws). */
const numbersIn = async (column: string) =>
  (await recordCellTexts(column)).map((text) => Number(digits(text)));

/**
 * The top formula bar while the doorway's line is the active formula
 * session. Its accessible name is `Edit <session label>` — the column name
 * alone — where a merely selected cell names itself `Edit Units · row 1`
 * (ScratchworkFormulaBar.tsx, `formulaBarCell.ts`), so this label is how a
 * person, and the spec, can tell the doorway has opened.
 */
const unitsLine = () => $('textarea[aria-label="Edit Units"]');
const SEEDED = "`Units` = `Units`";

async function openDoorwayFrom(row: number): Promise<void> {
  await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
  await pointAtColumnCell("Units", row);
  await browser.keys("=");
  const line = unitsLine();
  await line.waitForExist({ timeoutMsg: "= in a Units cell did not open the column's line" });
  await browser.waitUntil(async () => (await line.getValue()) === SEEDED, {
    timeoutMsg: `the doorway did not seed ${SEEDED}: ${await line.getValue()}`,
  });
}

/**
 * Replaces the whole draft line and commits it with Return. The full line
 * goes through `setValue`, as wrangle-calculated-column.e2e.ts does, rather
 * than typing over the selected expression: whether the driver's typing
 * replaces a textarea selection is not something this spec wants to depend
 * on, and the line it leaves is the same either way.
 */
async function commitLine(line: string): Promise<void> {
  const editor = unitsLine();
  await editor.setValue(line);
  await editor.click();
  await browser.keys(Key.Enter);
}

describe("= in a grid cell", () => {
  it("makes a frame by pasting onto the bare canvas", async () => {
    await pasteFrameOntoScratchCanvas(
      "Item\tUnits\tPrice\nA\t10\t2\nB\t20\t3\nC\t30\t4",
      "Item"
    );
    await browser.waitUntil(async () => (await numbersIn("Units")).join() === "10,20,30", {
      timeoutMsg: "the pasted frame never showed its rows",
    });
  });

  it("folds arithmetic typed after = into that one cell, writes no step, and undoes", async () => {
    await openDoorwayFrom(0);
    // Opening the doorway says nothing yet, so it saves nothing yet: the
    // line is a draft until Return.
    expect(await savedFrameSteps()).toHaveLength(0);

    await commitLine("`Units` = 1200/12");
    await browser.waitUntil(async () => (await numbersIn("Units")).join() === "100,20,30", {
      timeoutMsg: `=1200/12 did not land as the cell's value: ${(await recordCellTexts("Units")).join()}`,
    });
    expect(await savedFrameSteps()).toHaveLength(0);
    // Still a literal cell: the value is typed data, not a formula.
    expect(
      await browser.execute(() =>
        Array.from(document.querySelectorAll(".frame-object button.computed-cell")).length
      )
    ).toBe(0);

    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => (await numbersIn("Units")).join() === "10,20,30", {
      timeoutMsg: "undo did not restore the cell's old value",
    });
  });

  it("commits a formula that reads another column as the column's calculation, keeping its name", async () => {
    await openDoorwayFrom(1);
    await commitLine("`Units` = `Price` * 2");
    await browser.waitUntil(async () => (await numbersIn("Units")).join() === "4,6,8", {
      timeoutMsg: `Units did not become Price * 2: ${(await recordCellTexts("Units")).join()}`,
    });
    // The header is the column the person typed into, under its own name —
    // not a new "Column 1" beside it.
    expect(await frameHeaderNames()).toEqual(["Item", "Units", "Price"]);
    const steps = await savedFrameSteps();
    expect(steps.map((step) => step.kind)).toEqual(["withColumns"]);
  });

  // Column operations after a calculated column. The chain now holds a
  // withColumns step, which only adds a calculation to rows the document
  // owns, so the literal columns are still the person's to add and remove.
  it("still inserts and deletes a literal column beside the calculation", async () => {
    await browser.keys(Key.Escape);
    await openContextMenuOn('[aria-label="Sort by Item"]');
    await $(".framework-context-menu").$("button=Insert column right").click();
    await browser.waitUntil(
      async () => (await frameHeaderNames()).join() === "Item,New column,Units,Price",
      {
        timeoutMsg: `Insert column right did not land beside Item: ${(await frameHeaderNames()).join()}`,
      }
    );

    await openContextMenuOn('[aria-label="Sort by New column"]');
    await $(".framework-context-menu").$("button=Delete column").click();
    await browser.waitUntil(
      async () => (await frameHeaderNames()).join() === "Item,Units,Price",
      { timeoutMsg: "Delete column did not remove the literal column" }
    );
    // And the calculation went on computing through both edits.
    expect((await numbersIn("Units")).join()).toBe("4,6,8");
  });
});
