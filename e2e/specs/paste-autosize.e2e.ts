import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import { closeDataLibrary, pressAndRelease } from "../lib/helpers";

// A paste into an empty frame decides the frame's shape, and the card has to
// follow: the two-by-two stub it replaces used to stay the size it was and
// clip the result both ways.
describe("paste into an empty frame", () => {
  it("sizes the card to the columns and rows that arrived", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await browser.keys([Key.Command, Key.Alt, "f"]);
    const card = $(".frame-object");
    await card.waitForExist();
    const before = await card.getSize();

    await pressAndRelease(".frame-object td[data-column-id]");
    await browser.execute(() => {
      const dt = new DataTransfer();
      dt.setData(
        "text/plain",
        "Month\tRegion\tRevenue\tCost\n2026-01\tEast\t118000\t76000\n2026-02\tWest\t124000\t79000\n2026-03\tEast\t136000\t85000"
      );
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true })
      );
    });
    await $('button[aria-label="Sort by Cost"]').waitForExist();
    await browser.waitUntil(async () => (await card.getSize()).width > before.width, {
      timeoutMsg: "the card kept the empty stub's width after the paste",
    });
  });

  it("selects a whole column from its header", async () => {
    // The card's own press handler used to widen a header press back to the
    // frame, so a column could be dragged but never selected.
    await pressAndRelease('th.column-header:has(button[aria-label="Sort by Cost"])');
    await $("th.column-header.active").waitForExist();
    expect(
      await $("th.column-header.active").$('button[aria-label="Sort by Cost"]').isExisting()
    ).toBe(true);
  });
});
