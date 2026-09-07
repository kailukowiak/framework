import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { columnCellTexts, digits, openContextMenuOn, pointAtColumnCell, resetAndOpenTutorial } from "../lib/helpers";

// The embedded driver's pointer gap is the same one documented by
// pointAtColumnCell: dispatch pointer events on the real DOM affordance.
// No operation is injected; the handle, Tauri command, engine, undo, and
// refreshed grid must all agree about what this drag wrote.
async function dragRevenue(to: number) {
  await browser.execute((index: number) => {
    const handle = document.querySelector<HTMLElement>('[aria-label="Fill Revenue"]')!;
    const table = handle.closest("table")!;
    const columnId = handle.closest("td")!.getAttribute("data-column-id");
    const target = table.querySelectorAll<HTMLElement>(`td[data-column-id="${columnId}"]`)[index];
    target.scrollIntoView({ block: "center", inline: "center" });
    const bounds = target.getBoundingClientRect();
    const point = { bubbles: true, cancelable: true, pointerId: 1, button: 0, clientX: bounds.left + 8, clientY: bounds.top + bounds.height / 2 };
    handle.dispatchEvent(new PointerEvent("pointerdown", point));
    window.dispatchEvent(new PointerEvent("pointermove", point));
    window.dispatchEvent(new PointerEvent("pointerup", point));
    const hit = document.elementFromPoint(point.clientX, point.clientY);
    if (hit?.closest("table") !== table) throw new Error(`Fill destination is obscured by ${hit?.tagName}.${hit?.className}`);
  }, to);
}

async function chartContains(value: string) {
  return browser.execute((expected: string) => Array.from(document.querySelectorAll(".plot-visual svg [aria-label]"))
    .some((mark) => (mark.getAttribute("aria-label") ?? "").replaceAll(",", "").includes(expected)), value);
}

describe("analysis regression workflows", () => {
  it("drag fills the previewed rows and undo restores their original values", async () => {
    await browser.setWindowSize(1500, 1000);
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();
    const before = (await columnCellTexts("Revenue")).map(digits);
    await pointAtColumnCell("Revenue", 0);
    await $('[aria-label="Fill Revenue"]').waitForExist();
    await dragRevenue(2);
    await browser.waitUntil(async () => {
      const after = (await columnCellTexts("Revenue")).map(digits);
      return after[1] === before[0] && after[2] === before[0];
    }, { timeoutMsg: "dragging did not fill the indicated rows" });
    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => JSON.stringify((await columnCellTexts("Revenue")).map(digits)) === JSON.stringify(before));
  });

  it("refuses an overflowing paste without changing even the cells that fit", async () => {
    const before = await columnCellTexts("Cost");
    await pointAtColumnCell("Cost", 0);
    await browser.execute(() => {
      const clipboard = new DataTransfer();
      clipboard.setData("text/plain", "999\t1000");
      document.activeElement!.dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
    });
    await browser.waitUntil(() => browser.execute(() => document.body.textContent?.includes("Nothing pasted: the clipboard needs")), { timeoutMsg: "overflow paste did not explain its refusal" });
    const after = await columnCellTexts("Cost");
    if (JSON.stringify(before) !== JSON.stringify(after)) throw new Error("overflowing paste partly changed the table");
  });

  it("updates a paged chart after an upstream edit and after undo", async () => {
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Answer key");
    await $("div.cell-display*=142,000").waitForExist();
    const sourceCard = await browser.execute(() => {
      const tab = Array.from(document.querySelectorAll('[role="tab"]'))
        .find((element) => element.getAttribute("title") === "Monthly sales");
      const id = tab?.closest("[data-object-id]")?.getAttribute("data-object-id");
      if (!id) throw new Error("Monthly sales card is missing");
      return `[data-object-id="${id}"]`;
    });
    await openContextMenuOn(`${sourceCard} [aria-label="Sort by Revenue"]`);
    await $("button*=Plot in a new window").click();
    await browser.waitUntil(() => chartContains("118000"), { timeoutMsg: "initial chart data did not render" }).catch(async (reason) => {
      const state = await browser.execute(() => ({
        cards: Array.from(document.querySelectorAll(".plot-card")).map((card) => card.textContent),
        labels: Array.from(document.querySelectorAll(".plot-visual svg [aria-label]")).map((mark) => mark.getAttribute("aria-label")),
        errors: Array.from(document.querySelectorAll(".error-banner, .plot-render-error")).map((element) => element.textContent),
      }));
      throw new Error(`${String(reason)}: ${JSON.stringify(state)}`);
    });
    await browser.execute(() => {
      const tab = Array.from(document.querySelectorAll<HTMLElement>('[role="tab"]'))
        .find((element) => element.getAttribute("title") === "Monthly sales");
      tab!.click();
    });
    await browser.execute(() => {
      const tab = Array.from(document.querySelectorAll('[role="tab"]'))
        .find((element) => element.getAttribute("title") === "Monthly sales");
      const card = tab!.closest("[data-object-id]")!;
      const cell = Array.from(card.querySelectorAll("div.cell-display"))
        .find((element) => element.textContent?.replace(/[^\d.-]/g, "") === "118000");
      if (!cell) throw new Error("Monthly sales revenue cell is missing");
      cell.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 }));
      cell.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, cancelable: true, button: 0 }));
    });
    await browser.keys(Key.F2);
    const editor = $(".cell-editor");
    await editor.waitForExist();
    await editor.setValue("777000");
    await browser.keys(Key.Enter);
    await browser.waitUntil(() => chartContains("777000"), { timeoutMsg: "paged chart kept its old rows after an upstream edit" });
    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => await chartContains("118000") && !(await chartContains("777000")));
  });

  it("loads exact export counts through the native scope query", async () => {
    await $("button*=Project").click();
    await $("button*=Export to Excel…").click();
    await $('[aria-label="Rows to export"]').waitForExist();
    await browser.waitUntil(async () => browser.execute(() => {
      const row = Array.from(document.querySelectorAll(".excel-export-row"))
        .find((element) => element.textContent?.includes("Monthly sales"));
      return row?.querySelector("small")?.textContent === "6 rows";
    }), { timeoutMsg: "export did not load an exact row count" });
    await $('[aria-label="Rows to export"]').selectByAttribute("value", "all");
    await browser.waitUntil(async () => browser.execute(() => {
      const row = Array.from(document.querySelectorAll(".excel-export-row"))
        .find((element) => element.textContent?.includes("Monthly sales"));
      return row?.querySelector("small")?.textContent === "6 rows";
    }));
    await $('[aria-label="Close Excel export"]').click();
  });
});
