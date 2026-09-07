import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { columnCellTexts, focusBlockSource, openContextMenuOn, pointAtColumnCell, resetAndOpenTutorial, waitForGutterAnswer } from "../lib/helpers";

const lookupSource = 'total = `Expenses`.`Amount`.sum()\nlabel = lookup("Offce supplies", `Category fixes`.`Key`, `Category fixes`.`Value`)\nmissing = lookup("Software", `Category fixes`.`Key`, `Category fixes`.`Value`, "Not mapped")';
async function expectMappedRows() {
  await browser.waitUntil(async () => JSON.stringify((await columnCellTexts("Category")).slice(0, 4)) ===
    JSON.stringify(["Office supplies", "Travel", "Software", "Office supplies"]), {
      timeoutMsg: "Mapped category rows differ from the lesson",
    }).catch(async (reason) => {
      const state = await browser.execute(() => ({
        errors: Array.from(document.querySelectorAll('[role="alert"], .error-banner, .frame-edit-refusal')).map((element) => element.textContent),
        formulas: Array.from(document.querySelectorAll("textarea")).map((element) => element.value),
      }));
      throw new Error(`${String(reason)}: ${JSON.stringify(await columnCellTexts("Category"))}; ${JSON.stringify(state)}`);
    });
  await waitForGutterAnswer("200");
}
async function editCell(column: string, row: number, value: string) {
  // The embedded driver does not transfer native focus on synthetic clicks.
  await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
  await pointAtColumnCell(column, row);
  await browser.keys(Key.F2);
  const editor = $(".cell-editor");
  await editor.waitForExist();
  await editor.setValue(value);
  await browser.keys(Key.Enter);
}

describe("dictionary tutorial", () => {
  it("opens the shipped Start and follows the conversion and mapping steps", async () => {
    await browser.setWindowSize(1700, 1100);
    await resetAndOpenTutorial("Dictionaries and value mapping — Start");
    await $("div.cell-display*=Offce supplies").waitForExist();
    await waitForGutterAnswer("200");
    await openContextMenuOn('[aria-label="Sort by Key"]');
    await $(".framework-context-menu").$("button*=Use as dictionary — key: Key").click();
    await browser.waitUntil(() => browser.execute(() => document.querySelector(".framework-context-menu") === null));
    await openContextMenuOn('[aria-label="Sort by Category"]');
    await $(".framework-context-menu").$("summary*=Map values…").click();
    await $(".framework-context-menu").$("button*=Using Category fixes").click();
    await expectMappedRows();
  });

  it("updates both matching rows after a rule edit and restores them on undo", async () => {
    await editCell("Value", 0, "Office costs");
    await browser.waitUntil(async () => (await columnCellTexts("Category")).filter((value) => value === "Office costs").length === 2);
    await browser.keys([Key.Command, "z"]);
    await expectMappedRows();
  });

  it("evaluates the lesson's lookup examples and refuses a duplicate key", async () => {
    const example = await browser.execute(() => document.querySelector(".markdown-body pre code")?.textContent);
    if (example !== lookupSource.split("\n").slice(1).join("\n")) {
      throw new Error("The walkthrough does not display copyable lookup formulas");
    }
    await focusBlockSource("Checks");
    await $('textarea[aria-label="Checks lines"]').setValue(`${lookupSource}\npractice = 12345`);
    await browser.waitUntil(() => browser.execute(() => {
      const answers = document.querySelector('[aria-label="Scratchwork answers"]')?.textContent ?? "";
      return answers.includes("Office supplies") && answers.includes("Not mapped");
    }));
    await editCell("Key", 1, "Offce supplies");
    await browser.waitUntil(() => browser.execute(() => document.body.textContent?.includes("contains duplicates")));
    if (JSON.stringify((await columnCellTexts("Key")).slice(0, 2)) !== JSON.stringify(["Offce supplies", "Travel & meals"])) {
      throw new Error("duplicate-key refusal changed the dictionary");
    }
    await expectMappedRows();
  });

  it("opens the shipped answer key with correct results", async () => {
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await $("button*=Library").click();
    await $(".dataset-dialog").waitForExist();
    const tutorials = $(".dataset-dialog").$("button*=Tutorials and examples");
    await tutorials.scrollIntoView();
    await tutorials.waitForClickable();
    await tutorials.click();
    await $(".dataset-dialog").$("button*=Dictionaries and value mapping — Answer key").click();
    // The extra practice line belongs only to Start; its disappearance proves
    // the answer key loaded rather than asserting against the previous view.
    await browser.waitUntil(async () => await $('textarea[aria-label="Checks lines"]').getValue() === lookupSource);
    await expectMappedRows();
    await browser.waitUntil(() => browser.execute(() => document.querySelector('[aria-label="Scratchwork answers"]')?.textContent?.includes("Not mapped")));
  });
});
