import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary } from "../lib/helpers";

// The e2e shell has no native menu, so this proves the application's own
// accelerator path and the shared action table behind Quick Commands. Native
// menu wiring remains the one shortcut seam macOS WebDriver cannot exercise.
describe("commands and reference", () => {
  it("opens Reference through Quick Commands", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await browser.keys([Key.Command, Key.Shift, "p"]);
    await $('[aria-label="Quick Commands"]').waitForExist();
    const search = $('[aria-label="Search commands"]');
    await search.setValue("reference");
    await browser.keys(Key.Enter);

    await $('[aria-label="FrameWork Reference"]').waitForExist();
  });

  it("opens formulas from ⌘K while Scratchwork is active", async () => {
    await browser.keys(Key.Escape);
    await browser.keys([Key.Command, "j"]);
    await $(".block-source").waitForExist();
    await browser.keys([Key.Command, "k"]);

    const formulas = $("button=Formulas");
    await formulas.waitForExist();
    await expect(formulas).toHaveAttribute("aria-pressed", "true");
  });
});
