import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { digits, resetAndOpenTutorial, waitForGutterAnswer } from "../lib/helpers";

// The Scenarios, sensitivity and goal seek answer key, opened the way a
// learner opens it, and its section-5 objects read off the real canvas:
// two `under` lines answering while the document stays on Base, a bound
// sensitivity grid saying what it cost, and a `solve` line whose gutter
// reports the search and whose Apply writes the answer into `Price` as an
// ordinary edit — which ⌘Z takes back. Every number here is the lesson's
// own checkpoint; the seam under test is UI → engine → recompute → history
// for the three phase-3 surfaces at once.

/** The gutter rows' text, top to bottom, read the way waitForGutterAnswer does. */
async function gutterRows(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('[aria-label="Scratchwork answers"] > *')).map(
      (row) => row.textContent ?? ""
    )
  );
}

async function waitForRowContaining(text: string): Promise<string> {
  let found = "";
  await browser.waitUntil(
    async () => {
      const rows = await gutterRows();
      found = rows.find((row) => row.includes(text)) ?? "";
      return found !== "";
    },
    {
      timeout: 15000,
      timeoutMsg: `No Scratchwork gutter row contains ${JSON.stringify(text)}. Rows: ${(
        await gutterRows()
      ).join(" | ")}`,
    }
  );
  return found;
}

describe("goal seek tutorial answer key", () => {
  it("opens the answer key with the under lines answering on Base", async () => {
    await resetAndOpenTutorial("Scenarios, sensitivity and goal seek — Answer key");
    // Base ebitda, then the two scenario reads beside it — while the
    // scenario switcher still says Base.
    await waitForGutterAnswer("150000");
    await waitForGutterAnswer("272500");
    await waitForGutterAnswer("42500");
    const scenario = await $('select[aria-label="Scenario"]').getValue();
    if (scenario !== "") throw new Error(`Expected the document on Base, got ${scenario}`);
  });

  it("says how many evaluations the bound sensitivity grid ran", async () => {
    const cost = $(".calculation-matrix-cost");
    await cost.waitForExist();
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          () => document.querySelector(".calculation-matrix-cost")?.textContent ?? ""
        )).includes("25 evaluations"),
      { timeout: 15000, timeoutMsg: "The bound grid never reported 25 evaluations" }
    );
  });

  it("reports the solve in the gutter and applies it as an undoable edit", async () => {
    const solved = await waitForRowContaining("steps");
    // The answer is the first number in the row; the steps and residual
    // follow it, so the row is read piecewise rather than as one number.
    const answer = Number(digits(solved.split("·")[0]));
    if (Math.abs(answer - 138.75) > 1e-6) throw new Error(`Solved ${answer}, expected 138.75: ${solved}`);
    // The Apply button's text follows the residual with no separator, so
    // "residual 0Apply" is the zero case; "residual 0.5" is not.
    if (!/residual 0(?![.\d])/.test(solved)) throw new Error(`Residual not reported as zero: ${solved}`);

    const apply = $('button[aria-label="Apply solved value to Price"]');
    await apply.waitForClickable();
    await apply.click();
    // Price is now 138.75, so the base model hits the target …
    await waitForGutterAnswer("300000");
    // … and the search, asked again from the new price, lands on itself.
    await waitForRowContaining("steps");

    await browser.keys([Key.Command, "z"]);
    await waitForGutterAnswer("150000");
  });
});
