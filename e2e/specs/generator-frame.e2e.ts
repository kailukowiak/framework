import { browser, $ } from "@wdio/globals";
import { closeDataLibrary, openContextMenuOn } from "../lib/helpers";

// A generator frame from the canvas menu: rows grown from a rule, and the
// rule editable in place. This crosses context menu → addGeneratorFrame →
// engine evaluation → rendered rows → setFrameGenerator → regrown rows,
// which is the whole seam the feature lives on. The canvas flow is also
// dialog-free, which is what lets it run here at all.
describe("generator frames", () => {
  it("creates a generator from the canvas menu and shows its rows", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await openContextMenuOn('[aria-label="Free-form data canvas"]');
    const add = $("button*=Add generator here");
    await add.waitForExist();
    await add.click();

    // The default rule is sequence(1, 11): ten rows, 1 through 10.
    const rule = $('input[aria-label="Generator rule"]');
    await rule.waitForExist();
    expect(await rule.getValue()).toContain("sequence(1, 11)");
    await browser.waitUntil(
      () =>
        browser.execute(() => {
          const cells = Array.from(
            document.querySelectorAll(".frame-card .cell-display")
          ).map((cell) => cell.textContent?.trim());
          return cells.includes("10") && cells.includes("1");
        }),
      { timeoutMsg: "the generated rows 1..10 never rendered" }
    );
  });

  it("editing the rule regrows the frame", async () => {
    const rule = $('input[aria-label="Generator rule"]');
    await rule.setValue("sequence(1, 6)");
    // setValue leaves the input focused; the blur() stands in for the
    // click-elsewhere that commits, the same way the tutorial spec's does.
    await rule.click();
    await browser.execute(() =>
      (document.activeElement as HTMLElement | null)?.blur()
    );

    await browser.waitUntil(
      () =>
        browser.execute(() => {
          const cells = Array.from(
            document.querySelectorAll(".frame-card .cell-display")
          ).map((cell) => cell.textContent?.trim());
          return cells.includes("5") && !cells.includes("10");
        }),
      { timeoutMsg: "the regrown rows 1..5 never replaced 1..10" }
    );
  });
});
