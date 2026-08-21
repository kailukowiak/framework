const LIMIT = 5;
const PREFIX = "framework.document-colors";

export type ColorProperty = "text" | "fill";

/** Newest first, unique, and short enough to remain one compact row. */
export function withRecentColor(colors: string[], color: string): string[] {
  const normalized = color.toLowerCase();
  return [
    normalized,
    ...colors.filter((candidate) => candidate.toLowerCase() !== normalized),
  ].slice(0, LIMIT);
}

function key(documentId: string, property: ColorProperty): string {
  return `${PREFIX}.${documentId}.${property}`;
}

export function readRecentColors(
  documentId: string,
  property: ColorProperty
): string[] {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(key(documentId, property)) ?? "[]");
    if (!Array.isArray(parsed)) return [];
    const valid = parsed
      .filter(
        (color): color is string =>
          typeof color === "string" && /^#[0-9a-f]{6}$/i.test(color)
      )
      .map((color) => color.toLowerCase());
    return [...new Set(valid)].slice(0, LIMIT);
  } catch {
    return [];
  }
}

export function writeRecentColors(
  documentId: string,
  property: ColorProperty,
  colors: string[]
): void {
  try {
    window.localStorage.setItem(key(documentId, property), JSON.stringify(colors));
  } catch {
    // The current window still remembers through React state; only the next
    // launch loses the row when storage is unavailable.
  }
}
