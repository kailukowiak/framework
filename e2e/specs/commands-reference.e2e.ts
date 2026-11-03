import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary, focusBlockSource } from "../lib/helpers";

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

  // A question asked of the palette is answered there, beside the commands,
  // and choosing the answer opens it in the docked Reference — the words are
  // never typed a second time into a second box.
  it("answers a help question from Quick Commands", async () => {
    await browser.keys(Key.Escape);
    await browser.keys([Key.Command, Key.Shift, "p"]);
    const search = $('[aria-label="Search commands"]');
    await search.waitForExist();
    await search.setValue("pivot");
    // Partial-text selectors take a bare tag; the listbox scope is what keeps
    // this to the palette's own rows.
    const answer = $('[aria-label="Commands"]').$("button*=Pivot");
    await answer.waitForExist({ timeoutMsg: "the palette listed no help answer for pivot" });
    await answer.click();

    const panel = $('[aria-label="FrameWork Reference"]');
    await panel.$("h2=Pivot").waitForExist({
      timeoutMsg: "choosing the palette's help answer did not open it in the Reference",
    });
    await expect($('[aria-label="Search help"]')).toHaveValue("pivot");
  });

  // ⌘K reaches the app through the menu, which cannot see the cursor; the
  // formula bar reads the call the Scratchwork cursor is inside, so the
  // Reference opens on that function rather than on a blank search.
  it("opens the function under a Scratchwork cursor from ⌘K", async () => {
    await $('[aria-label="Close help"]').click();
    await browser.keys([Key.Command, "j"]);
    const source = $(".block-source");
    await source.waitForExist();
    await source.setValue("sequence(1, ");
    await focusBlockSource();
    await browser.keys([Key.Command, "k"]);

    const panel = $('[aria-label="FrameWork Reference"]');
    await panel.$("h2*=sequence(").waitForExist({
      timeoutMsg: "⌘K inside sequence( did not open sequence in the Reference",
    });
    // Docked beside the canvas, the panel leaves the block it was opened from
    // in place to keep writing into.
    await expect(source).toExist();
  });

  // Escape used to close nothing here. The panel's keydown never reached a
  // window listener in the packaged app, so the one key everybody presses to
  // dismiss a panel worked in Quick Commands — which handles it on its own
  // surface — and not in the Reference. The docked panel handles it on its
  // own surface too, and only there: on the canvas Escape means "stop editing
  // this cell". Both places a person's focus can be when they press it inside
  // the panel are covered: the search field, and a result row.
  it("closes on Escape from the search field", async () => {
    const panel = $('[aria-label="FrameWork Reference"]');
    await panel.waitForExist();
    const search = $('[aria-label="Search help"]');
    await search.waitForExist();
    await search.setValue("sum");
    await $('[aria-label="Help results"]').$("button[role=option]").waitForExist();
    // Focus put back in-page, for the reason `focusBlockSource` exists: the
    // embedded driver's input transfers no DOM focus, and the claim under
    // test is specifically about a key raised *inside* the panel.
    await focusInsidePanel('[aria-label="Search help"]');

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
    await $('[aria-label="Search help"]').setValue("sum");

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
