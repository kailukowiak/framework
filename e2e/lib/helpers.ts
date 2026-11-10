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
  rowIndex: number,
  // Shift-press is how a person drags out a rectangle without dragging:
  // the grid reads `shiftKey` off the press itself (`beginCellSelection` in
  // FrameCard.tsx) and keeps the earlier cell as the anchor.
  { shift = false }: { shift?: boolean } = {}
): Promise<void> {
  await browser.execute(
    (name: string, index: number, shiftKey: boolean) => {
      const header = document
        .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
        ?.closest("th");
      const columnId = header?.getAttribute("data-column-id");
      if (!columnId) throw new Error(`no column header for ${name}`);
      const cell = Array.from(
        document.querySelectorAll<HTMLElement>(`td[data-column-id="${columnId}"]`)
      )[index];
      if (!cell) throw new Error(`no ${name} cell at row ${index}`);
      const press = { bubbles: true, cancelable: true, button: 0, shiftKey };
      cell.dispatchEvent(new PointerEvent("pointerdown", press));
      cell.dispatchEvent(new PointerEvent("pointerup", press));
    },
    columnName,
    rowIndex,
    shift
  );
}

/**
 * `pointAtColumnCell`, then waits for the grid to have taken the cell as its
 * keyboard focus. Use it when the next thing sent is a key meant for that
 * cell (F2, a typed character).
 *
 * A key is the grid's to handle only while a cell holds grid focus, which is
 * a render after the press; the `cell-focus` class is that state made
 * visible, so this waits for it rather than hoping the round trip was long
 * enough. Not folded into `pointAtColumnCell` itself because, while a formula
 * editor is active, the same press is formula-pointing and never focuses the
 * cell.
 */
export async function focusColumnCell(columnName: string, rowIndex: number): Promise<void> {
  await pointAtColumnCell(columnName, rowIndex);
  await browser.waitUntil(
    () =>
      browser.execute(
        (name: string, index: number) => {
          const columnId = document
            .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
            ?.closest("th")
            ?.getAttribute("data-column-id");
          const cells = document.querySelectorAll<HTMLElement>(
            `td[data-column-id="${columnId}"]`
          );
          return cells[index]?.classList.contains("cell-focus") ?? false;
        },
        columnName,
        rowIndex
      ),
    { timeoutMsg: `the grid never focused the ${columnName} cell at row ${rowIndex}` }
  );
}

/**
 * The rendered text of one column's *record* cells, top to bottom: the cells
 * of `tr[data-row-id]` rows only. `columnCellTexts` also reads the new-row
 * line under the last record (its `td` carries the column id too, holding
 * an input whose text is empty), which is harmless when a spec indexes from
 * the top and wrong when it counts rows — and counting rows is how the
 * grid-growth specs see a row arrive or leave.
 */
export async function recordCellTexts(columnName: string): Promise<string[]> {
  return browser.execute((name: string) => {
    const header = document
      .querySelector<HTMLElement>(`button[aria-label="Sort by ${name}"]`)
      ?.closest("th");
    const columnId = header?.getAttribute("data-column-id");
    if (!columnId) throw new Error(`no column header for ${name}`);
    const table = header!.closest("table");
    return Array.from(
      table?.querySelectorAll<HTMLElement>(
        `tbody tr[data-row-id] td[data-column-id="${columnId}"]`
      ) ?? []
    ).map((cell) => cell.textContent?.trim() ?? "");
  }, columnName);
}

/** The column names of the canvas's first frame, left to right, as drawn. */
export async function frameHeaderNames(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(
      document
        .querySelector<HTMLElement>(".frame-object")
        ?.querySelectorAll<HTMLElement>('button[aria-label^="Sort by "]') ?? []
    ).map((button) => button.getAttribute("aria-label")!.replace("Sort by ", ""))
  );
}

/**
 * Sends one keydown, with exactly the modifier flags and `code` given, to
 * the element holding the keyboard (the grid's clipboard target while a
 * cell is selected), from where it bubbles to the window-level dispatcher in
 * App.tsx. In-page for the reason `extendSelectionDown` gives — the driver's
 * modifier-plus-key actions do not reliably set the event's modifier flags —
 * and because the structure chords are matched on `event.code`
 * (`structureChord` in src/lib/gridEdgeGrowth.ts): ⌥ turns `=` into `≠` on a
 * Mac keyboard, so only the physical key name says which chord it was. The
 * handler that runs is the application's own.
 */
