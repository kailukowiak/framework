import { $, browser } from "@wdio/globals";

/**
 * Strips presentation from a rendered value so specs assert the number, not
 * the locale: "839,000" and "839000" are the same answer, and a spec that
 * broke over a thousands separator would be testing the formatter twice.
 */
export const digits = (text: string) => text.replace(/[^\d.-]/g, "");

/**
 * Waits until some answer in a Scratchwork gutter equals `expected`.
 *
 * Scratchwork evaluation is live — there is no Execute gesture to wait
 * behind — so the only honest synchronization is watching the gutter itself.
 * The timeout failing is a real product answer: the formula never produced
 * this value.
 */
export async function waitForGutterAnswer(expected: string): Promise<void> {
  const gutter = $('[aria-label="Scratchwork answers"]');
  await gutter.waitForExist();
  await browser.waitUntil(
    async () => {
      // Per-row textContent via execute, because the embedded server's
      // getText answers "" for this container. Numeric comparison, not
      // string: the gutter renders "40" as "40.00" and larger answers with
      // thousands separators, and the spec is asserting the answer, not
      // the formatter.
      const rows = await browser.execute(() =>
        Array.from(
          document.querySelectorAll('[aria-label="Scratchwork answers"] > *')
        ).map((row) => row.textContent ?? "")
      );
      return rows.map((row) => Number(digits(row))).includes(Number(expected));
    },
    {
      timeoutMsg: `Scratchwork gutter never showed ${expected}. Current: ${await browser
        .execute(
          () =>
            Array.from(
              document.querySelectorAll('[aria-label="Scratchwork answers"] > *')
            )
              .map((row) => row.textContent ?? "")
              .join(" | ") || "<empty>"
        )
        .catch(() => "<gone>")}`,
    }
  );
}

/** Closes the Data library dialog a bare launch opens over the canvas. */
export async function closeDataLibrary(): Promise<void> {
  const dialog = $(".dataset-dialog");
  await dialog.waitForExist();
  await $(".dataset-dialog .dialog-header .icon-button").click();
  await dialog.waitForExist({ reverse: true });
}

/** Opens the one collapsed home for tutorial workbooks and sample documents. */
export async function openTutorialsAndExamples(): Promise<void> {
  const toggle = $("button*=Tutorials and examples");
  await toggle.waitForClickable();
  if ((await toggle.getAttribute("aria-expanded")) !== "true") {
    await toggle.click();
  }
}

/**
 * Resets the tutorial workbooks through the Data library's own two-click
 * confirm and opens the one named. Create first, because Create is
 * idempotent and is what makes Reset appear on a machine that never had
 * the workbooks.
 */
export async function resetAndOpenTutorial(title: string): Promise<void> {
  await $(".dataset-dialog").waitForExist();
  await openTutorialsAndExamples();
  await $("button*=Create tutorials").click();
  await $("button*=Reset tutorials").waitForClickable();
  await $("button*=Reset tutorials").click();
  await $("button*=Replace all tutorial workbooks").click();
  const entry = $(`button*=${title}`);
  await entry.waitForClickable();
  await entry.click();
}

/**
 * Points at a grid cell the way formula-by-pointing does: a pointerdown
 * and pointerup on the cell rendering `cellText`. Dispatched in-page
 * because the embedded driver synthesizes mouse events but never pointer
 * events, and the picking path deliberately listens for pointer events.
 * Both halves of the press matter: a Scratchwork cell pick is a
 * press-drag-release (one cell or a row slice), so the insertion — or the
 * refusal notice — only lands on the release. Everything downstream of the
 * events — the pick, the gate, the insertion, the evaluation — is the real
 * application.
 */
