import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import { closeDataLibrary, pressAndRelease } from "../lib/helpers";

// The native menu is the one seam WebDriver cannot touch: the harness build
// draws no menu, because synthesized keys never reach an NSMenu. What that
// left untested was not the menu's appearance but its *routing* — whether the
// id an item emits still finds a handler. That gap is how an enabled-but-inert
// Undo shipped in the Scratchwork pop-out and had to be found by hand.
//
// `replay_menu_command` (src-tauri/src/menu.rs, compiled only under the `e2e`
// feature) closes it: it refuses an id the menu does not declare, then emits
// through the same `deliver` a real click uses. So everything downstream of
// the click — the event, the window it lands in, the handler, the operation,
// the engine, the redraw — is the real application. The click itself, and the
// menu's own drawing, remain Kai's manual check.
describe("native menu routing", () => {
  it("replays menu commands into the workbook they name", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    // A small table, pasted the way the pop-out spec pastes one, so there is
    // a document change for Undo to be about.
    await pressAndRelease('[aria-label="Free-form data canvas"]');
    await browser.execute(() => {
      const data = new DataTransfer();
      data.setData("text/plain", "Amount\n21");
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new ClipboardEvent("paste", {
          clipboardData: data,
          bubbles: true,
          cancelable: true,
        })
      );
    });
    await $("div.cell-display*=21").waitForExist();

    await replayMenuCommand("undo");
    await $("div.cell-display*=21").waitForExist({ reverse: true });

    await replayMenuCommand("redo");
    await $("div.cell-display*=21").waitForExist();

    await replayMenuCommand("quick-commands");
    await $('[aria-label="Search commands"]').waitForExist();
    await browser.keys(Key.Escape);
    await $('[aria-label="Search commands"]').waitForExist({ reverse: true });

    const workbook = await browser.getWindowHandle();
    await replayMenuCommand("open-scratchwork-window");
    await browser.waitUntil(
      async () => (await browser.getWindowHandles()).length === 2,
      { timeoutMsg: "the replayed menu command opened no Scratchwork window" }
    );
    const scratchwork = (await browser.getWindowHandles()).find(
      (handle) => handle !== workbook
    );
    await browser.switchToWindow(scratchwork!);
    await $('[aria-label="Scratchwork window"]').waitForExist();
    await browser.switchToWindow(workbook);
  });

  it("refuses an id the menu does not declare", async () => {
    expect(await replayOutcome("not-a-menu-command")).toContain(
      "is not a menu command"
    );
  });
});

/**
 * Invokes the replay command and answers with what it settled on — "ok", or
 * the rejection's text. Invoke-then-poll rather than an async script: the
 * embedded server's execute endpoint does not await a returned promise, so
 * the answer is parked on window and read back once it lands.
 */
async function replayOutcome(id: string): Promise<string> {
  await browser.execute((command: string) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w = window as any;
    w.__e2eMenuReplay = null;
    w.__TAURI__.core
      .invoke("replay_menu_command", { id: command })
      .then(() => (w.__e2eMenuReplay = "ok"))
      .catch((reason: unknown) => (w.__e2eMenuReplay = String(reason)));
  }, id);
  await browser.waitUntil(
    async () =>
      browser.execute(
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        () => typeof (window as any).__e2eMenuReplay === "string"
      ),
    { timeoutMsg: `replaying ${id} never answered` }
  );
  return browser.execute(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    () => String((window as any).__e2eMenuReplay)
  );
}

/** Replays one menu command, failing the spec if the desktop side refused. */
async function replayMenuCommand(id: string): Promise<void> {
  expect(await replayOutcome(id)).toBe("ok");
}
