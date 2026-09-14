import type { Column, FrameObject } from "./types";

export function normalizedKeyName(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "")
    .replace(/identifier$/, "id");
}

export function joinKeysCompatible(
  left: Column | undefined,
  right: Column | undefined
): boolean {
  if (!left || !right) return false;
  const numeric = ["integer", "number", "currency", "accounting", "percentage"];
  const text = ["string", "categorical"];
  return (
    left.dataType === right.dataType ||
    (numeric.includes(left.dataType) && numeric.includes(right.dataType)) ||
    (text.includes(left.dataType) && text.includes(right.dataType))
  );
}

export function suggestedLookupKey(
  primaryKey: Column | undefined,
  lookup: FrameObject
): string {
  const compatible = lookup.columns.filter((column) =>
    joinKeysCompatible(primaryKey, column)
  );
  const unique = lookup.uniqueKeys.flatMap((key) =>
    key.columnIds.length === 1 ? key.columnIds : []
  );
  const sameName = (column: Column) =>
    normalizedKeyName(column.name) === normalizedKeyName(primaryKey?.name ?? "");
  return (
    compatible.find((column) => sameName(column) && unique.includes(column.id))?.id ??
    unique.find((id) => compatible.some((column) => column.id === id)) ??
    compatible.find(sameName)?.id ??
    compatible[0]?.id ??
    lookup.columns[0]?.id ??
    ""
  );
}

export function sameNamedKeyId(columns: Column[], name: string): string | undefined {
  return columns.find(
    (column) => normalizedKeyName(column.name) === normalizedKeyName(name)
  )?.id;
}
