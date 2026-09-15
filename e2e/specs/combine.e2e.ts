import { browser, $ } from "@wdio/globals";
import {
  columnCellTexts,
  openContextMenuOn,
  resetAndOpenTutorial,
} from "../lib/helpers";

// Combine is one dialog answering "how do this table's rows relate to that
// table's rows", and two of its four answers append a step to the frame's own
// chain. That path crosses the context menu, the dialog, `setFramePipeline`
// with the existing chain resent, the engine's zip and union, and the
// rendered grid — none of which a mounted component test can reach. The
// tutorial's Expenses (4 rows) against Category fixes (2 rows) is the case
// worth proving: 4 is a whole multiple of 2, so *Repeat to fill* tiles the
// two values twice and the grid must show them repeating.
//
// The embedded driver clicks option elements without moving the native
// selection, so selects are driven by dispatching their own change event —
// the same compensation `read-columns.e2e.ts` documents. Everything after
// that event is the real application.
async function chooseOption(label: string, optionText: string) {
  await browser.execute(
    (name: string, text: string) => {
      const select = document.querySelector<HTMLSelectElement>(
        `select[aria-label="${name}"]`
      );
      const choice = Array.from(select?.options ?? []).find(
        (item) => (item.textContent ?? "").trim() === text
      );
      if (!select || !choice) throw new Error(`Missing option: ${name} / ${text}`);
      select.value = choice.value;
      select.dispatchEvent(new Event("change", { bubbles: true }));
    },
    label,
    optionText
  );
}

async function openCombine() {
  await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
  await openContextMenuOn('[aria-label="Sort by Category"]');
  const item = $("button*=Combine with");
  await item.waitForExist();
  await item.click();
  await $(".combine-dialog").waitForExist();
}

describe("combine two tables", () => {
  it("repeats a short list across the rows, then stacks rows underneath", async () => {
    await resetAndOpenTutorial("Dictionaries and value mapping — Start");
    await $("div.cell-display*=100").waitForExist();

    await openCombine();
    await chooseOption("Relationship", "Same rows in order");

    await chooseOption("Other table", "Category fixes");
    // Exact cannot work here and the dialog must say so before the click:
    // four rows against a two-value list.
    const add = $("button*=Add columns");
    await add.waitForExist();
    expect(await add.isEnabled()).toBe(false);
    await expect($(".combine-refusal")).toHaveText(
      expect.stringContaining("Exact needs equal counts")
    );

    await chooseOption("Fill", "Repeat to fill");
    await add.waitForEnabled();
    await add.click();
    await $(".combine-dialog").waitForExist({ reverse: true });

    // The two mapped values, tiled across four rows.
    await browser.waitUntil(
      async () =>
        JSON.stringify(await columnCellTexts("Value")) ===
        JSON.stringify(["Office supplies", "Travel", "Office supplies", "Travel"]),
      {
        timeoutMsg: `the repeated column never tiled: ${JSON.stringify(
          await columnCellTexts("Value").catch(() => "<no Value column>")
        )}`,
      }
    );

    // The appended step is editable on its own in Wrangle, fill included —
    // the dialog is an authoring surface, not a separate chain construct.
    await $('[aria-label="Pair fill"]').waitForExist();
    expect(await $('[aria-label="Pair fill"]').getValue()).toBe("repeat");

    await openCombine();
    await chooseOption("Relationship", "Rows under rows");
    await chooseOption("Other table", "Category fixes");
    const stack = $("button*=Stack rows");
    await stack.waitForClickable();
    await stack.click();
    await $(".combine-dialog").waitForExist({ reverse: true });

    await browser.waitUntil(
      async () => (await columnCellTexts("Category")).length === 6,
      {
        timeoutMsg: `stacking never grew the row count: ${
          (await columnCellTexts("Category")).length
        }`,
      }
    );
  });
});
