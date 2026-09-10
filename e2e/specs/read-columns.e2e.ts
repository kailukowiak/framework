import { browser, $, expect } from "@wdio/globals";
import { Key } from "webdriverio";
import { resetAndOpenTutorial, pointAtColumnCell, columnCellTexts } from "../lib/helpers";

// The Read editor emits one rename through Tauri; the grid must show the
// engine's answer and Undo must restore the entire batch in one operation.
describe("Read column names", () => {
  it("renames pasted headers together and undoes the batch", async () => {
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();
    expect(await $(".scratchwork-formula-bar").getSize("height")).toBeLessThanOrEqual(34);
    await pointAtColumnCell("Revenue", 0);
    await expect($('[aria-label="Collapsed inspector"]')).toExist();
    await browser.keys([Key.Command, "3"]);
    await $('[aria-label="Read source"]').waitForExist();
    await $("summary=Column names").click();
    const names = $('textarea[aria-label="Column names"]');
    const original = await names.getValue();
    const replacement = original.split("\n").map((name) =>
      name === "Revenue" ? "Sales" : name === "Cost" ? "Expenses" : name).join("\n");
    await names.setValue(replacement);
    // WKWebView's embedded driver does not perform native focus transfer.
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await $('button[aria-label="Sort by Sales"]').waitForExist();
    await $('button[aria-label="Sort by Expenses"]').waitForExist();
    await browser.keys([Key.Command, "z"]);
    await $('button[aria-label="Sort by Revenue"]').waitForExist();
    await $('button[aria-label="Sort by Cost"]').waitForExist();
  });
  it("reads a column as text and restores the numeric result with Undo", async () => {
    await $("summary=Column types").click();
    await chooseReadOption("Read column", "Revenue");
    await chooseReadOption("Read column type", "Text");
    await browser.waitUntil(async () => (await columnCellTexts("Revenue")).includes("142000"))
      .catch(async (error) => { throw new Error(`${error}\n${await browser.execute(() => document.body.innerText)}`); });
    await expect($('[aria-label="Read column type"]')).toHaveValue("string");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.keys([Key.Command, "z"]);
    await $("div.cell-display*=142,000").waitForExist();
    await expect($('[aria-label="Read column type"]')).toHaveValue("");
  });
  it("edits a column from the top bar while the inspector stays collapsed", async () => {
    await $('[aria-label="Hide inspector"]').click();
    await pointAtColumnCell("Revenue", 0);
    const formula = $(".scratchwork-formula-bar textarea");
    await formula.setValue("=`Revenue` * 2");
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => ((await formula.getAttribute("aria-label")) ?? "").startsWith("Edit Revenue"));
    await expect($('[aria-label="Collapsed inspector"]')).toBeDisplayed();
    await expect($(".inspector")).not.toBeDisplayed();
    await formula.click();
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await columnCellTexts("Revenue")).some((value) => value.replaceAll(",", "") === "284000"))
      .catch(async (error) => { throw new Error(`${error}\n${await browser.execute(() => document.body.innerText)}`); });
    await expect($(".inspector")).not.toBeDisplayed();
  });
});

// The embedded driver clicks option elements without changing the native
// selection. Supply the select's input event directly; the operation, engine
// result, rendered cells and Undo are still exercised through the real app.
async function chooseReadOption(label: string, option: string) {
  await browser.execute((name: string, text: string) => {
    const select = document.querySelector<HTMLSelectElement>(`select[aria-label="${name}"]`);
    const choice = Array.from(select?.options ?? []).find((item) => item.textContent === text);
    if (!select || !choice) throw new Error(`Missing option: ${name} / ${text}`);
    select.value = choice.value;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }, label, option);
}
