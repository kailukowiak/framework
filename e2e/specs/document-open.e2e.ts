import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { documentJson, openTutorialsAndExamples, resetAndOpenTutorial } from "../lib/helpers";

// What has to be true the moment a different document arrives in this window.
//
// A scroll position belongs to a window, not to a file — nothing saves one —
// so a window that had been scrolled right and down kept that offset when the
// next document arrived in it. Opening the tour's Start workbook straight
// after its Answer key showed exactly that: a workbook whose main table was
// off screen before a single gesture, with the formula bar announcing a
// pointing session nobody had started, because the previous document's ⌘J
// request survived the open too. Five callers used to keep five drifting
// lists of what an open clears; this spec is the seam under all of them.
describe("opening another document", () => {
  /** Opens a tutorial workbook the way a person does after the first one. */
  async function openFromLibrary(title: string): Promise<void> {
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await $(".dataset-dialog").waitForExist();
    await openTutorialsAndExamples();
    const entry = $(".dataset-dialog").$(`button*=${title}`);
    await entry.waitForClickable();
    await entry.click();
  }

  /** The canvas's scroll offset, in the pixels the viewport itself counts. */
  const canvasScroll = () =>
    browser.execute(() => {
      const viewport = document.querySelector<HTMLElement>(".canvas-viewport");
      return {
        left: viewport?.scrollLeft ?? -1,
        top: viewport?.scrollTop ?? -1,
      };
    });

  it("scrolls the first workbook's canvas away from its contents", async () => {
    await resetAndOpenTutorial("The FrameWork tour — Answer key");
    await browser.waitUntil(async () => (await documentJson()).includes("Monthly sales"), {
      timeoutMsg: "the tour's Answer key never loaded",
    });

    await browser.execute(() => {
      const viewport = document.querySelector<HTMLElement>(".canvas-viewport")!;
      viewport.scrollLeft = viewport.scrollWidth;
      viewport.scrollTop = viewport.scrollHeight;
    });
    const scrolled = await canvasScroll();
    // If the canvas cannot scroll at all there is nothing for the next test
    // to prove, so this is an assertion rather than a setup step.
    expect(scrolled.left + scrolled.top).toBeGreaterThan(100);
  });

  it("opens the next workbook at its own top-left, with nothing being edited", async () => {
    await openFromLibrary("The FrameWork tour — Start");
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          Array.from(document.querySelectorAll<HTMLInputElement>(".frame-name")).some(
            (name) => name.value === "Budget"
          )
        ),
      { timeoutMsg: "the tour's Start workbook never drew its cards" }
    );

    await browser.waitUntil(
      async () => {
        const scroll = await canvasScroll();
        // The corner of the cards rather than a flat (0, 0), less the
        // margin that keeps the topmost card off the very edge: the Start
        // workbook lays its first card out at 70.
        return scroll.left >= 0 && scroll.left < 120 && scroll.top >= 0 && scroll.top < 120;
      },
      {
        timeoutMsg: `the new document kept the previous one's scroll offset: ${JSON.stringify(
          await canvasScroll()
        )}`,
      }
    );

    // No formula session came across with it. The bar says how the mode ends
    // for exactly as long as the mode is running, so that sentence is the
    // honest test for "an editor is active".
    const bar = await browser.execute(
      () => document.querySelector(".scratchwork-formula-bar")?.textContent ?? ""
    );
    expect(bar).not.toContain("esc to finish");
  });

  it("names the rail's document panel after what it lists", async () => {
    // "Data" named a panel of frames and where each one reads from, beside a
    // Data library that opens files: two different things wearing one word.
    const rail = await browser.execute(() =>
      Array.from(document.querySelectorAll<HTMLElement>(".rail-button")).map((button) =>
        (button.textContent ?? "").trim()
      )
    );
    expect(rail).toContain("Sources");
    expect(rail).not.toContain("Data");
  });

  it("gives a fresh six-row table room to draw six rows", async () => {
    // The card's chrome tally was short by the draft row and half its rules,
    // so a six-row paste arrived showing five rows and an in-card scrollbar:
    // a card sized for content it had no room to draw.
    const frameCards = () =>
      browser.execute(() => document.querySelectorAll(".frame-object").length);
    const before = await frameCards();
    await browser.keys([Key.Command, Key.Alt, "f"]);
    await browser.waitUntil(async () => (await frameCards()) === before + 1, {
      timeoutMsg: "the new frame card never appeared",
    });

    await browser.execute(() => {
      const cards = document.querySelectorAll<HTMLElement>(".frame-object");
      const cell = cards[cards.length - 1]?.querySelector<HTMLElement>("td[data-column-id]");
      if (!cell) throw new Error("the new frame has no cell to press");
      for (const type of ["pointerdown", "pointerup"])
        cell.dispatchEvent(
          new PointerEvent(type, { bubbles: true, cancelable: true, button: 0 })
        );
    });
    await browser.execute(() => {
      const transfer = new DataTransfer();
      transfer.setData(
        "text/plain",
        "Month\tRevenue\n2026-01\t118000\n2026-02\t124000\n2026-03\t136000\n2026-04\t142000\n2026-05\t151000\n2026-06\t168000"
      );
      (document.activeElement ?? window).dispatchEvent(
        new ClipboardEvent("paste", {
          clipboardData: transfer,
          bubbles: true,
          cancelable: true,
        })
      );
    });

    await browser.waitUntil(
      async () => {
        const shape = await browser.execute(() => {
          const cards = document.querySelectorAll<HTMLElement>(".frame-object");
          const card = cards[cards.length - 1];
          const scroll = card?.querySelector<HTMLElement>(".frame-scroll");
          const columnId = card
            ?.querySelector<HTMLElement>("td[data-column-id]")
            ?.getAttribute("data-column-id");
          if (!card || !scroll || !columnId) return null;
          return {
            rows: card.querySelectorAll(
              `td[data-column-id="${columnId}"] div.cell-display`
            ).length,
            overflow: scroll.scrollHeight - scroll.clientHeight,
          };
        });
        return shape?.rows === 6 && shape.overflow <= 1;
      },
      {
        timeoutMsg: `the pasted six rows did not all fit the card: ${JSON.stringify(
          // The same first-column count the assertion makes, not every
          // column's: a bare cell count reads as "twelve rows" and sends the
          // next reader looking for a bug that is not there.
          await browser.execute(() => {
            const cards = document.querySelectorAll<HTMLElement>(".frame-object");
            const card = cards[cards.length - 1];
            const scroll = card?.querySelector<HTMLElement>(".frame-scroll");
            const columnId = card
              ?.querySelector<HTMLElement>("td[data-column-id]")
              ?.getAttribute("data-column-id");
            return {
              rows: columnId
                ? card?.querySelectorAll(
                    `td[data-column-id="${columnId}"] div.cell-display`
                  ).length ?? -1
                : -1,
              overflow: scroll ? scroll.scrollHeight - scroll.clientHeight : -1,
            };
          })
        )}`,
      }
    );
  });
});
