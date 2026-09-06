import { useEffect } from "react";

/**
 * While the shortcut modifier is held down and nothing else has been pressed,
 * every control that owns a keyboard shortcut wears it.
 *
 * The whole mechanism is one attribute on `<html>`: `data-modifier-hints`.
 * Controls opt in with `data-shortcut="⌥⌘B"`, and one rule in `styles.css`
 * draws the badge as an absolutely positioned `::before`, so a hint costs no
 * layout and no React render. Putting the state in the DOM rather than in
 * context is what keeps it that way — a boolean in React state would re-render
 * the whole canvas twice per ⌘ press.
 *
 * Which physical key counts is the same question `applicationShortcut` asks:
 * it accepts `metaKey || ctrlKey` rather than sniffing the platform, so both
 * Meta and Control raise hints here too.
 *
 * The four ways hints are taken down all matter, and the reason is macOS. A
 * ⌘-combo that fires a system action (⌘Tab, ⌘H, a native menu item) can steal
 * the keyup, so waiting for Meta's own keyup would leave badges painted over
 * a window nobody is holding a modifier at. Window blur and visibility both
 * catch that. The fourth — any other key going down — is what stops ⌘Z from
 * leaving the hints stuck, and it is also the literal reading of the rule:
 * hints are for a modifier held on its own, before a shortcut is chosen.
 */
export const MODIFIER_HINTS_ATTRIBUTE = "data-modifier-hints";

const isHintModifier = (key: string) => key === "Meta" || key === "Control";

export function useModifierHints() {
  useEffect(() => {
    const root = window.document.documentElement;
    const show = () => root.setAttribute(MODIFIER_HINTS_ATTRIBUTE, "");
    const clear = () => root.removeAttribute(MODIFIER_HINTS_ATTRIBUTE);

    const onKeyDown = (event: KeyboardEvent) => {
      if (isHintModifier(event.key)) show();
      else clear();
    };
    const onKeyUp = (event: KeyboardEvent) => {
      if (isHintModifier(event.key)) clear();
    };

    // Capture, so an editor that stops a keydown from bubbling cannot leave
    // the badges up: the hint is a property of the window, not of the field
    // that happens to have focus.
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    window.addEventListener("blur", clear);
    window.document.addEventListener("visibilitychange", clear);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
      window.removeEventListener("blur", clear);
      window.document.removeEventListener("visibilitychange", clear);
      clear();
    };
  }, []);
}
