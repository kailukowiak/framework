import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import {
  closeDataLibrary,
  columnCellTexts,
  digits,
  openContextMenuOn,
  resetAndOpenTutorial,
} from "../lib/helpers";

// PlotCard renders through vega-embed, and Vega's default runtime compiles
// expressions with `new Function(...)` -- eval, which the bundled app's CSP
// refuses on purpose (no unsafe-eval in script-src). `tauri dev` never
// enforces CSP at all, so a regression in the CSP-safe interpreter wiring
// is invisible at the desk and only shows up as a red EvalError in a real
// build. This spec is what would have caught it: it drives the actual
// "Plot in this card" gesture in the e2e bundle, which does enforce CSP,
// and checks for a rendered chart mark rather than trusting a mocked test.
describe("plotting a frame", () => {
  it("renders a mark instead of a CSP eval error", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await $("button*=Frame").click();

    // The blank frame ships with two empty rows already; typing into the
    // draft row at the foot adds real ones instead, and the draft inputs
    // carry a genuine accessible name (`New row <column>`) rather than
    // needing a coordinate click into a blank, unlabeled cell.
    const col1 = $('input[aria-label="New row Column 1"]');
    const col2 = $('input[aria-label="New row Column 2"]');
    await col1.waitForExist();
    await col1.setValue("10");
    await col2.setValue("5");
    await browser.keys(Key.Enter);
    await $("div.cell-display*=10").waitForExist();

    await col1.setValue("20");
    await col2.setValue("15");
    await browser.keys(Key.Enter);
    await $("div.cell-display*=20").waitForExist();

    // Right-click the column header, same as a person would, and choose
    // the tab-shaped plot rather than the popped-out one -- both reach the
    // same `embed()` call in PlotCard, so either proves the fix.
    await openContextMenuOn('[aria-label="Sort by Column 1"]');
    const plotHere = $("button*=Plot in this card");
    await plotHere.waitForExist();
    await plotHere.click();

    // Neither column has typed data, so the default spec falls back to a
    // bar of Column 1 against a row count -- deterministic regardless of
    // numeric type inference, and enough to prove marks were drawn at all.
    await browser.waitUntil(
      async () => {
        const state = await browser.execute(() => {
          const shell = document.querySelector(".plot-visual-shell");
          const errorText =
            shell?.querySelector(".plot-render-error")?.textContent ?? "";
          // Vega's SVG renderer stamps the root <svg class="marks"> and
          // gives each data mark's group a "role-mark" class (see
          // vega-scenegraph's cssClass helper) -- distinct from the axis,
          // legend, and vega-embed's own action-menu icons, so this can't
          // pass on chrome alone.
          const marks = shell?.querySelectorAll("svg.marks .role-mark").length ?? 0;
          return { errorText, marks };
        });
        if (/EvalError|Refused to evaluate/.test(state.errorText)) {
          throw new Error(`plot render blocked by CSP: ${state.errorText}`);
        }
        return state.marks > 0;
      },
      { timeoutMsg: "the plot card never rendered a mark" }
    );
  });

  // A declared sort is part of what the table is. The default spec used to
  // order a categorical axis by the bars' own heights ("-y"), so a chart of a
  // table sorted by month drew the months by descending revenue — a picture
  // of a different table than the one on screen. The axis is read out of the
  // rendered SVG rather than out of the spec object, because what a person
  // sees is the claim.
  it("draws a category axis in the table's own row order", async () => {
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();

    await $('[aria-label="Sort by Month"]').click();
    await browser.waitUntil(
      async () => Number(digits((await columnCellTexts("Revenue"))[0])) === 118000,
      { timeoutMsg: "the Month sort never put January first" }
    );

    await openContextMenuOn('[aria-label="Sort by Month"]');
    await $(".framework-context-menu").$("button*=Plot in this card").click();

    await browser.waitUntil(
      async () => {
        // Every text Vega drew, filtered to the month labels: the axis
        // renderer's class names are Vega's business, the labels are ours.
        const months = await browser.execute(() =>
          Array.from(
            document.querySelectorAll(".plot-visual-shell svg.marks text")
          )
            .map((text) => (text.textContent ?? "").trim())
            .filter((text) => /^2026-\d\d$/.test(text))
        );
        return months.length >= 2 && months[0] === "2026-01";
      },
      {
        timeoutMsg:
          "the month axis did not begin at January — the chart is ordering by bar height, not by the table's rows",
      }
    );
  });
});
