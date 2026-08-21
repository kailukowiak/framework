import { useEffect, useState } from "react";
import {
  paletteColorName,
  usePrefersDarkMode,
  storedColor,
  themedColor,
} from "./lib/palette";
import {
  readRecentColors,
  withRecentColor,
  writeRecentColors,
  type ColorProperty,
} from "./lib/recentColors";

/**
 * One row of the Format inspector: a named color, the palette it can be
 * picked from, and a way out to any other color.
 *
 * The swatches paint themselves the way the cell will paint — through the
 * same `light-dark()` pair — so what is on the button is what lands on the
 * data. Without that the grid would be honest about dark mode and the
 * control choosing its colors would not, which is the worse half of the two
 * to get wrong: a person picks from what they can see.
 *
 * The document keeps one hex per color, always the light-mode one, so a pick
 * made while the window is dark is written down reflected. That is what
 * makes the swatch somebody clicked and the color that appears the same
 * color in either theme, off one stored value.
 */
export function FormatColorField({
  label,
  swatches,
  perRow,
  value,
  exact,
  fallback,
  documentId,
  property,
  canReset,
  resetLabel = "Reset",
  onChange,
}: {
  label: string;
  swatches: string[];
  perRow: number;
  /** What the cell shows now, cascade included -- what the well opens on. */
  value: string | null;
  /** What this cell holds itself: nothing to reset when there is none. */
  exact: string | null;
  fallback: string;
  documentId: string;
  property: ColorProperty;
  canReset?: boolean;
  resetLabel?: string;
  onChange: (color: string | null) => void;
}) {
  const dark = usePrefersDarkMode();
  const [recent, setRecent] = useState(() => readRecentColors(documentId, property));
  useEffect(() => {
    setRecent(readRecentColors(documentId, property));
  }, [documentId, property]);
  const choose = (color: string) => {
    // Fixed swatches are already one glance away. The extra row earns its
    // space only for colors that would otherwise be lost in the native well.
    if (!swatches.includes(color)) {
      const next = withRecentColor(recent, color);
      setRecent(next);
      writeRecentColors(documentId, property, next);
    }
    onChange(color);
  };
  const custom = recent.filter((color) => !swatches.includes(color));
  // The native well speaks in the colors on screen, so it is handed the
  // reflection going in and its answer is reflected coming back out.
  const shown = storedColor(value ?? fallback, dark);
  return (
    <div>
      <span>{label}</span>
      <div className="format-color-row">
        <div
          className="format-swatches"
          style={{ gridTemplateColumns: `repeat(${perRow}, 1fr)` }}
        >
          {swatches.map((color) => (
            <button
              key={color}
              className={`format-swatch ${value === color ? "active" : ""}`}
              style={{ backgroundColor: themedColor(color) }}
              aria-label={`Set ${label.toLowerCase()} ${paletteColorName(color) ?? color}`}
              title={`${paletteColorName(color) ?? "Custom"} · ${color}`}
              aria-pressed={value === color}
              onClick={() => choose(color)}
            />
          ))}
        </div>
        <input
          type="color"
          aria-label={`Custom ${label.toLowerCase()}`}
          value={shown}
          onChange={(event) => choose(storedColor(event.target.value, dark))}
        />
        <button
          disabled={canReset === false || exact === null}
          title={canReset === false ? `Keep the only active color scale` : resetLabel}
          onClick={() => onChange(null)}
        >
          {resetLabel}
        </button>
      </div>
      {custom.length > 0 && (
        <div className="format-recent-colors" aria-label={`Recent ${label.toLowerCase()}`}>
          <span>Recent</span>
          {custom.map((color) => (
            <button
              key={color}
              className={`format-swatch ${value === color ? "active" : ""}`}
              style={{ backgroundColor: themedColor(color) }}
              aria-label={`Set ${label.toLowerCase()} recent ${color}`}
              title={`Recent · ${color}`}
              aria-pressed={value === color}
              onClick={() => choose(color)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
