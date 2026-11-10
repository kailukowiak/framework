import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import {
  openContextMenuOn,
  pasteFrameOntoScratchCanvas,
  pointAtColumnCell,
  recordCellTexts,
} from "../lib/helpers";

// Three gestures a spreadsheet hand reaches for on a table's chrome, each of
// which writes the document and has to come back through the engine: the
// header's value checklist (a Filter step written for you), the column edge
// (a stored width), and the Format tab over a rectangle (one style patch over
// every cell in it). One frame, shared in order.

const regionPicker = () => $('[role="group"][aria-label="Filter Region"]');

/** The picker row for one value: a label holding the checkbox, the value and its count. */
const pickerRow = (value: string) => regionPicker().$(`label*=${value}`);

async function openRegionPicker(): Promise<void> {
  await $('button[aria-label="Filter Region by value"]').click();
  await regionPicker().waitForExist({ timeoutMsg: "the Region filter checklist did not open" });
  await pickerRow("North").waitForExist({ timeoutMsg: "the checklist never listed North" });
}

async function regionTexts(): Promise<string[]> {
  return recordCellTexts("Region");
}

/** Rendered width of a column's header cell, in layout pixels. */
const headerWidth = (column: string) =>
  browser.execute(
    (name: string) =>
      document
        .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
        ?.closest("th")?.offsetWidth ?? -1,
    column
  );

/** Computed font weight of the record cell at (column, row). */
const cellWeight = (column: string, row: number) =>
  browser.execute(
    (name: string, index: number) => {
      const columnId = document
        .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
        ?.closest("th")
        ?.getAttribute("data-column-id");
      const cell = document.querySelectorAll<HTMLElement>(
        `tbody tr[data-row-id] td[data-column-id="${columnId}"]`
      )[index];
      return cell ? Number(getComputedStyle(cell).fontWeight) : -1;
    },
    column,
    row
  );

const blockWeights = async () => [
  await cellWeight("Region", 0),
  await cellWeight("Units", 0),
  await cellWeight("Region", 1),
  await cellWeight("Units", 1),
];

