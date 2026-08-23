const VECTOR_MIME = "application/x-framework-vector";

export type VectorDrag = {
  objectId: string;
  formula: string;
  name: string;
  length: number;
};

export function writeVectorDrag(transfer: DataTransfer, payload: VectorDrag) {
  transfer.effectAllowed = "link";
  transfer.setData(VECTOR_MIME, JSON.stringify(payload));
  // Gives the operating system something intelligible if the drag leaves
  // FrameWork without turning the in-app gesture into a text paste.
  transfer.setData("text/plain", payload.formula);
}

export function readVectorDrag(transfer: DataTransfer): VectorDrag | null {
  const raw = transfer.getData(VECTOR_MIME);
  if (!raw) return null;
  try {
    const value = JSON.parse(raw) as Partial<VectorDrag>;
    return typeof value.objectId === "string" &&
      typeof value.formula === "string" &&
      typeof value.name === "string" &&
      Number.isInteger(value.length) &&
      (value.length ?? 0) > 0
      ? (value as VectorDrag)
      : null;
  } catch {
    return null;
  }
}

export function hasVectorDrag(transfer: DataTransfer): boolean {
  return Array.from(transfer.types).includes(VECTOR_MIME);
}

/**
 * Native HTML drag and drop does not make a reliable round trip through the
 * macOS webview. It advertises the drag to AppKit -- hence the green plus --
 * but may never deliver the matching drop event back to React. Tabs use a
 * pointer gesture for the same reason. Vectors follow that lane too, then
 * hand the completed gesture to the existing drop targets as an ordinary
 * in-app event so there is still one implementation of what a drop means.
 */
export function beginVectorPointerDrag(event: PointerEvent, payload: VectorDrag) {
  if (event.button !== 0 || payload.length < 1) return;
  event.preventDefault();
  event.stopPropagation();
  const source = event.currentTarget as HTMLElement | null;
  const start = { x: event.clientX, y: event.clientY };
  let moved = false;
  let marked: HTMLElement | null = null;
  let markedEdge: HTMLTableElement | null = null;
  let preview: HTMLElement | null = null;

  const clearMark = () => {
    marked?.classList.remove("vector-column-drop");
    markedEdge?.classList.remove("vector-edge-drop");
    marked = null;
    markedEdge = null;
  };
  const removePreview = () => {
    preview?.remove();
    preview = null;
  };
  const targetAt = (x: number, y: number) => {
    const under = document.elementFromPoint(x, y);
    const declaredTarget = under?.closest<HTMLElement>("[data-vector-drop-target]");
    if (declaredTarget) return declaredTarget;
    const tableTarget = under?.closest<HTMLElement>(
      ".column-header, .frame-edge-header"
    );
    if (tableTarget) return tableTarget;
    // The visible right edge runs down the full table, while its drop handler
    // lives on the header. Treat every edge cell as that same target. Requiring
    // a precise hit on the 26px header made the tutorial's "drop on the +
    // edge" gesture look broken anywhere below the first row.
    const edgeCell = under?.closest<HTMLElement>(".frame-edge-cell");
    const edgeHeader = edgeCell
      ?.closest("table")
      ?.querySelector<HTMLElement>(".frame-edge-header");
    if (edgeHeader) return edgeHeader;
    const viewport = under?.closest<HTMLElement>(".canvas-viewport");
    return viewport && !under?.closest(".canvas-object") ? viewport : null;
  };
  const move = (moveEvent: globalThis.PointerEvent) => {
    if (
      !moved &&
      Math.hypot(moveEvent.clientX - start.x, moveEvent.clientY - start.y) < 3
    )
      return;
    moved = true;
    moveEvent.preventDefault();
    source?.classList.add("vector-dragging");
    clearMark();
    const target = targetAt(moveEvent.clientX, moveEvent.clientY);
    if (target?.matches(".column-header, .frame-edge-header, [data-vector-drop-target]")) {
      target.classList.add("vector-column-drop");
      marked = target;
      if (target.matches(".frame-edge-header")) {
        markedEdge = target.closest("table");
        markedEdge?.classList.add("vector-edge-drop");
      }
    }
    preview = updateVectorDragPreview(preview, payload, moveEvent, target);
  };
  const cleanup = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", end);
    window.removeEventListener("pointercancel", cancel);
    source?.classList.remove("vector-dragging");
    clearMark();
    removePreview();
  };
  const end = (upEvent: globalThis.PointerEvent) => {
    const target = moved ? targetAt(upEvent.clientX, upEvent.clientY) : null;
    cleanup();
    if (target) dispatchVectorDrop(target, payload, upEvent);
  };
  const cancel = () => cleanup();
  window.addEventListener("pointermove", move, { passive: false });
  window.addEventListener("pointerup", end);
  window.addEventListener("pointercancel", cancel);
}

