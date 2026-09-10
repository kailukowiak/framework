import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import {
  columnCellTexts,
  digits,
  documentJson,
  openContextMenuOn,
  pressAndRelease,
  resetAndOpenTutorial,
  selectCard,
} from "../lib/helpers";

/** The frame's column names, left to right, as the grid draws them. */
const headerNames = () =>
  browser.execute(() =>
    Array.from(
      document
        .querySelector<HTMLElement>(".frame-object")
        ?.querySelectorAll<HTMLElement>('button[aria-label^="Sort by "]') ?? []
    ).map((button) => button.getAttribute("aria-label")!.replace("Sort by ", ""))
  );

/** The top formula bar's textarea, whatever formula it is currently hosting. */
const formulaBar = () => $(".scratchwork-formula-bar textarea");

/** Brings the frame's Wrangle chain up, wherever the inspector happens to be. */
async function openWrangle(): Promise<void> {
  await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
  await selectCard(".frame-object");
  await browser.keys([Key.Command, "3"]);
  await $(".pipeline-command-list").waitForExist();
}

// The one sanctioned authoring surface for a calculated column: the frame
// context menu appends a withColumns step to the Wrangle chain and focuses
// its formula. The creation gesture first saves `None.cast("number")` — a
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

    // Revenue is a column in the middle of the table, and the new calculation
    // still belongs at the end of it. Wedging Column 1 between Revenue and
    // Cost would be a positional edit nobody asked for — "Insert column here"
    // is the gesture that means that.
    const names = await headerNames();
    expect(names[names.length - 1]).toBe("Column 1");
    expect(names.indexOf("Column 1")).toBeGreaterThan(names.indexOf("Cost"));
  });

  // This test once failed as "orphaned formula session commit no-op": the
  // creation gesture saved a chain that could not reconcile with its own
  // echo — unformatted, and spelling the placeholder `null` where the
  // engine renders `None` — so the round trip reseeded the wrangle step
  // list with fresh row identities, and the session the gesture had just
  // focused was left addressing an editor that no longer existed: the bar
  // edited the draft, Return saved nothing. The inspector never unmounted;
  // the editor under it changed identity. Creation now persists the
  // formatted, canonically spelled chain (identity survives the round
  // trip), and the registry keeps a departed surface's binding so Return
  // still commits even after the wrangle surface is closed.
  it("replacing the placeholder formula computes down the grid", async () => {
    // The Wrangle chain's formula editor is the one textarea that is not
    // the Scratchwork block.
    const formula = formulaBar();
    await formula.waitForExist();
    // A withColumns formula names its output: `Column` = expression. The
    // spec keeps the name the creation gesture chose and replaces only the
    // expression — feeding it a bare expression earns the product's own
    // inline "Write a backticked column name, =, and a formula" error.
    await formula.setValue("`Column 1` = `Revenue` * 2");
    // The one non-block textarea is the top bar's mirror of the wrangle
    // formula ("Edit Column 1"), not an inspector-local editor — so the
    // commit gesture is the bar's, and the bar commits on Return. The
    // driver delivers Enter as a key event (it only skips the newline
    // default), which is exactly what the bar's handler listens for.
    await formula.click();
    await browser.keys(Key.Enter);

    // April's revenue is 142000; the calculated cell must render its double
    // through the real pipeline. Computed cells are not the literal-cell
    // div.cell-display: they render as button.computed-cell, the clickable
    // formula-reference surface.
    await $("button.computed-cell*=284,000").waitForExist();
  });

  // Nobody reading "name it Profit and enter `Revenue` - `Cost`" types the
  // quoting, and refusing `Profit = …` taught the syntax by rejection. The
  // name is accepted bare and stored canonically, so the saved chain reads
  // the same either way — which is what the Wrangle line is asserted on.
  it("accepts a name written without backticks and prints it backticked", async () => {
    await openWrangle();
    await $(".pipeline-command-list").$("button.pipeline-command").click();
    const formula = formulaBar();
    await formula.waitForExist();
    await formula.setValue("Profit = `Revenue` - `Cost`");
    await formula.click();
    await browser.keys(Key.Enter);

    // April's 142000 less its 91000 of cost, through the real pipeline.
    await browser.waitUntil(
      async () => {
        const profit = await columnCellTexts("Profit").catch(() => []);
        return profit.some((value) => Number(digits(value)) === 51000);
      },
      { timeoutMsg: "the bare-named Profit column never computed" }
    );
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          Array.from(document.querySelectorAll(".pipeline-command")).some((command) =>
            (command.textContent ?? "").includes("`Profit`")
          )
        ),
      { timeoutMsg: "the Wrangle line did not print the name back in backticks" }
    );
  });

  // Half a formula is not a calculation yet. The draft used to be written
  // into the step on every keystroke, and a step is what the *next* save
  // writes to the document — so an abandoned draft arrived in the file under
  // some unrelated gesture's save, and the schema preview kept announcing
  // "unknown name" about a word still being typed.
  it("keeps an abandoned draft out of the document when something else saves", async () => {
    await openWrangle();
    await $(".pipeline-command-list").$("button.pipeline-command").click();
    const formula = formulaBar();
    await formula.waitForExist();
    await formula.setValue("Profit = `Revenue` - `Cost` - 4242");

    // Left without Return, the way an abandoned edit is left: focus goes
    // elsewhere and an unrelated save follows.
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    const name = $(".frame-object .frame-name");
    await name.setValue("Monthly sales renamed");
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await documentJson()).includes("Monthly sales renamed"), {
      timeoutMsg: "the frame rename never reached the document",
    });

    expect(await documentJson()).not.toContain("4242");
    // And the calculation the person did save is still the one on file.
    const profit = await columnCellTexts("Profit");
    expect(profit.some((value) => Number(digits(value)) === 51000)).toBe(true);
  });

  // A calculated column's rows arrive already evaluated while the cached
  // `computed.rows` still describes the frame's *stored* input, where a
  // calculated column has no literal at all. Reading that null as an empty
  // cell is what made Profit, selected on its own, report "Count 0" over six
  // numbers plainly on screen.
  it("counts a calculated column's rows in the selection statistics", async () => {
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await pressAndRelease('th.column-header:has(button[aria-label="Sort by Profit"])');

    await browser.waitUntil(
      async () => {
        const reading = await browser.execute(
          () =>
            document.querySelector('[aria-label="Selection statistics"]')?.textContent ??
            ""
        );
        return /Count\s*[1-9]/.test(reading);
      },
      {
        timeoutMsg: `selecting the calculated column read ${JSON.stringify(
          await browser.execute(
            () =>
              document.querySelector('[aria-label="Selection statistics"]')
                ?.textContent ?? "<no statistics>"
          )
        )}`,
      }
    );
  });
});
