import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary, waitForGutterAnswer } from "../lib/helpers";

describe("named variable controls", () => {
  it("edits constructor values through the UI, recomputes dependents and undoes", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();
    await browser.keys([Key.Command, "j"]);
    const source = $('textarea[aria-label$=" lines"]');
    await source.waitForExist();
    await source.setValue('growth = slider(0,1,0.1,value=0.2)\nregion = dropdown(["North","South"])\ncutoff = date_input(date(2026,12,31))\nanswer = growth * 100\nselected = region\nyear = cutoff.dt.year()');
    await waitForGutterAnswer("20");
    await waitForGutterAnswer("2026");
    // The embedded driver cannot transfer native focus on click; leave the
    // source editor explicitly, as in the formula-click tutorial workflow.
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    const value = $('[aria-label="growth value"]');
    await value.waitForExist();
    await value.setValue("0.35");
    await browser.keys(Key.Enter);
    await waitForGutterAnswer("35");
    await browser.waitUntil(async () => (await source.getValue()).includes("value=0.35"));
    await browser.keys([Key.Command, "z"]);
    await waitForGutterAnswer("20");
    await browser.waitUntil(async () => (await value.getValue()) === "0.2");
    // Same native-select compensation as models.e2e: the embedded driver
    // changes the DOM selection without the bubbling event React needs.
    await browser.execute(() => {
      const select = document.querySelector<HTMLSelectElement>('[aria-label="region choice"]')!;
      const option = Array.from(select.options).find((item) => item.text === "South")!;
      select.value = option.value;
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await browser.waitUntil(async () => (await browser.execute(() => document.querySelector('[aria-label="Open selected result"]')?.textContent ?? "")).includes("South"));
    await browser.waitUntil(async () => (await source.getValue()).includes('value="South"'));
    const date = $('[aria-label="cutoff date"]');
    await date.setValue("2027-01-01");
    await browser.keys(Key.Enter);
    await waitForGutterAnswer("2027");
    await browser.keys([Key.Command, "z"]);
    await waitForGutterAnswer("2026");
    await browser.waitUntil(async () => (await date.getValue()) === "2026-12-31");
  });
});
