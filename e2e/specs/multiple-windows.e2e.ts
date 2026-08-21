import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import { closeDataLibrary } from "../lib/helpers";

// A window is a document boundary, not a second view onto the process-wide
// store. This crosses the menu-less accelerator path, Tauri window creation,
// per-window command routing, and the two independent Rust stores.
describe("multiple document windows", () => {
  it("opens an independent workbook with ⌘⇧N", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();
    await browser.keys([Key.Command, Key.Alt, "b"]);
    await $(".block-object").waitForExist();

    const original = await browser.getWindowHandle();
    await browser.keys([Key.Command, Key.Shift, "n"]);
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 2, {
      timeoutMsg: "⌘⇧N did not create a second document window",
    });

    const created = (await browser.getWindowHandles()).find((handle) => handle !== original);
    expect(created).toBeDefined();
    await browser.switchToWindow(created!);
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await expect($(".block-object")).not.toExist();

    await browser.keys([Key.Command, Key.Alt, "f"]);
    await $(".frame-object").waitForExist();

    await browser.switchToWindow(original);
    await expect($(".block-object")).toExist();
    await expect($(".frame-object")).not.toExist();
  });
});
