import type { FrameObject } from "../lib/types";

/** One ordered surface makes import mapping explicit without a form per feature. */
export function resolveModelColumns(frame: FrameObject | undefined, text: string): string[] {
  if (!frame) throw new Error("Choose a source frame.");
  const names = text.split("\n").map(name => name.trim()).filter(Boolean);
  if (!names.length) throw new Error("Choose at least one feature column.");
  if (new Set(names).size !== names.length) throw new Error("Each feature column must appear once.");
  return names.map(name => {
    const matches = frame.columns.filter(column => column.name === name);
    if (matches.length !== 1) throw new Error(`Feature “${name}” must match one column in ${frame.name}.`);
    return matches[0].id;
  });
}

export function modelColumnNames(frame: FrameObject | undefined, ids: string[]): string {
  return ids.map(id => frame?.columns.find(column => column.id === id)?.name ?? "").join("\n");
}