export async function pointAtCell(cellText: string): Promise<void> {
  await browser.execute((text: string) => {
    // A numeric target matches on the number, not the rendering: the same
    // 142000 renders as "142,000" or "$142,000.00" depending on the column's
    // format, and the spec is pointing at the value, not the formatter.
    const digitsOf = (value: string) => value.replace(/[^\d.-]/g, "");
    const wanted = digitsOf(text);
    const cell = Array.from(
      document.querySelectorAll<HTMLElement>("div.cell-display")
    ).find((candidate) => {
      const rendered = (candidate.textContent ?? "").trim();
      if (rendered === text) return true;
      const renderedDigits = digitsOf(rendered);
      return (
        wanted !== "" &&
        renderedDigits !== "" &&
        Number(renderedDigits) === Number(wanted)
      );
    });
    if (!cell) throw new Error(`no cell renders ${text}`);
    cell.dispatchEvent(
      new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
    );
    cell.dispatchEvent(
      new PointerEvent("pointerup", { bubbles: true, cancelable: true, button: 0 })
    );
  }, cellText);
}

/**
 * Selects a canvas card the way a person's press does: a pointerdown on the
 * element matching `selector`. Dispatched in-page because the embedded
 * driver synthesizes mouse events but never pointer events, and card
 * selection deliberately listens for pointerdown — a WebDriver click on a
 * card therefore selects nothing.
 */
export async function selectCard(selector: string): Promise<void> {
  await browser.execute((sel: string) => {
    const card = document.querySelector<HTMLElement>(sel);
    if (!card) throw new Error(`no element matches ${sel}`);
    card.dispatchEvent(
      new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
    );
  }, selector);
}

/**
 * Opens the app's context menu on the element matching `selector` — by
 * dispatching the contextmenu event in-page, because the embedded driver's
 * right-click synthesizes nothing React can see. The menu that opens, and
 * everything chosen from it, is the real interface.
 */
export async function openContextMenuOn(selector: string): Promise<void> {
  await browser.execute((sel: string) => {
    const target = document.querySelector(sel);
    if (!target) throw new Error(`no element matches ${sel}`);
    target.dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true })
    );
  }, selector);
}

/**
 * The block textarea's selector: a named block's own textarea (labelled
 * "<name> lines"), or the document's first block when no name is given. A
 * workbook can hold several blocks — the formula-clicks tutorial ships both
 * "Checks" and "Scratchwork" — so any spec on a multi-block document must
 * name the one it means.
 */
const blockSourceSelector = (blockName?: string) =>
  blockName ? `textarea[aria-label="${blockName} lines"]` : ".block-source";

/**
 * Focuses a block textarea in-page. Formula pointing is gated on an
 * *active* editor, and activation happens in the textarea's onFocus —
 * which `setValue` never fires (it writes the draft without taking DOM
 * focus) and a synthesized click never causes (the driver's clicks run no
 * focus transfer). A person cannot type without focusing, so dispatching
 * the focus is restoring the real precondition, not faking one.
 */
export async function focusBlockSource(blockName?: string): Promise<void> {
  await browser.execute((selector: string) => {
    document.querySelector<HTMLTextAreaElement>(selector)?.focus();
  }, blockSourceSelector(blockName));
}

/** The current block draft, read from the block textarea. */
export async function blockDraft(blockName?: string): Promise<string> {
  return browser.execute(
    (selector: string) =>
      document.querySelector<HTMLTextAreaElement>(selector)?.value ?? "",
    blockSourceSelector(blockName)
  );
}

/**
 * Rendered text of every cell in one grid column, top to bottom — found by
 * the column's own sort header (`Sort by <name>`) rather than by value, so
 * a spec can read what landed at a row without already knowing what to
 * expect there (a fresh paste, a cell a cut just cleared). Literal cells
 * render as `div.cell-display` and calculated ones as the clickable
 * `button.computed-cell`; reading `textContent` off the `td` picks up
 * whichever the cell is showing.
 */
export async function columnCellTexts(columnName: string): Promise<string[]> {
  return browser.execute((name: string) => {
    const header = document
      .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
      ?.closest("th");
    const columnId = header?.getAttribute("data-column-id");
    if (!columnId) throw new Error(`no column header for ${name}`);
    return Array.from(
      document.querySelectorAll<HTMLElement>(`td[data-column-id="${columnId}"]`)
    ).map((cell) => cell.textContent?.trim() ?? "");
  }, columnName);
}

