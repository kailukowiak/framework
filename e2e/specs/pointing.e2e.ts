import { $ } from "@wdio/globals";
import {
  blockDraft,
  focusBlockSource,
  pointAtCell,
  resetAndOpenTutorial,
  waitForGutterAnswer,
} from "../lib/helpers";

// Formula-by-pointing, both halves of the positional-identity rule from
// AGENTS.md. The formula-clicks Start workbook is the fixture on purpose:
// its Monthly sales frame is document-owned with no pipeline and no display
// ordering, which is exactly the one situation where a clicked cell has a
// stable address — and applying a display sort through the grid's own sort
// button is what takes that eligibility away, on the same frame, in the
// same session. A spec that only tested the happy half would be asserting
// the feature while ignoring the rule that shapes it.
describe("formula by pointing", () => {
  it("opens the Start workbook", async () => {
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();
  });

  it("inserts a cell reference and evaluates it live", async () => {
    // The Scratchwork block by name: the workbook also ships a "Checks"
    // block, and a bare .block-source answers whichever renders first.
    const source = $('textarea[aria-label="Scratchwork lines"]');
    await source.waitForExist();
    await source.setValue("April = ");
    await focusBlockSource("Scratchwork");

    await pointAtCell("142,000");

    // The inserted token's spelling belongs to the engine; the spec asserts
    // the outcome — the draft grew past what was typed and the real engine
    // evaluated the pointed-at cell.
    await waitForGutterAnswer("142000");
    expect(await blockDraft("Scratchwork")).not.toBe("April = ");
  });

  it("refuses the same cell once a display sort makes rows positional", async () => {
    await $('[aria-label="Sort by Month"]').click();
    // The sort landed when January leads the Month column.
    await $("div.cell-display*=2026-01").waitForExist();

    const source = $('textarea[aria-label="Scratchwork lines"]');
    await source.setValue("Later = ");
    await focusBlockSource("Scratchwork");
    await pointAtCell("142,000");

    const notice = $(".notice-toast");
    await notice.waitForExist();
    await expect(notice).toHaveText(
      expect.stringContaining("stable row address")
    );
    expect(await blockDraft("Scratchwork")).toBe("Later = ");
  });
});
