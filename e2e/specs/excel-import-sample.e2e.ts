import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { openTutorialsAndExamples } from "../lib/helpers";

// This sample is produced by the same range-import function the desktop
// command uses, then opened through the same Data-library card a person uses.
// Seeing values from named tables and loose rectangles proves the `.fw`
// working copy brought its Parquet sidecar along; a plain file copy would render
// imports here even though the sample card itself opened successfully.
describe("Excel import learning files", () => {
  it("creates and opens the pre-filled Excel import tutorial answer", async () => {
    // A compact but supported app window makes the overflow deterministic;
    // the product bug only appears when the library is taller than its host.
    await browser.setWindowSize(1000, 640);

    // The private e2e tutorial directory begins empty. Populate it through
    // the real UI so this reproduces the tall library shown when a person
    // has all eight tutorial workbooks installed.
    await openTutorialsAndExamples();
    const createTutorials = $("button*=Create tutorials");
    await createTutorials.waitForClickable();
    await createTutorials.click();
    await $("button*=Importing an Excel workbook — Start").waitForExist();

    const scrollGeometry = await browser.execute(() => {
      const host = document.querySelector<HTMLElement>(".dataset-dialog-backdrop")!;
      const dialog = document.querySelector<HTMLElement>(".dataset-dialog")!;
      return {
        clientHeight: host.clientHeight,
        scrollHeight: host.scrollHeight,
        innerHeight: window.innerHeight,
        dialogHeight: dialog.getBoundingClientRect().height,
        overflowY: getComputedStyle(host).overflowY,
      };
    });
    if (scrollGeometry.scrollHeight <= scrollGeometry.clientHeight) {
      throw new Error(`Data Library is not vertically scrollable: ${JSON.stringify(scrollGeometry)}`);
    }

    const answer = $("button*=Importing an Excel workbook — Answer key");
    await answer.waitForExist();
    await answer.scrollIntoView();
    const scrollTop = await browser.execute(
      () => document.querySelector<HTMLElement>(".dataset-dialog-backdrop")!.scrollTop
    );
    if (scrollTop <= 0) {
      throw new Error("Excel tutorial did not move into view by scrolling the Data Library");
    }
    await answer.click();

    await $(".markdown-body*=Importing Excel data").waitForExist();
    await $("div.cell-display*=CUS-101").waitForExist();
    await $("div.cell-display*=SKU-101").waitForExist();
    await $("div.cell-display*=North Peak Goods").waitForExist();
    await $("div.cell-display*=ORD-1001").waitForExist();
    await $("div.cell-display*=27.48").waitForExist();
    await $("div.cell-display*=ADJ-201").waitForExist();
    await $("div.cell-display*=Stretch").waitForExist();
  });

  it("still opens the compact three-table example document", async () => {
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await $(".dataset-dialog").waitForExist();
    await openTutorialsAndExamples();
    const sample = $("button*=Excel import workbook");
    await sample.waitForExist();
    await sample.scrollIntoView();
    await sample.click();

    await $("div.cell-display*=SKU-101").waitForExist();
    await $("div.cell-display*=North Peak Goods").waitForExist();
    await $("div.cell-display*=ORD-1001").waitForExist();
  });
});
