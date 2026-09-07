import type { FrameObject } from "./types";

/** A dictionary is a table with one key and one value, so switching to
 * dictionary use does not copy data or give it a second persistence model. */
export function dictionaryColumns(frame: FrameObject) {
  if (frame.columns.length !== 2) return null;
  const keyId = frame.uniqueKeys.find((key) => key.columnIds.length === 1)?.columnIds[0];
  const key = frame.columns.find((column) => column.id === keyId);
  const value = frame.columns.find((column) => column.id !== keyId);
  return key && value ? { key, value } : null;
}
