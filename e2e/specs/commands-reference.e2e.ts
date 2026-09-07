import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary } from "../lib/helpers";

/** Puts DOM focus on one element of the Reference panel, and proves it landed. */
async function focusInsidePanel(selector: string): Promise<void> {
  const landed = await browser.execute((sel: string) => {
    const target = document.querySelector<HTMLElement>(sel);
    target?.focus();
    const active = document.activeElement;
    return (
      active instanceof Node &&
      Boolean(document.querySelector(".help-browser")?.contains(active))
    );
  }, selector);
  expect(landed).toBe(true);
}

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

  // Escape used to close nothing here. The panel's keydown never reached the
  // window listener in the packaged app, so the one key everybody presses to
  // dismiss a panel worked in Quick Commands — which handles it on its own
  // surface — and not in the Reference. Both places a person's focus can be
  // when they press it are covered: the search field they were typing in, and
  // a result row they had moved to.
  it("closes on Escape from the search field", async () => {
    const panel = $('[aria-label="FrameWork Reference"]');
    await panel.waitForExist();
    const search = $('[aria-label="Search formulas"]');
    await search.waitForExist();
    await search.setValue("sum");
    await $('[aria-label="Help results"]').$("button[role=option]").waitForExist();
    // Focus put back in-page, for the reason `focusBlockSource` exists: the
    // embedded driver's input transfers no DOM focus, and the claim under
    // test is specifically about a key raised *inside* the panel — the
    // window-level listener deliberately ignores those.
    await focusInsidePanel('[aria-label="Search formulas"]');

    await browser.keys(Key.Escape);
    await panel.waitForExist({
      reverse: true,
      timeoutMsg: "Escape from the Reference's search field closed nothing",
    });
  });

  it("closes on Escape with a result row focused", async () => {
    await browser.keys([Key.Command, "k"]);
    const panel = $('[aria-label="FrameWork Reference"]');
    await panel.waitForExist();
    await $('[aria-label="Search formulas"]').setValue("sum");

    const result = $('[aria-label="Help results"]').$("button[role=option]");
    await result.waitForExist();
    await focusInsidePanel('[aria-label="Help results"] button[role="option"]');

    await browser.keys(Key.Escape);
    await panel.waitForExist({
      reverse: true,
      timeoutMsg: "Escape from a Reference result closed nothing",
    });
  });
});
