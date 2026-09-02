import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import {
  columnCellTexts,
  extendSelectionDown,
  pointAtColumnCell,
  pressAndRelease,
  resetAndOpenTutorial,
} from "../lib/helpers";

// Cut/Copy/Paste reach the grid through Tauri's predefined Edit-menu roles,
// which raise real DOM ClipboardEvents rather than calling any command this
// harness could invoke directly. E2e builds are menu-less (no NSMenu to
// raise them from), so this spec dispatches the same events by hand — on
// `document.activeElement`, which the app keeps pinned to the hidden
// `gridClipboardTarget` textarea for exactly this reason (see
// `src/lib/gridClipboardTarget.ts`) — which is exactly the surface a real
// Cut/Copy/Paste menu click reaches. Everything downstream — the window
// listeners in App.tsx, `useGridClipboard`'s copy/cut/paste handlers, and
// `useCanvasClipboard`'s bare-canvas paste — is the real application.
//
// The Month-over-month formulas by pointing — Start workbook is the
// fixture: its Monthly sales frame is document-owned with no display sort,
// so Revenue and Cost are plain, editable, literal columns and the six rows
// render in a known, stable order (row 1 is 2026-01/East/118000/76000, row 2
// 2026-06/East/168000/104000, row 3 2026-03/East/136000/85000).
describe("grid clipboard", () => {
  it("opens the Start workbook", async () => {
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();
  });

  it("copies a three-cell selection as newline-joined text", async () => {
    await pointAtColumnCell("Revenue", 1);
    await extendSelectionDown();
    await extendSelectionDown();

    const copied = await browser.execute(() => {
      const w = window as any; // eslint-disable-line @typescript-eslint/no-explicit-any
      const dt = new DataTransfer();
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new ClipboardEvent("copy", { clipboardData: dt, bubbles: true, cancelable: true })
      );
      w.__e2eClipboard = dt;
      return dt.getData("text/plain");
    });

    expect(copied).toBe("118000\n168000\n136000");
  });

  it("pastes the copied text into another column and keeps the clipboard target focused", async () => {
    await pointAtColumnCell("Cost", 1); // the paste anchor

    await browser.execute(() => {
      const w = window as any; // eslint-disable-line @typescript-eslint/no-explicit-any
      const source: DataTransfer = w.__e2eClipboard;
      const dt = new DataTransfer();
      dt.setData("text/plain", source.getData("text/plain"));
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true })
      );
    });

    await browser.waitUntil(
      async () => {
        const cost = await columnCellTexts("Cost");
        return (
          cost[1]?.replace(/[^\d.-]/g, "") === "118000" &&
          cost[2]?.replace(/[^\d.-]/g, "") === "168000" &&
          cost[3]?.replace(/[^\d.-]/g, "") === "136000"
        );
      },
      { timeoutMsg: "the pasted Revenue values never landed in Cost" }
    );

    // The native Cut/Copy/Paste menu items only apply while the hidden
    // clipboard target holds the keyboard focus — losing it after a paste
    // would silently break the next Cut/Copy the person tries.
    const activeIsClipboardTarget = await browser.execute(() =>
      document.activeElement?.hasAttribute("data-framework-grid-clipboard")
    );
    expect(activeIsClipboardTarget).toBe(true);
  });

  it("cuts the pasted cells back out", async () => {
    await pointAtColumnCell("Cost", 1); // the values just pasted in
    await extendSelectionDown();
    await extendSelectionDown();

    const cut = await browser.execute(() => {
      const dt = new DataTransfer();
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new ClipboardEvent("cut", { clipboardData: dt, bubbles: true, cancelable: true })
      );
      return dt.getData("text/plain");
    });
    expect(cut).toBe("118000\n168000\n136000");

    // An emptied integer cell renders "—" (displayedColumnFormat gives every
    // integer column an implicit number format, and FormattedCellValue falls
    // back to an em dash when there is no value) rather than blank text —
    // the same placeholder an untouched blank cell shows, so this is reading
    // the cell as genuinely cleared, not just non-numeric.
    await browser.waitUntil(
      async () => {
        const cost = await columnCellTexts("Cost");
        return cost[1] === "—" && cost[2] === "—" && cost[3] === "—";
      },
      { timeoutMsg: "the cut cells in Cost never cleared" }
    );
  });

  it("pastes onto the bare canvas as a new frame", async () => {
    // Escape hands the keyboard back from the grid (setGridFocus(null) in
    // useGridKeyboardNavigation) before the bare-canvas click can register
    // as one: `useCanvasClipboard` only arms once gridFocus is null.
    await browser.keys(Key.Escape);
    await pressAndRelease('[aria-label="Free-form data canvas"]');

    await browser.execute(() => {
      const dt = new DataTransfer();
      dt.setData("text/plain", "Product\tUnits\nWidget\t10\nGadget\t20");
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true })
      );
    });

    await $('button[aria-label="Sort by Product"]').waitForExist();
    await $('button[aria-label="Sort by Units"]').waitForExist();
  });
});
