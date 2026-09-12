import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary, waitForGutterAnswer } from "../lib/helpers";

async function vary(label: string, text: string) {
  // Native select compensation shared with models.e2e: the embedded driver's
  // selection does not dispatch the bubbling change consumed by React.
  await browser.execute((name, optionText) => {
    const select = Array.from(document.querySelectorAll("select")).find((item) => item.getAttribute("aria-label") === name)!;
    select.value = Array.from(select.options).find((option) => option.text === optionText)!.value;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }, label, text);
}

async function answers(expected: string[]) {
  await browser.waitUntil(async () => JSON.stringify(await browser.execute(() =>
    Array.from(document.querySelectorAll('[aria-label="Calculation Matrix output"] td')).map((cell) => Number(cell.textContent))
  )) === JSON.stringify(expected.map(Number)), { timeout: 15000, timeoutMsg: `Expected sensitivity answers ${expected.join(", ")}` }).catch(async (error) => {
    const state = await browser.execute(() => ({
      text: document.querySelector(".calculation-matrix-card")?.textContent,
      formulas: Array.from(document.querySelectorAll<HTMLTextAreaElement>(".calculation-matrix-card textarea")).map((field) => field.value),
      cells: Array.from(document.querySelectorAll(".calculation-matrix-output td")).map((cell) => ({ text: cell.textContent, error: cell.getAttribute("title") })),
    }));
    throw new Error(`${String(error)}: ${JSON.stringify(state)}`);
  });
}

describe("matrix sensitivity", () => {
  it("varies two named inputs without changing their live values and undoes the binding", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();
    await browser.keys([Key.Command, "j"]);
    const source = $('textarea[aria-label$=" lines"]');
    await source.waitForExist();
    await source.setValue("growth = slider(0,1,0.1,0.2)\nprice = slider(1,100,1,10)\nrevenue = (1 + growth) * price");
    await waitForGutterAnswer("12");
    const sourceText = await source.getValue();
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await $("button*=Matrix").click();
    const rows = $("label*=Add rows source").$("textarea");
    await rows.waitForExist();
    await rows.setValue("[0.1,0.3]");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    const rowInput = $('[aria-label="Rows · Row input"]');
    await rowInput.waitForExist();
    // The dropdown labels retain the block's qualified name.
    const rowLabel = await rowInput.$('option[value]:not([value=""])').getText();
    await vary("Rows · Row input", rowLabel);
    await browser.waitUntil(async () => (await rowInput.getValue()) !== "");
    const columns = $("label*=Add columns source").$("textarea");
    await columns.setValue("[10,20]");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await $('[aria-label="Columns · Column input"]').waitForExist();
    const priceLabel = await browser.execute(() => Array.from(document.querySelector<HTMLSelectElement>('[aria-label="Columns · Column input"]')!.options).find((option) => option.text.includes("price"))!.text);
    await vary("Columns · Column input", priceLabel);
    await browser.waitUntil(async () => (await $('[aria-label="Columns · Column input"]').getValue()) !== "");
    const blockName = (await source.getAttribute("aria-label"))!.replace(/ lines$/, "");
    const body = $('.calculation-matrix-body-formula textarea');
    await body.setValue(`\`${blockName}\`.\`revenue\``);
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await answers(["11", "22", "13", "26"]);
    await waitForGutterAnswer("12");
    if ((await source.getValue()) !== sourceText) throw new Error("Sensitivity changed the live inputs");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await vary("Columns · Column input", "Local axis only");
    await answers(["11", "11", "13", "13"]);
    await browser.keys([Key.Command, "z"]);
    await answers(["11", "22", "13", "26"]);
  });
});
