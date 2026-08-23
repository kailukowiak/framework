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

  const clearMark = () => {
    marked?.classList.remove("vector-column-drop");
    marked = null;
  };
  const targetAt = (x: number, y: number) => {
    const under = document.elementFromPoint(x, y);
    const tableTarget = under?.closest<HTMLElement>(
      ".column-header, .frame-edge-header"
    );
    if (tableTarget) return tableTarget;
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
    if (target?.matches(".column-header, .frame-edge-header")) {
      target.classList.add("vector-column-drop");
      marked = target;
    }
  };
  const cleanup = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", end);
    window.removeEventListener("pointercancel", cancel);
    source?.classList.remove("vector-dragging");
    clearMark();
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

function dispatchVectorDrop(
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
