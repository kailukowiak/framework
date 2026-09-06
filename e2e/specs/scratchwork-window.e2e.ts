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

  it("points into the pop-out even after the canvas block was edited", async () => {
    // A canvas-block session outlives its focus on purpose, so after ⌘J the
    // workbook still holds a Scratchwork editor that is merely blurred. The
    // pop-out is where the caret is; the pick has to land there.
    // While the pop-out is open the canvas block is a preview, so close it
    // first: the session under test is the one ⌘J opens on the canvas.
    const workbook = await browser.getWindowHandle();
    for (const handle of await browser.getWindowHandles()) {
      if (handle === workbook) continue;
      await browser.switchToWindow(handle);
      await browser.closeWindow();
    }
    await browser.switchToWindow(workbook);
    await browser.keys([Key.Command, "j"]);
    await $(".block-source").waitForExist();
    await browser.keys([Key.Command, Key.Shift, "p"]);
    const search = $('[aria-label="Search commands"]');
    await search.setValue("scratchwork window");
    await browser.keys(Key.Enter);

    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 2, {
      timeoutMsg: "Quick Commands did not reopen the Scratchwork window",
    });
    const scratchwork = (await browser.getWindowHandles()).find(
      (handle) => handle !== workbook
    );
    await browser.switchToWindow(scratchwork!);
    await $('[aria-label="Scratchwork window"]').waitForExist();
    await focusBlockSource("Scratchwork");
    await $(".block-source").setValue("picked = 21\nagain = ");

    await browser.switchToWindow(workbook);
    await pointAtCell("21");

    await browser.switchToWindow(scratchwork!);
    await browser.waitUntil(
      async () => /again = .*`Amount`/.test(await blockDraft("Scratchwork")),
      { timeoutMsg: "the pop-out did not receive the pointed reference" }
    );
  });

  it("survives retyping a line from scratch", async () => {
    // Select-all and retype, twice, with the caret passing through an open
    // backtick: the sequence that once looped the pop-out into React #185.
    const pointed = /again = (.+)$/m.exec(await blockDraft("Scratchwork"))?.[1];
    expect(pointed).toBeTruthy();
    await focusBlockSource("Scratchwork");
    await $(".block-source").setValue("x = 5");
    await waitForGutterAnswer("5");
    await $(".block-source").setValue(`total = ${pointed}`);
    await waitForGutterAnswer("21");
    expect(await $("p*=could not recover").isExisting()).toBe(false);
  });
});
