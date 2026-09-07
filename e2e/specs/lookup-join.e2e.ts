import { browser, $ } from "@wdio/globals";
import { resetAndOpenTutorial } from "../lib/helpers";

// The Excel-simple lookup path: drag the column wanted from one table onto
// the destination key, confirm the inferred source key, and get a derived
// table whose first Wrangle row records that relationship. This crosses the
// pointer gesture, full-plan diagnostics, two operations, engine join, and
// rendered result; a mounted component test cannot prove that seam.
describe("look up columns by dragging", () => {
  it("brings Budget into Actuals and records the keys in Wrangle", async () => {
    await resetAndOpenTutorial("Month-end close — Start");
    await $("div.cell-display*=168,000").waitForExist();

    await browser.execute(() => {
      const cardNamed = (name: string) =>
        Array.from(document.querySelectorAll<HTMLElement>(".canvas-object")).find(
          (card) => card.querySelector<HTMLInputElement>(".frame-name")?.value === name
        );
      const headerNamed = (card: HTMLElement | undefined, name: string) =>
        Array.from(card?.querySelectorAll<HTMLElement>(".column-header") ?? []).find(
          (header) => (header.querySelector(".column-select")?.textContent ?? "").trim() === name
        );
      const source = headerNamed(cardNamed("Budget"), "Budget")?.querySelector<HTMLElement>(
        ".column-select"
      );
      const target = headerNamed(cardNamed("Actuals"), "Key");
      if (!source || !target) throw new Error("lookup drag headers are missing");
      const from = source.getBoundingClientRect();
      const to = target.getBoundingClientRect();
      source.dispatchEvent(new PointerEvent("pointerdown", {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: from.left + from.width / 2,
        clientY: from.top + from.height / 2,
      }));
      window.dispatchEvent(new PointerEvent("pointermove", {
        bubbles: true,
        cancelable: true,
        buttons: 1,
        clientX: to.left + to.width / 2,
        clientY: to.top + to.height / 2,
      }));
      window.dispatchEvent(new PointerEvent("pointerup", {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: to.left + to.width / 2,
        clientY: to.top + to.height / 2,
      }));
    });

    await $(".lookup-join-prompt").waitForExist();

    // Confirming the key used to remove the row that asked about it, which
    // recentred the dialog and slid "Bring columns over" up under the
    // pointer — the next click landed somewhere nobody had aimed at. The
    // answer replaces the question in place, so the button below it must not
    // move at all.
    const create = $("button*=Bring columns over");
    await create.waitForExist();
    const buttonTop = (await create.getLocation()).y;
    await $("button*=Mark Key as unique").click();
    const unique = $(".lookup-key-unique");
    await unique.waitForExist();
    await expect(unique).toHaveText(expect.stringContaining("is unique"));
    expect(Math.abs((await create.getLocation()).y - buttonTop)).toBeLessThan(2);

    await create.waitForEnabled();
    await create.click();

    await browser.waitUntil(
      () => browser.execute(() =>
        Array.from(document.querySelectorAll<HTMLInputElement>(".frame-name")).some(
          (name) => name.value === "Actuals + Budget"
        )
      ),
      { timeoutMsg: "the joined frame never appeared" }
    );
    await browser.execute(() => {
      const card = Array.from(document.querySelectorAll<HTMLElement>(".canvas-object")).find(
        (candidate) =>
          candidate.querySelector<HTMLInputElement>(".frame-name")?.value ===
            "Actuals + Budget"
      );
      card?.dispatchEvent(
        new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
      );
    });
    await $("button=Wrangle").click();
    await $("strong=Look up columns").waitForExist();
    expect(await $('[aria-label="Destination join key"]').getValue()).not.toBe("");
    expect(await $('[aria-label="Lookup join key"]').getValue()).not.toBe("");
  });
});
