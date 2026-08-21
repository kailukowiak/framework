import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { openContextMenuOn, resetAndOpenTutorial } from "../lib/helpers";

// The one sanctioned authoring surface for a calculated column: the frame
// context menu appends a withColumns step to the Wrangle chain and focuses
// its formula. The creation gesture first saves `null.cast("number")` — a
// typed, blank column visible immediately — and replacing that formula must
// flow through the pipeline into the grid. This crosses context menu →
// pipeline operation → engine → rendered cells, which is exactly the chain
// AGENTS.md routes to a native workflow spec.
describe("calculated column through Wrangle", () => {
  it("adds a typed blank column from the header menu", async () => {
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();

    await openContextMenuOn('[aria-label="Sort by Revenue"]');
    const add = $("button*=Add calculated column");
    await add.waitForExist();
    await add.click();

    // The placeholder column is real before any formula is typed: named,
    // number-typed, cells blank. Read through execute because the driver's
    // bare `*=` text strategy locates nothing (tag-scoped `div.foo*=` works).
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          (document.body.textContent ?? "").includes("Column 1")
        ),
      { timeoutMsg: "the placeholder calculated column never appeared" }
    );

    // The creation gesture's other promise: the formula arrives focused with
    // the whole line selected — name included, because `Column 1` is
    // nobody's chosen name — so one keystroke starts replacing all of it.
    await browser.waitUntil(
      async () => {
        const state = await browser.execute(() => {
          const active = document.activeElement;
          return active instanceof HTMLTextAreaElement &&
            !active.classList.contains("block-source")
            ? {
                length: active.value.length,
                start: active.selectionStart,
                end: active.selectionEnd,
              }
            : null;
        });
        return (
          state !== null &&
          state.length > 0 &&
          state.start === 0 &&
          state.end === state.length
        );
      },
      {
        timeoutMsg:
          "the new formula never held focus with its whole text selected",
      }
    );
  });

  it("replacing the placeholder formula computes down the grid", async () => {
    // The Wrangle chain's formula editor is the one textarea that is not
    // the Scratchwork block.
    const formula = $('//textarea[not(contains(@class, "block-source"))]');
    await formula.waitForExist();
    // A withColumns formula names its output: `Column` = expression. The
    // spec keeps the name the creation gesture chose and replaces only the
    // expression — feeding it a bare expression earns the product's own
    // inline "Write a backticked column name, =, and a formula" error.
    await formula.setValue("`Column 1` = `Revenue` * 2");
    // The formula editor's commit is ⌘↵ — the editor labels it "⌘↵ run" —
    // handled on its own keydown, so the textarea has to hold focus first.
    // Clicking a focusable element does move focus, unlike the divs.
    await formula.click();
    await browser.keys([Key.Command, Key.Enter]);

    // April's revenue is 142000; the calculated cell must render its double
    // through the real pipeline.
    await $("div.cell-display*=284,000").waitForExist();
  });
});
