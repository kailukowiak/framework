import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import {
  digits,
  extendSelectionDown,
  focusColumnCell,
  frameHeaderNames,
  openContextMenuOn,
  openContextMenuOnColumnCell,
  pasteFrameOntoScratchCanvas,
  pointAtColumnCell,
  pressGridKey,
  recordCellTexts,
} from "../lib/helpers";

// A spreadsheet has no edge: Return at the bottom of a column and Tab at the
// end of a row carry on into space that becomes data the moment it is typed
// into. FrameWork's grid is a bounded table with a new-row line and a
// new-column strip hung off it, so that continuity is built by hand
// (src/lib/gridEdgeGrowth.ts), and it crosses every seam there is: the
// window keydown dispatcher in App.tsx decides where the key goes, the
// ghost inputs turn typing into `addRow` / `addColumn`, the engine writes
// the row or column, and history has to take each gesture back in one step.
//
// The fixture is a hand-entered frame pasted onto the scratch canvas. The
// tests share it deliberately, in order, so each one starts from what the
// last one left; each says what that is.

/** The element holding the keyboard, by its accessible name. */
const activeLabel = () =>
  browser.execute(() => document.activeElement?.getAttribute("aria-label") ?? null);

/**
 * A column's record cells as numbers, blank as null. Compared as numbers
 * because the grid formats what it draws ("40" may render "40.00" in a
 * Number column, a blank integer renders "—"), and these specs assert the
 * value, not the formatter.
 */
const numbersIn = async (column: string) =>
  (await recordCellTexts(column)).map((text) =>
    digits(text) === "" ? null : Number(digits(text))
  );

const waitForUnits = (
  expected: (units: Array<number | null>) => boolean,
  timeoutMsg: string
) =>
  browser.waitUntil(async () => expected(await numbersIn("Units")), { timeoutMsg });

