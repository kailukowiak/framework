import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import {
  blockDraft,
  closeDataLibrary,
  focusBlockSource,
  pointAtCell,
  pressAndRelease,
  waitForGutterAnswer,
} from "../lib/helpers";

// The pop-out is a second view of one workbook: edits return to the canvas,
// and the formula cursor can still point at data in the canvas webview.
describe("Scratchwork window", () => {
  it("opens from Quick Commands and shares edits and formula pointing", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await pressAndRelease('[aria-label="Free-form data canvas"]');
    await browser.execute(() => {
      const data = new DataTransfer();
      data.setData("text/plain", "Amount\n21");
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new ClipboardEvent("paste", {
          clipboardData: data,
          bubbles: true,
          cancelable: true,
        })
      );
    });
    await $("div.cell-display*=21").waitForExist();

    const workbook = await browser.getWindowHandle();
    await browser.keys([Key.Command, Key.Shift, "p"]);
    const search = $('[aria-label="Search commands"]');
    await search.setValue("scratchwork window");
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 2, {
      timeoutMsg: "Quick Commands did not open a Scratchwork window",
    });

    const scratchwork = (await browser.getWindowHandles()).find(
      (handle) => handle !== workbook
    );
    await browser.switchToWindow(scratchwork!);
    await $('[aria-label="Scratchwork window"]').waitForExist();
    await focusBlockSource("Scratchwork");
    await $(".block-source").setValue("picked = ");

    await browser.switchToWindow(workbook);
    await pointAtCell("21");

    await browser.switchToWindow(scratchwork!);
    await waitForGutterAnswer("21");
    expect(await blockDraft("Scratchwork")).toContain("picked = ");

    await browser.switchToWindow(workbook);
    await browser.waitUntil(
      async () => (await $(".block-preview-source").getText()).includes("picked = "),
      { timeoutMsg: "the canvas preview did not receive the pop-out edit" }
    );
  });
});
