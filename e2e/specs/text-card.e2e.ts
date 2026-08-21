import { $, browser } from "@wdio/globals";
import { closeDataLibrary } from "../lib/helpers";

// Prose crosses the whole desktop seam here: the rail creates a real engine
// object, blur saves parsed formula segments, and the computed answer returns
// as rendered markdown. Completion itself is React-local and has a mounted
// interaction test; repeating that catalog assertion here would prove less.
describe("text card", () => {
  it("renders a live scalar formula inside markdown", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await $("button*=Text").click();
    await $('[title="Edit text"]').click();
    const editor = $('[aria-label="Text markdown"]');
    await editor.waitForExist();
    await editor.setValue("## Note\n\nTotal is {{1 + 2}}.");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());

    await browser.waitUntil(
      async () =>
        browser.execute(
          () =>
            document.querySelector(".text-card-body")?.textContent?.includes("Total is 3") ??
            false
        ),
      { timeoutMsg: "the text card never rendered its computed formula" }
    );
  });

  it("shows a formula error instead of echoing failed source", async () => {
    await $('[title="Edit text"]').click();
    const editor = $('[aria-label="Text markdown"]');
    await editor.setValue("Broken is {{`Missing value`}}.");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());

    const error = $(".text-hole.broken");
    await error.waitForExist();
    await browser.waitUntil(
      async () => (await error.getText()).startsWith("Formula error:"),
      { timeoutMsg: "the text card hid the formula error" }
    );
    if ((await error.getText()).includes("{{`Missing value`}}"))
      throw new Error("the failed formula source was rendered as the answer");
  });

  it("resizes from the card grow box", async () => {
    const before = await browser.execute(
      () => document.querySelector(".text-object")?.getBoundingClientRect().width ?? 0
    );
    const handle = $('[aria-label="Resize Text"]');
    await handle.waitForExist();
    await browser.execute(() => {
      const target = document.querySelector<HTMLElement>('[aria-label="Resize Text"]');
      if (!target) throw new Error("text resize handle is missing");
      const card = target.closest<HTMLElement>(".text-object");
      if (!card) throw new Error("text card is missing");
      const bounds = card.getBoundingClientRect();
      target.dispatchEvent(
        new PointerEvent("pointerdown", {
          bubbles: true,
          cancelable: true,
          clientX: bounds.right,
          clientY: bounds.bottom,
        })
      );
      window.dispatchEvent(
        new PointerEvent("pointermove", {
          bubbles: true,
          clientX: bounds.right + 140,
          clientY: bounds.bottom + 80,
        })
      );
      window.dispatchEvent(
        new PointerEvent("pointerup", {
          bubbles: true,
          clientX: bounds.right + 140,
          clientY: bounds.bottom + 80,
        })
      );
    });
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          () => document.querySelector(".text-object")?.getBoundingClientRect().width ?? 0
        )) >= before + 130,
      { timeoutMsg: "the text card did not grow after dragging its resize handle" }
    );
  });
});
