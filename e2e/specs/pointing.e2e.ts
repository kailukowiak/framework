import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import {
  blockDraft,
  columnCellTexts,
  digits,
  focusBlockSource,
  pointAtCell,
  resetAndOpenTutorial,
  selectCard,
  waitForGutterAnswer,
} from "../lib/helpers";

/** The top formula bar's textarea, whatever formula it is currently hosting. */
const formulaBar = () => $(".scratchwork-formula-bar textarea");

/** What the bar is showing, focus state included, in one round trip. */
const barState = () =>
  browser.execute(() => {
    const input = document.querySelector<HTMLTextAreaElement>(
      ".scratchwork-formula-bar textarea"
    );
    if (!input) return null;
    return {
      value: input.value,
      start: input.selectionStart,
      end: input.selectionEnd,
      focused: document.activeElement === input,
    };
  });

/**
 * A gesture on the grid cell at a column and a display row — addressed the
 * way a person addresses it, by which column and which row rather than by
 * what it happens to say.
 *
 * `gesture` is "context" for the right-click that offers "Formula here", and
 * "click" for an ordinary left press. The left press is a whole press:
 * pointerdown, pointerup, *and* the click that follows them on real
 * hardware. The click is the point. A cancelled pointerdown does not suppress
 * the click in WKWebView, so a pointed reference used to insert correctly and
 * then also select the cell it pointed at — moving the keyboard to the grid's
 * clipboard target, where the next characters typed went into the cell
 * instead of the formula. A helper that dispatched only the pointer halves
 * could not see that at all.
 *
 * All of it in one in-page call: the lookup and the events have to be the
 * same turn, because a marker left on a cell between two calls is a marker
 * React's next render throws away.
 */
async function cellGesture(
  columnName: string,
  rowIndex: number,
  gesture: "click" | "context"
): Promise<void> {
  await browser.execute(
    (name: string, index: number, kind: string) => {
      const header = document
        .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
        ?.closest("th");
      const columnId = header?.getAttribute("data-column-id");
      if (!columnId) throw new Error(`no column header for ${name}`);
      const cell = Array.from(
        document.querySelectorAll<HTMLElement>(`td[data-column-id="${columnId}"]`)
      )[index];
      if (!cell) throw new Error(`no ${name} cell at row ${index}`);
      if (kind === "context") {
        cell.dispatchEvent(
          new MouseEvent("contextmenu", { bubbles: true, cancelable: true })
        );
        return;
      }
      const target =
        cell.querySelector<HTMLElement>("div.cell-display, button.computed-cell") ?? cell;
      for (const type of ["pointerdown", "pointerup"])
        target.dispatchEvent(
          new PointerEvent(type, { bubbles: true, cancelable: true, button: 0 })
        );
      target.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    },
    columnName,
    rowIndex,
    gesture
  );
}

/** A whole press on a column's own header — what "click the column" means. */
async function clickColumnHeader(columnName: string): Promise<void> {
  await browser.execute((name: string) => {
    const header = document
      .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
      ?.closest("th");
    const target = header?.querySelector<HTMLElement>("button.column-select");
    if (!target) throw new Error(`no column header for ${name}`);
    for (const type of ["pointerdown", "pointerup"])
      target.dispatchEvent(
        new PointerEvent(type, { bubbles: true, cancelable: true, button: 0 })
      );
    target.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  }, columnName);
}

/**
 * A press on a context-menu item, by the words on it.
 *
 * In page, like the cell gestures above, and for the same reason they are:
 * the driver's own click never reaches this menu. The menu is a fixed
 * overlay the driver locates and scrolls to happily — its button reports a
 * live rect well inside the viewport, and `elementFromPoint` at that rect's
 * centre answers with the button itself — yet the native press that follows
 * arrives nowhere, leaving the menu open and nothing chosen. Dispatching
 * the whole press on the button is what a person's click does reach.
 */
