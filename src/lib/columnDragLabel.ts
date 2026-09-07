/**
 * The floating text that follows a header drag.
 *
 * Dragging a header is the gesture that both rearranges a column and starts
 * a lookup between two tables, and until this existed it drew nothing at
 * all: the pointer moved over a foreign header and the only way to learn
 * what a release would do was to release. A drag that says what it is
 * costs one line of text — the column being carried and the verb — so it
 * is text, positioned beside the pointer, and not a card.
 *
 * The vector drag has its own richer preview (it carries values, and their
 * count is the thing worth knowing); this one carries columns.
 */
export type ColumnDragAction = "join" | "reorder" | null;

/** The names being carried, abbreviated the way a header run reads. */
export function columnDragLabelText(names: string[]): string {
  if (names.length === 0) return "column";
  if (names.length === 1) return names[0];
  return `${names[0]} +${names.length - 1}`;
}

/** What releasing here would do, in the words the resulting dialog uses. */
export function columnDragActionText(action: ColumnDragAction): string {
  if (action === "join") return "Match to this column";
  if (action === "reorder") return "Move here";
  return "Drop on a column";
}

export function updateColumnDragLabel(
  label: HTMLElement | null,
  names: string[],
  action: ColumnDragAction,
  pointer: Pick<PointerEvent, "clientX" | "clientY">
): HTMLElement {
  const next = label ?? window.document.createElement("div");
  if (!label) {
    next.className = "column-drag-label";
    next.setAttribute("role", "presentation");
    window.document.body.append(next);
  }
  next.replaceChildren(
    Object.assign(window.document.createElement("strong"), {
      textContent: columnDragLabelText(names),
    }),
    Object.assign(window.document.createElement("span"), {
      textContent: columnDragActionText(action),
    })
  );
  next.style.translate = `${pointer.clientX + 14}px ${pointer.clientY + 14}px`;
  next.dataset.target = action ? "true" : "false";
  return next;
}