export async function pressGridKey(init: KeyboardEventInit): Promise<void> {
  await browser.execute((eventInit: KeyboardEventInit) => {
    (document.activeElement ?? window).dispatchEvent(
      new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...eventInit })
    );
  }, init);
}

/**
 * Pastes `text` where the keyboard is, as the Edit menu's Paste would: a
 * ClipboardEvent on the focused element (see grid-clipboard.e2e.ts for why
 * the menu-less e2e shell dispatches it by hand). With a cell selected that
 * is the grid's clipboard target; with nothing selected, the canvas.
 */
export async function pasteText(text: string): Promise<void> {
  await browser.execute((value: string) => {
    const clipboard = new DataTransfer();
    clipboard.setData("text/plain", value);
    (document.activeElement ?? window).dispatchEvent(
      new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true })
    );
  }, text);
}

/**
 * Opens the context menu on one record cell, found by column name and row
 * index the way `pointAtColumnCell` finds it. The menu reads the row from
 * the cell's `tr[data-row-id]` and the column from its `data-column-id`
 * (`contextMenuAt` in src/hooks/useContextMenu.ts), so this is the same
 * question a right-click on that cell asks.
 */
export async function openContextMenuOnColumnCell(
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
        document.querySelectorAll<HTMLElement>(
          `tbody tr[data-row-id] td[data-column-id="${columnId}"]`
        )
      )[index];
      if (!cell) throw new Error(`no ${name} cell at row ${index}`);
      cell.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    },
    columnName,
    rowIndex
  );
}

/**
 * A selector for the canvas card that shows the frame called `name`. A card
 * holding several tabs names each one on a `[role="tab"]` (titled with the
 * tab's name, or with its filter count when it has filters); a card holding
 * one frame folds its tab strip away and names it only in the title row's
 * `Frame name` field (`FrameViewTabs.tsx`, the `folded` comment). Either
 * shape is a person's way of finding the card, so either is accepted.
 *
 * Keyed by the card's view, not its object: a card's `data-object-id` is
 * its *active tab's* object (App.tsx draws each view from `view.objectId`),
 * so it changes the moment another tab is brought forward, while the view
 * is the card itself.
 */
export async function cardOfFrame(name: string): Promise<string> {
  const id = await browser.execute((frameName: string) => {
    const tab = Array.from(document.querySelectorAll<HTMLElement>('[role="tab"]')).find(
      (element) =>
        element.getAttribute("title") === frameName ||
        element.textContent?.trim() === frameName
    );
    const field = Array.from(
      document.querySelectorAll<HTMLInputElement>('input[aria-label="Frame name"]')
    ).find((input) => input.value === frameName);
    return (tab ?? field)?.closest("[data-view-id]")?.getAttribute("data-view-id") ?? null;
  }, name);
  if (!id) throw new Error(`no card shows the frame ${name}`);
  return `[data-view-id="${id}"]`;
}

/**
 * The first frame's saved Wrangle chain, as the Rust store holds it — the
 * answer to "did that gesture write a step", which the grid alone cannot
 * give (a draft line is drawn in the chain before it is saved).
 */
export async function savedFrameSteps(): Promise<Array<{ kind: string }>> {
  const view = JSON.parse(await documentJson()) as {
    objects: Array<{ kind: string; steps?: Array<{ kind: string }> }>;
  };
  return view.objects.find((object) => object.kind === "frame")?.steps ?? [];
}

/**
 * A fresh scratch canvas with one hand-entered frame on it, made the way a
 * person makes one without a dialog: paste tab-separated text onto the bare
 * canvas. The first line becomes the headers. The frame is document-owned,
 * with no chain, so its rows and columns both take hand edits.
 */
export async function pasteFrameOntoScratchCanvas(
  text: string,
  firstHeader: string
): Promise<void> {
  await $('[aria-label="Free-form data canvas"]').waitForExist();
  await closeDataLibrary();
  await pressAndRelease('[aria-label="Free-form data canvas"]');
  await pasteText(text);
  await $(`button[aria-label="Sort by ${firstHeader}"]`).waitForExist();
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
