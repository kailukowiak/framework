import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { columnCellTexts, openContextMenuOn, pointAtColumnCell, resetAndOpenTutorial } from "../lib/helpers";

// Use the real menu, clipboard, and Wrangle authoring path. Pointer and
// clipboard events use the same embedded-driver compensation as grid tests.
async function pasteEntries(text: string) {
  await browser.execute((source: string) => {
    const clipboard = new DataTransfer();
    clipboard.setData("text/plain", source);
    document.activeElement!.dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
  }, text);
}

describe("dictionaries", () => {
  it("creates a keyed table and maps an existing column through Wrangle", async () => {
    await browser.setWindowSize(1700, 1100);
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();
    await browser.execute(() => {
      const canvas = document.querySelector('[aria-label="Free-form data canvas"]')!;
      canvas.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 1100, clientY: 550 }));
    });
    await $("button*=Add dictionary here").click();
    await $('[aria-label="Sort by Key"]').waitForExist();
    await pointAtColumnCell("Key", 0);
    await pasteEntries("East\tEastern\nWest\tWestern");
    await browser.waitUntil(async () => (await columnCellTexts("Value")).includes("Eastern"));
    await openContextMenuOn('[aria-label="Sort by Region"]');
    await $("summary*=Map values…").click();
    await $("button*=Using Dictionary").click();
    await browser.waitUntil(async () => (await columnCellTexts("Region")).includes("Eastern"), { timeoutMsg: "mapping did not reach the source column" });
  });

  it("updates mapped results after editing the dictionary, and undo restores them", async () => {
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await pointAtColumnCell("Value", 0);
    await browser.keys(Key.F2);
    const editor = $(".cell-editor");
    await editor.waitForExist();
    await editor.setValue("East corrected");
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await columnCellTexts("Region")).includes("East corrected"));
    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => (await columnCellTexts("Region")).includes("Eastern"));
  });
});
