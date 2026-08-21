import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { openTutorialsAndExamples, waitForGutterAnswer } from "../lib/helpers";

// The month-over-month tutorial, driven the way a learner drives it. The
// tutorial workbooks are the agreed e2e fixtures: each run resets them
// through the Data library's own two-click confirm, so this spec also keeps
// the reset path honest. Revenue in the Start workbook sums to 839000; after
// editing one revenue cell upward by a thousand the live check line must say
// 840000, and ⌘Z must take it back. That one chain covers document load,
// grid rendering, cross-object formula authoring, live recompute after a
// cell edit, and undo — the exact seams unit tests cannot reach.
//
// Gesture notes, learned the hard way: cell editing goes click + F2 (the
// cell's own advertised alternative) because the embedded server's
// double-click never produces a React-visible dblclick; and the readiness
// signal for the opened workbook is a rendered cell, because column headers
// are not inputs at rest.
describe("formula-clicks tutorial", () => {
  it("resets and opens the Start workbook from the Data library", async () => {
    const dialog = $(".dataset-dialog");
    await dialog.waitForExist();
    await openTutorialsAndExamples();

    // Create is idempotent — it only writes workbooks that are missing —
    // and it is what makes Reset appear on a machine that never had them.
    await $("button*=Create tutorials").click();
    await $("button*=Reset tutorials").waitForClickable();
    await $("button*=Reset tutorials").click();
    await $("button*=Replace all tutorial workbooks").click();

    const start = $("button*=Month-over-month formulas by pointing — Start");
    await start.waitForClickable();
    await start.click();

    // The workbook is open when its data is on screen: April's revenue,
    // rendered with the grid's own formatting.
    await $("div.cell-display*=142,000").waitForExist();
  });

  it("computes a cross-object total typed into the Checks block", async () => {
    const source = $(".block-source");
    await source.waitForExist();
    await source.click();
    await browser.keys("Total revenue = `Monthly sales`.`Revenue`.sum()");

    await waitForGutterAnswer("839000");
  });

  it("moves the total live when a revenue cell is edited, and back on undo", async () => {
    // Leave the Checks editor first. While a formula editor is active, a
    // click on a frame cell is formula-by-pointing — it inserts a reference
    // into the draft, which is the tutorial's whole subject — so the click
    // below would never select the cell. Escape is how a person ends
    // pointing mode before going back to the grid.
    // A real mouse click on the cell would blur the Checks editor as the
    // browser's default focus-transfer action — but the embedded driver
    // dispatches synthetic events, which never run default actions, so the
    // editor (and its formula-pointing mode, which swallows frame clicks)
    // stays active no matter where we click. This blur() stands in for the
    // one native behavior the driver cannot produce; everything after it is
    // ordinary gestures.
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());

    const cell = $("div.cell-display*=142,000");
    await cell.waitForExist();
    await cell.click();
    await browser.keys(Key.F2);

    const editor = $(".cell-editor");
    await editor.waitForExist();
    await editor.setValue("143000");
    await browser.keys(Key.Enter);

    await waitForGutterAnswer("840000");

    await browser.keys([Key.Command, "z"]);
    await waitForGutterAnswer("839000");
  });

  it("promotes a date sequence typed in a cell to a live column fill", async () => {
    const cell = $("div.cell-display*=142,000");
    await cell.click();
    await browser.keys(Key.F2);
    const editor = $(".cell-editor");
    await editor.waitForExist();
    await editor.setValue(
      "=sequence(2026-01-31, periods=frame.len(), step=1mo)"
    );
    await browser.keys(Key.Enter);

    await $("div.cell-display*=2026-06-30").waitForExist({
      timeoutMsg: "the typed calendar-month sequence never filled the Revenue column",
    });
  });
});