// Units is the last column, so the edge tests below act on it.
describe("growing a hand-entered grid at its edges", () => {
  it("makes a frame by pasting onto the bare canvas", async () => {
    await pasteFrameOntoScratchCanvas("Item\tUnits\nA\t10\nB\t20\nC\t30", "Item");
    await waitForUnits(
      (units) => units.join() === "10,20,30",
      "the pasted frame never showed its three rows"
    );
  });

  // Return on the last row moves the cursor into the new-row line, in the
  // same column; nothing is written until something is typed there.
  it("carries Return off the last row into the new-row line and adds the typed row", async () => {
    await pointAtColumnCell("Units", 2);
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await activeLabel()) === "New row Units", {
      timeoutMsg: `Return on the last row did not reach the new-row line (focus is on ${await activeLabel()})`,
    });
    expect(await recordCellTexts("Units")).toHaveLength(3);

    await browser.keys("40");
    await browser.keys(Key.Enter);
    await waitForUnits(
      (units) => units.join() === "10,20,30,40",
      "typing a value and Return in the new-row line did not add a row"
    );
  });

  // Tab on the very last cell is the Excel table gesture, and it does write:
  // a blank row arrives and the cursor lands on its first cell.
  it("adds a blank row when Tab leaves the last cell", async () => {
    await pointAtColumnCell("Units", 3);
    await browser.keys(Key.Tab);
    await waitForUnits(
      (units) => units.length === 5 && units[4] === null,
      "Tab on the last cell did not append a blank row"
    );
    await browser.waitUntil(
      () =>
        browser.execute(() => {
          const focused = document.querySelector("td.cell-focus");
          const rows = document.querySelectorAll(".frame-object tbody tr[data-row-id]");
          const firstColumn = document
            .querySelector('button[aria-label="Sort by Item"]')
            ?.closest("th")
            ?.getAttribute("data-column-id");
          return (
            focused?.closest("tr") === rows[rows.length - 1] &&
            focused?.getAttribute("data-column-id") === firstColumn
          );
        }),
      { timeoutMsg: "the cursor did not land on the new row's first cell" }
    );
  });

  // The column that does not exist yet: Right-arrow off the last column steps
  // onto it, and a value typed there arrives as a new column with this row
  // filled — one `addColumn` carrying the value, so one undo takes both.
  it("grows a column from a value typed past the last column, and one undo removes it", async () => {
    const before = await recordCellTexts("Units");
    await pointAtColumnCell("Units", 0);
    await browser.keys(Key.ArrowRight);
    await browser.waitUntil(async () => (await activeLabel()) === "New column, row 1", {
      timeoutMsg: `Right-arrow off the last column did not reach the new-column strip (focus is on ${await activeLabel()})`,
    });

    await browser.keys("42");
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await frameHeaderNames()).includes("New column"), {
      timeoutMsg: "typing into the new-column strip did not add a column",
    });
    expect((await numbersIn("New column"))[0]).toBe(42);

    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(
      async () =>
        !(await frameHeaderNames()).includes("New column") &&
        JSON.stringify(await recordCellTexts("Units")) === JSON.stringify(before),
      { timeoutMsg: "one undo did not take back both the new column and its value" }
    );
  });

  // ⇧Space selects the row; ⌥⌘= inserts above it (an `addRow` with
  // `beforeRowId`), and ⌥⌘− deletes the selected rows. The chords are
  // matched on `event.code`, which is why they are sent with one.
  it("inserts a row above the selected row and deletes it again from the keyboard", async () => {
    const before = await recordCellTexts("Units");
    await pointAtColumnCell("Units", 1);
    await pressGridKey({ key: " ", code: "Space", shiftKey: true });
    await pressGridKey({ key: "≠", code: "Equal", metaKey: true, altKey: true });
    await waitForUnits(
      (units) =>
        units.length === before.length + 1 &&
        units[0] === 10 &&
        units[1] === null &&
        units[2] === 20,
      "⌥⌘= did not put a blank row above the selected one"
    );

    // The inserted row, selected as a whole row, deleted by the chord.
    await pointAtColumnCell("Units", 1);
    await pressGridKey({ key: " ", code: "Space", shiftKey: true });
    await pressGridKey({ key: "–", code: "Minus", metaKey: true, altKey: true });
    await browser.waitUntil(
      async () => JSON.stringify(await recordCellTexts("Units")) === JSON.stringify(before),
      { timeoutMsg: "⌥⌘− did not delete the selected row" }
    );
  });

  // Two rows deleted as one `deleteRows`, so one undo restores both, each in
  // the position it had. A cell range across two rows is enough for the
  // menu: "Delete row" acts on every row the selection spans
  // (`selectedRowIds`, src/lib/gridStructure.ts).
  it("deletes two selected rows from the menu and one undo puts both back in place", async () => {
    const before = await recordCellTexts("Item");
    const beforeUnits = await recordCellTexts("Units");
    await pointAtColumnCell("Units", 0);
    await extendSelectionDown();
    await openContextMenuOnColumnCell("Units", 1);
    await $(".framework-context-menu").$("button=Delete 2 rows").click();
    await waitForUnits(
      (units) => units.length === beforeUnits.length - 2 && units[0] === 30,
      "Delete 2 rows did not remove the first two rows"
    );

    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(
      async () =>
        JSON.stringify(await recordCellTexts("Item")) === JSON.stringify(before) &&
        JSON.stringify(await recordCellTexts("Units")) === JSON.stringify(beforeUnits),
      { timeoutMsg: "one undo did not restore both rows in their original positions" }
    );
  });

  // "Left" is spelled with `beforeColumnId`, because `afterColumnId` has no
  // way to say "first".
  it("inserts a column at the far left from the first column's menu", async () => {
    await browser.keys(Key.Escape);
    await openContextMenuOn('[aria-label="Sort by Item"]');
    await $(".framework-context-menu").$("button=Insert column left").click();
    await browser.waitUntil(
      async () => (await frameHeaderNames()).join() === "New column,Item,Units",
      {
        timeoutMsg: `Insert column left did not lead the table: ${(await frameHeaderNames()).join()}`,
      }
    );
    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => (await frameHeaderNames()).join() === "Item,Units", {
      timeoutMsg: "undo did not remove the inserted column",
    });
  });

  // History used to stop at ten entries, so the eleventh undo did nothing and
  // the value someone started from could not be reached again.
  it("undoes more than ten single-cell edits back to the first value", async () => {
    const original = (await numbersIn("Units"))[0];
    const edits = Array.from({ length: 12 }, (_, index) => 101 + index);
    for (const value of edits) {
      await focusColumnCell("Units", 0);
      await browser.keys(Key.F2);
      const editor = $(".cell-editor");
      await editor.waitForExist();
      await editor.setValue(String(value));
      await browser.keys(Key.Enter);
      await browser.waitUntil(async () => (await numbersIn("Units"))[0] === value, {
        timeoutMsg: `the edit to ${value} never landed`,
      });
    }

    // Undo one step at a time, waiting for each to land, so a failure names
    // the depth at which history ran out.
    const expected = [...edits.slice(0, -1).reverse(), original];
    for (const [depth, value] of expected.entries()) {
      await browser.keys([Key.Command, "z"]);
      await browser.waitUntil(async () => (await numbersIn("Units"))[0] === value, {
        timeoutMsg: `undo ${depth + 1} of ${edits.length} did not restore ${value}`,
      });
    }
  });
});
