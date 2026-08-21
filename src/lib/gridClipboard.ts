import type { FrameObject } from "./types";

export function serializeGrid(grid: string[][]): string {
  return grid
    .map((row) =>
      row
        .map((value) => {
          if (!/[\t\n\r"]/.test(value)) return value;
          return `"${value.replace(/"/g, '""')}"`;
        })
        .join("\t")
    )
    .join("\n");
}

/**
 * Whether ordinary text is selected somewhere on the page.
 *
 * The grid is `user-select: none`, so a live selection is never cells — it
 * is an error message, a hint, a name in the sidebar, something somebody
 * highlighted in order to quote it. Copying that is the browser's job.
 *
 * The grid has to ask because it keeps its focus across a click into the
 * sidebar: without this, every ⌘C anywhere in the app answered with cells,
 * which is worst exactly where it matters most — an error you are trying
 * to paste to somebody.
 */
export function hasTextSelection(): boolean {
  const selected = window.getSelection();
  return Boolean(selected && !selected.isCollapsed && selected.toString().trim());
}

/**
 * A frame with nothing in it yet — the 2×2 a "new frame" starts as.
 *
 * Pasting into one replaces it outright rather than writing into its two
 * columns, which is what makes "new frame, paste" the whole import flow for
 * a block of copied cells. A frame with anything in it keeps its shape.
 */
export function isEmptyLiteralFrame(frame: FrameObject): boolean {
  if (frame.derivation || frame.artifact || frame.sourceFile) return false;
  if (frame.columns.some((column) => column.formula)) return false;
  return frame.rows.every((row) =>
    frame.columns.every((column) => !(row.cells[column.id]?.raw ?? "").trim())
  );
}