async function clickMenuItem(text: string): Promise<void> {
  const press = (label: string) => {
    const item = Array.from(
      document.querySelectorAll<HTMLElement>(".framework-context-menu button")
    ).find((node) => (node.textContent ?? "").includes(label));
    if (!item) return true;
    for (const type of ["pointerdown", "pointerup"])
      item.dispatchEvent(
        new PointerEvent(type, { bubbles: true, cancelable: true, button: 0 })
      );
    item.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    return false;
  };
  await browser.waitUntil(
    async () =>
      browser.execute(
        (label: string) =>
          Array.from(
            document.querySelectorAll<HTMLElement>(".framework-context-menu button")
          ).some((node) => (node.textContent ?? "").includes(label)),
        text
      ),
    { timeoutMsg: `no context menu item reading ${text}` }
  );
  // Pressed until the menu answers by closing. A menu item pressed in the
  // same breath as the right-click that opened it can land before the menu
  // is listening — the item is in the document, its handler is not attached
  // to it yet — and the press is simply lost. Choosing an item always closes
  // the menu, so the menu still standing is the signal to press again.
  await browser.waitUntil(async () => browser.execute(press, text), {
    timeoutMsg: `pressing ${text} never closed the menu`,
  });
}

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
    // The block by name: the regenerated Start workbook ships one block,
    // "Checks" — the old Scratchwork block no longer exists in it, and a
    // bare .block-source would answer whichever block renders first.
    const source = $('textarea[aria-label="Checks lines"]');
    await source.waitForExist();
    await source.setValue("April = ");
    await focusBlockSource("Checks");

    await pointAtCell("142,000");

    // The inserted token's spelling belongs to the engine; the spec asserts
    // the outcome — the draft grew past what was typed and the real engine
    // evaluated the pointed-at cell.
    await waitForGutterAnswer("142000");
    expect(await blockDraft("Checks")).not.toBe("April = ");
  });

  it("refuses the same cell once a display sort makes rows positional", async () => {
    await $('[aria-label="Sort by Month"]').click();
    // The sort landed when January leads the Month column. Asserted on the
    // first cell's text, not on "2026-01 exists somewhere": the unsorted
    // table already holds January on its second row, so the weaker wait
    // passed before the sort had applied and the next assertions read a
    // table that was never reordered.
    await browser.waitUntil(
      async () => (await columnCellTexts("Month"))[0] === "2026-01",
      { timeoutMsg: "the Month sort never put January first" }
    );

    const source = $('textarea[aria-label="Checks lines"]');
    await source.setValue("Later = ");
    await focusBlockSource("Checks");
    await pointAtCell("142,000");

    const notice = $(".notice-toast");
    await notice.waitForExist();
    await expect(notice).toHaveText(
      expect.stringContaining("stable row address")
    );
    expect(await blockDraft("Checks")).toBe("Later = ");
  });

  // The other half of pointing: a *wrangle* formula rather than a block line.
  // The month sort the previous test applied is the precondition the tutorial
  // insists on — `.shift(1)` only means anything once the order is declared —
  // and the six rows now read January to June, so the row above row 2 is
  // January's 118000.
  it("builds a shift by pointing, and leaves the keyboard in the bar", async () => {
    // A person's next click would blur the Checks editor; the driver's
    // synthesized events run no default actions, so this stands in for it.
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.waitUntil(
      async () => Number(digits((await columnCellTexts("Revenue"))[0])) === 118000,
      { timeoutMsg: "the Month sort never put January first" }
    );

    await cellGesture("Revenue", 1, "context");
    await clickMenuItem("Formula here");

    // The gesture's promise: a placeholder calculation, in the bar, with the
    // whole line selected so one keystroke replaces all of it. The bar holds
    // the *formatted* spelling the chain is saved with — `None.cast("number")`
    // laid out over two lines — so the assertion reads the shape of the line
    // rather than one exact rendering of the placeholder expression.
    let seen: Awaited<ReturnType<typeof barState>> = null;
    try {
      await browser.waitUntil(async () => {
        seen = await barState();
        return (
          seen !== null &&
          seen.value.startsWith("`Column 1` =") &&
          seen.value.includes('cast("number")') &&
          seen.start === 0 &&
          seen.end === seen.value.length
        );
      });
    } catch {
      throw new Error(
        `Formula here left the bar holding ${JSON.stringify(
          seen
        )} instead of the placeholder, selected whole`
      );
    }

    await cellGesture("Revenue", 0, "click");

    // One row above the anchor is one period back — and the column's name
    // survives, because a line without its name is not a calculation any more
    // and could not be saved at all.
    await browser.waitUntil(
      async () => (await barState())?.value === "`Column 1` = `Revenue`.shift(1)",
      {
        timeoutMsg: `pointing wrote ${JSON.stringify(
          (await barState())?.value
        )} instead of the shift`,
      }
    );

    // The click the pick carried must not also have been a cell click: type
    // one character and it belongs to the formula, not to the grid.
    await browser.keys("x");
    const typed = await barState();
    expect(typed?.value).toContain("x");
    expect(typed?.focused).toBe(true);
  });

  it("saves the pointed formula under a name typed over the placeholder", async () => {
    const bar = formulaBar();
    await bar.setValue("`Previous revenue` = `Revenue`.shift(1)");
    // The bar commits on Return; the click restores the focus setValue does
    // not take, the same way the wrangle spec does it.
    await bar.click();
    await browser.keys(Key.Enter);

    await browser.waitUntil(
      async () => {
        const previous = await columnCellTexts("Previous revenue").catch(() => []);
        return previous.length > 1 && Number(digits(previous[1])) === 118000;
      },
      {
        timeoutMsg:
          "Previous revenue never showed January's 118000 against February's row",
      }
    );
  });

  it("says a calculation written beside this one cannot be read yet", async () => {
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await selectCard(".frame-object");
    await browser.keys([Key.Command, "3"]);
    const step = $(".pipeline-command-list");
    await step.waitForExist();

    // A second line inside the *same* Add or replace columns step — which is
    // exactly the arrangement where one calculation cannot read the other.
    await step.$("button*=Add or replace column").click();
    await browser.waitUntil(
      async () => (await barState())?.value.includes('None.cast("number")') === true,
      { timeoutMsg: "the added line never opened in the bar" }
    );

    await clickColumnHeader("Previous revenue");

    const notice = $(".notice-toast");
    await notice.waitForExist();
    await expect(notice).toHaveText(
      expect.stringContaining("is added in this same step")
    );
    // The refusal left the session alone: the bar is still writing the new
    // column rather than having quietly become the one that was clicked.
    expect((await barState())?.value).not.toContain("`Revenue`.shift(1)");
  });
});
