import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary } from "../lib/helpers";

// A compact variable crosses the same important seam as Scratchwork — rail
// gesture, public operation, parser, live evaluation, and the shared formula
// editor — while deliberately drawing only one variable per canvas object.
describe("compact variable", () => {
  it("adds from the rail and edits vectors locally and dates in the top bar", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await $("button*=Variable").click();
    const local = $(".variable-card textarea");
    await local.waitForExist();
    await local.setValue("[1, 2, 3]");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await $(".variable-answer*=[1, 2, 3]").waitForExist();

    // The embedded driver does not refocus an already-known textarea when a
    // second setValue follows a programmatic blur, while a person necessarily
    // clicks back into it. Make that real gesture explicit.
    await local.click();
    await local.setValue("[1, 2, 3].at(1)");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await $(".variable-answer*=→ 1").waitForExist();
    await browser.waitUntil(async () => (await local.getValue()).includes("\n  .at(1)"), {
      timeoutMsg: "the compact variable did not show its autoformatted multiline formula",
    });

    const before = await browser.execute(() => {
      const card = document.querySelector<HTMLElement>(".variable-object");
      if (!card) throw new Error("variable card is missing");
      return { width: card.getBoundingClientRect().width, height: card.getBoundingClientRect().height };
    });
    await browser.execute(() => {
      const handle = document.querySelector<HTMLElement>('[aria-label="Resize x"]');
      const card = handle?.closest<HTMLElement>(".variable-object");
      if (!handle || !card) throw new Error("variable resize handle is missing");
      const bounds = card.getBoundingClientRect();
      handle.dispatchEvent(new PointerEvent("pointerdown", {
        bubbles: true, cancelable: true, clientX: bounds.right, clientY: bounds.bottom,
      }));
      window.dispatchEvent(new PointerEvent("pointermove", {
        bubbles: true, clientX: bounds.right + 120, clientY: bounds.bottom + 60,
      }));
      window.dispatchEvent(new PointerEvent("pointerup", {
        bubbles: true, clientX: bounds.right + 120, clientY: bounds.bottom + 60,
      }));
    });
    await browser.waitUntil(async () => {
      const after = await browser.execute(() => {
        const card = document.querySelector<HTMLElement>(".variable-object");
        if (!card) return { width: 0, height: 0 };
        return { width: card.getBoundingClientRect().width, height: card.getBoundingClientRect().height };
      });
      return after.width >= before.width + 110 && after.height >= before.height + 50;
    }, { timeoutMsg: "the variable did not grow after dragging its resize handle" });

    const top = $('textarea[aria-label="Edit x"]');
    await top.waitForExist();
    await top.click();
    await top.setValue('"2024-10-10"d');
    await browser.keys(Key.Enter);
    await $(".variable-answer*=2024-10-10").waitForExist();
  });
});