/**
 * Points at a grid cell by column name and row index (0-based, in display
 * order) rather than by rendered value — needed once two columns can hold
 * the same number (a copy pasted into a second column reads identically to
 * its source until one of them changes), where `pointAtCell`'s value match
 * would find whichever comes first in the DOM and not necessarily the one
 * meant. Dispatched in-page for the same reason `pointAtCell` is: the
 * embedded driver never synthesizes pointer events, and cell selection
 * listens for pointerdown on the `td` itself.
 */
export async function pointAtColumnCell(
  columnName: string,
  rowIndex: number
): Promise<void> {
  await browser.execute(
    (name: string, index: number) => {
      const header = document
        .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
        ?.closest("th");
      const columnId = header?.getAttribute("data-column-id");
      if (!columnId) throw new Error(`no column header for ${name}`);
      const cell = Array.from(
        document.querySelectorAll<HTMLElement>(`td[data-column-id="${columnId}"]`)
      )[index];
      if (!cell) throw new Error(`no ${name} cell at row ${index}`);
      cell.dispatchEvent(
        new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
      );
      cell.dispatchEvent(
        new PointerEvent("pointerup", { bubbles: true, cancelable: true, button: 0 })
      );
    },
    columnName,
    rowIndex
  );
}

/**
 * Extends a grid selection by dispatching a native ArrowDown keydown with
 * `shiftKey: true`, in-page, rather than `browser.keys([Key.Shift,
 * Key.ArrowDown])`: the grid reads `event.shiftKey` straight off the
 * KeyboardEvent to decide extend vs. plain move
 * (`useGridKeyboardNavigation.ts`), and the driver's own modifier-plus-key
 * action does not reliably set it — the arrow moves the focus cell but
 * never grows the range. Dispatched on `window` because App.tsx's
 * navigation keydown listener is window-level.
 */
export async function extendSelectionDown(): Promise<void> {
  await browser.execute(() => {
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "ArrowDown",
        shiftKey: true,
        bubbles: true,
        cancelable: true,
      })
    );
  });
}

/**
 * A press-and-release on the element matching `selector`, dispatched
 * in-page for the same reason `pointAtCell`/`selectCard` are: the embedded
 * driver never synthesizes pointer events. Used for the bare-canvas click
 * that clears card selection and re-arms the grid clipboard target — the
 * canvas's own pointerdown/pointerup handler (`App.tsx`) needs both halves,
 * not just the press.
 */
export async function pressAndRelease(selector: string): Promise<void> {
  await browser.execute((sel: string) => {
    const target = document.querySelector<HTMLElement>(sel);
    if (!target) throw new Error(`no element matches ${sel}`);
    target.dispatchEvent(
      new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
    );
    target.dispatchEvent(
      new PointerEvent("pointerup", { bubbles: true, cancelable: true, button: 0 })
    );
  }, selector);
}

/**
 * The document as the Rust store describes it, over the real IPC bridge.
 *
 * Invoke-then-poll rather than an async script, for the reason `smoke.e2e.ts`
 * spells out: the embedded server's execute endpoint does not await a
 * returned promise, so the answer is parked on `window` and read back once it
 * lands. Returned as JSON text rather than an object because the only
 * questions specs ask of it are "is this text in the saved document" and
 * "which document is this" — and a JSON string survives the WebDriver bridge
 * unchanged, where an object graph does not.
 */
export async function documentJson(): Promise<string> {
  await browser.execute(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w = window as any;
    w.__e2eDocumentJson = undefined;
    w.__TAURI__.core
      .invoke("get_document")
      .then((view: unknown) => (w.__e2eDocumentJson = JSON.stringify(view)))
      .catch((reason: unknown) => (w.__e2eDocumentJson = JSON.stringify({ failure: String(reason) })));
  });
  await browser.waitUntil(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    async () => browser.execute(() => (window as any).__e2eDocumentJson !== undefined),
    { timeoutMsg: "get_document never answered" }
  );
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return browser.execute(() => (window as any).__e2eDocumentJson as string);
}