export function updateVectorDragPreview(
  preview: HTMLElement | null,
  payload: VectorDrag,
  pointer: Pick<PointerEvent, "clientX" | "clientY">,
  target: HTMLElement | null
): HTMLElement {
  const next = preview ?? document.createElement("div");
  if (!preview) {
    next.className = "vector-drag-preview";
    next.setAttribute("role", "presentation");
    document.body.append(next);
  }
  const noun = payload.length === 1 ? "value" : "values";
  next.replaceChildren(
    Object.assign(document.createElement("strong"), { textContent: payload.name }),
    Object.assign(document.createElement("span"), {
      textContent: `${payload.length} ${noun} · ${vectorDragAction(target, payload.length)}`,
    })
  );
  next.style.translate = `${pointer.clientX + 14}px ${pointer.clientY + 14}px`;
  next.dataset.target = target ? "true" : "false";
  return next;
}

function vectorDragAction(target: HTMLElement | null, vectorLength: number) {
  return target?.dataset.vectorDropAction ?? standardVectorDragAction(target, vectorLength);
}

function standardVectorDragAction(target: HTMLElement | null, vectorLength: number) {
  if (target?.dataset.vectorCombine === "true") return "Choose layout";
  const selectedColumns = Number(target?.dataset.vectorSelectedColumns ?? 0);
  const columnsAfter = Number(target?.dataset.vectorColumnsAfter ?? 0);
  const targetColumns =
    selectedColumns > 0 && selectedColumns % vectorLength === 0
      ? selectedColumns
      : Math.min(columnsAfter, vectorLength);
  if (
    target?.matches(".frame-edge-header") ||
    (target?.matches(".column-header") && targetColumns === 1 && vectorLength > 1)
  )
    return "Add column";
  if (target?.matches(".column-header")) return "Use in columns";
  if (target?.matches(".canvas-viewport")) return "New table";
  return "Drag to a table or canvas";
}

/** A small DataTransfer-shaped envelope for the existing React drop handlers. */
function vectorTransfer(payload: VectorDrag): DataTransfer {
  const values = new Map<string, string>();
  const transfer = {
    dropEffect: "none",
    effectAllowed: "link",
    get types() {
      return Array.from(values.keys());
    },
    clearData(type?: string) {
      if (type) values.delete(type);
      else values.clear();
    },
    getData(type: string) {
      return values.get(type) ?? "";
    },
    setData(type: string, value: string) {
      values.set(type, value);
    },
    setDragImage() {},
  } as unknown as DataTransfer;
  writeVectorDrag(transfer, payload);
  return transfer;
}

export function dispatchVectorDrop(
  target: HTMLElement,
  payload: VectorDrag,
  pointer: PointerEvent
) {
  const drop = new Event("drop", { bubbles: true, cancelable: true });
  Object.defineProperties(drop, {
    clientX: { value: pointer.clientX },
    clientY: { value: pointer.clientY },
    dataTransfer: { value: vectorTransfer(payload) },
  });
  target.dispatchEvent(drop);
}
