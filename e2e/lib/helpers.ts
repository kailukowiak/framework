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
