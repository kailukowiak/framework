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