describe("header tools on a hand-entered frame", () => {
  it("makes a frame by pasting onto the bare canvas", async () => {
    await pasteFrameOntoScratchCanvas(
      "Region\tUnits\nEast\t10\nWest\t20\nEast\t30\nNorth\t40\nWest\t50",
      "Region"
    );
    await browser.waitUntil(async () => (await regionTexts()).length === 5, {
      timeoutMsg: "the pasted frame never showed its five rows",
    });
  });

  // The checklist is a way of *writing* the Filter step, not a second
  // filter: Apply rewrites the chain, the Wrangle line shows what was done,
  // and reopening lists every value — read from the chain with the picker's
  // own predicate taken off (ColumnFilterPickerHost.tsx) — so an unticked
  // value can be ticked again.
  it("drops an unticked value's rows, shows the Filter step, and lists it again unticked", async () => {
    await openRegionPicker();
    // Each row reads value then count: East appears twice. Read through
    // execute because the embedded server's getText can answer "" for a
    // container whose text sits in child spans.
    const eastRow = await browser.execute(
      () =>
        Array.from(
          document.querySelectorAll('[aria-label="Filter Region"] label')
        )
          .map((label) => (label.textContent ?? "").replace(/\s+/g, ""))
          .find((text) => text.startsWith("East")) ?? ""
    );
    expect(eastRow).toBe("East2");
    const north = pickerRow("North").$('input[type="checkbox"]');
    expect(await north.isSelected()).toBe(true);
    await north.click();
    await browser.waitUntil(async () => !(await north.isSelected()), {
      timeoutMsg: "clicking North's checkbox did not untick it",
    });
    await regionPicker().$("button=Apply").click();

    await browser.waitUntil(
      async () => (await regionTexts()).join() === "East,West,East,West",
      { timeoutMsg: `Apply did not drop the North row: ${(await regionTexts()).join()}` }
    );
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          Array.from(document.querySelectorAll(".pipeline-command")).some((command) =>
            // The chain shows formulas as the formatter lays them out, and it
            // breaks a method call onto its own line; compare without the
            // layout so the check is about the step, not its line breaks.
            (command.textContent ?? "").replace(/\s+/g, "").includes("`Region`.is_in(")
          )
        ),
      { timeoutMsg: "the Wrangle chain did not show the checklist's Filter step" }
    );

    await openRegionPicker();
    expect(await pickerRow("North").$('input[type="checkbox"]').isSelected()).toBe(false);
    expect(await pickerRow("East").$('input[type="checkbox"]').isSelected()).toBe(true);
  });

  it("brings the rows back when the value is ticked again, and undo restores the filter", async () => {
    const north = pickerRow("North").$('input[type="checkbox"]');
    await north.click();
    await browser.waitUntil(async () => north.isSelected(), {
      timeoutMsg: "clicking North's checkbox did not tick it again",
    });
    await regionPicker().$("button=Apply").click();
    await browser.waitUntil(async () => (await regionTexts()).length === 5, {
      timeoutMsg: "re-ticking North did not bring its row back",
    });

    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(
      async () => (await regionTexts()).join() === "East,West,East,West",
      { timeoutMsg: "undo did not restore the filter that excluded North" }
    );
  });

  // The resize handle is a separator so it can be driven without a pointer:
  // arrows step the width, and the write happens on key up — the same single
  // write as a drag's release (ColumnResizeHandle.tsx).
  it("widens a column from its resize handle's keyboard and keeps the width", async () => {
    const before = await headerWidth("Units");
    await browser.execute(() =>
      document
        .querySelector<HTMLElement>('[role="separator"][aria-label="Resize Units"]')
        ?.focus()
    );
    await browser.keys(Key.ArrowRight);
    await browser.keys(Key.ArrowRight);
    await browser.keys(Key.ArrowRight);
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());

    await browser.waitUntil(async () => (await headerWidth("Units")) > before, {
      timeoutMsg: `the Units column did not grow from ${before}px`,
    });
    // Kept means stored: "Reset column width" is offered only for a column
    // the document holds a width for (ColumnWidthMenuItems.tsx).
    await openContextMenuOn('[aria-label="Sort by Units"]');
    await $(".framework-context-menu").$("button=Reset column width").waitForExist({
      timeoutMsg: "the widened column has no stored width to reset",
    });
    expect(await headerWidth("Units")).toBeGreaterThan(before);

    // The first resize pins every column where it stood (`automaticColumnBasis`),
    // so the table is fixed until *all* of them are let go: resetting Units
    // alone leaves it at the fixed fallback, not at the width its content
    // asked for. "Reset all column widths" is what hands sizing back.
    await $(".framework-context-menu").$("button=Reset all column widths").click();
    await browser.waitUntil(async () => Math.abs((await headerWidth("Units")) - before) <= 1, {
      timeoutMsg: `Reset all column widths did not return Units to ${before}px`,
    });
  });

  // A rectangle takes the style as one patch over every cell in it
  // (`styleSelectionOperation`), so one undo takes all four back.
  it("bolds a selected two-by-two block and one undo un-bolds all four", async () => {
    await browser.keys(Key.Escape);
    await pointAtColumnCell("Region", 0);
    await pointAtColumnCell("Units", 1, { shift: true });
    await browser.keys([Key.Command, "2"]);
    const bold = $('button[aria-label="Bold"]');
    await bold.waitForExist({ timeoutMsg: "the Format tab did not show its Bold control" });
    const normal = await cellWeight("Region", 2);
    expect(normal).toBeLessThan(600);

    await bold.click();
    await browser.waitUntil(async () => (await blockWeights()).every((weight) => weight >= 700), {
      timeoutMsg: `not every cell of the block turned bold: ${(await blockWeights()).join()}`,
    });
    // Nothing outside the rectangle.
    expect(await cellWeight("Region", 2)).toBe(normal);

    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => (await blockWeights()).every((weight) => weight === normal), {
      timeoutMsg: `one undo did not un-bold the whole block: ${(await blockWeights()).join()}`,
    });
  });
});
