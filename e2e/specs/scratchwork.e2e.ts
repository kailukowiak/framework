import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import {
  closeDataLibrary,
  resetAndOpenTutorial,
  waitForGutterAnswer,
} from "../lib/helpers";

// The scratchpad on a blank canvas: ⌘J summons a Scratchwork block, lines
// evaluate as they are typed, and editing a line upstream moves the answers
// downstream. This is the shortest path through the whole stack — keyboard,
// parser, engine, live recompute — with no document fixture at all.
//
// Multi-line entry goes through setValue rather than typed Enter keys: the
// embedded WebDriver server inserts printable characters as text but does
// not perform Enter's default newline insertion, and a literal "\n" in a
// key action crashes its script interpolation. setValue is still the
// element send-keys endpoint — the same input a person's typing amounts to
// on the wire — so nothing here bypasses the interface.
describe("scratchpad", () => {
  it("summons with ⌘J and evaluates lines live", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await browser.keys([Key.Command, "j"]);
    const source = $(".block-source");
    await source.waitForExist();

    await source.setValue("x = 10\ny = 30\nx + y");

    await waitForGutterAnswer("40");
  });

  it("recomputes dependents when an upstream line changes", async () => {
    const source = $(".block-source");
    await source.setValue("x = 20\ny = 30\nx + y");

    await waitForGutterAnswer("50");
  });

  // Where ⌘J puts the block, on a canvas bigger than the window. "Somewhere
  // free" is the right answer only while everything fits: with a workbook
  // open, the free space was below the tables, and focusing the new block's
  // editor scrolled the canvas down to it — so the table the tour was about
  // to ask you to click had left the screen. A card you asked for arrives
  // where you are looking, and what you were looking at is still there.
  //
  // The claim is made against whatever was on screen before the keystroke,
  // not against a named table: the embedded driver sizes the window in
  // physical pixels, so on a 2× display the 2400×1700 asked for here is a
  // 1200×850 window, and on a smaller screen it is whatever fits. Which
  // cards that shows is the harness's business; that none of them leaves is
  // the application's.
  it("summons the block onto the screen without pushing the visible cards off it", async () => {
    await browser.setWindowSize(2400, 1700);
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await resetAndOpenTutorial("The FrameWork tour — Start");
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          Array.from(document.querySelectorAll<HTMLInputElement>(".frame-name")).some(
            (name) => name.value === "Monthly sales"
          )
        ),
      { timeoutMsg: "the tour's Start workbook never drew its Monthly sales table" }
    );

    // Which cards a person could see before asking for the block.
    const visibleCards = (): Promise<string[]> =>
      browser.execute(() => {
        const viewport = document
          .querySelector<HTMLElement>(".canvas-viewport")
          ?.getBoundingClientRect();
        if (!viewport) return [];
        return Array.from(document.querySelectorAll<HTMLElement>(".canvas-object"))
          .filter((card) => {
            const rect = card.getBoundingClientRect();
            return (
              Math.min(rect.right, viewport.right) - Math.max(rect.left, viewport.left) > 0 &&
              Math.min(rect.bottom, viewport.bottom) - Math.max(rect.top, viewport.top) > 0
            );
          })
          .map((card) => card.dataset.objectId ?? card.id ?? card.className);
      });
    const before = await visibleCards();
    expect(before.length).toBeGreaterThan(0);

    await browser.keys([Key.Command, "j"]);
    await $(".block-object").waitForExist();

    let seen: unknown = null;
    try {
      await browser.waitUntil(
        async () => {
          const geometry = await browser.execute(() => {
            const viewport = document
              .querySelector<HTMLElement>(".canvas-viewport")
              ?.getBoundingClientRect();
            const block = document
              .querySelector<HTMLElement>(".block-object")
              ?.getBoundingClientRect();
            if (!viewport || !block) return null;
            return {
              viewport: viewport.toJSON(),
              block: block.toJSON(),
              window: { width: window.innerWidth, height: window.innerHeight },
              // The block has arrived when its name and first lines are on
              // screen — where the typing goes. On a canvas with no free
              // screen the whole block cannot show without taking a kept
              // card away, and the kept cards win (see minimalRevealScroll).
              blockOnScreen:
                block.left >= viewport.left - 1 &&
                block.right <= viewport.right + 1 &&
                block.top >= viewport.top - 1 &&
                block.top + 120 <= viewport.bottom + 1,
            };
          });
          const after = await visibleCards();
          // "At least partly": a card does not have to be untouched, only
          // still visible.
          const stillThere = before.every((id) => after.includes(id));
          seen = { ...geometry, before, after };
          return geometry?.blockOnScreen === true && stillThere;
        },
        { timeoutMsg: "placement" }
      );
    } catch {
      // The rectangles travel with the message: a placement bug is a
      // geometry question, and "off screen" alone sends the reader back to
      // reproduce it by hand.
      throw new Error(
        `⌘J either put the block off screen or scrolled a visible card away: ${JSON.stringify(seen)}`
      );
    }
  });
});
