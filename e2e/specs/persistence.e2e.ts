import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { resetAndOpenTutorial } from "../lib/helpers";
import { launchSecondInstance } from "../lib/secondInstance";

// The product's core promise: every edit is on disk before the key comes
// back up. Nothing proves that like a process boundary — so after editing a
// cell in the harness's app, this spec launches a second, independent app
// instance whose fresh Data library must offer the workbook under Recents
// and show the edited value once opened. The second process never saw the
// edit happen; the only place it can get 143,000 from is the file.
describe("persistence", () => {
  it("an edit survives a fresh process, reached through Recents", async () => {
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");

    const cell = $("div.cell-display*=142,000");
    await cell.waitForExist();
    await cell.click();
    await browser.keys(Key.F2);
    const editor = $(".cell-editor");
    await editor.waitForExist();
    await editor.setValue("143000");
    await browser.keys(Key.Enter);
    await $("div.cell-display*=143,000").waitForExist();

    const second = await launchSecondInstance();
    try {
      await second.click(
        `//button[contains(., "Formula clicks tutorial")]`
      );
      await second.waitForElement(
        `//div[contains(@class, "cell-display")][contains(., "143,000")]`
      );
    } finally {
      await second.dispose();
    }
  });
});
