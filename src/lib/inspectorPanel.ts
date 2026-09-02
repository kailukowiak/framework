import type { SetStateAction } from "react";
import type { InspectorSection } from "../Inspector";

export type InspectorPanel = { hidden: boolean; section: InspectorSection };

/**
 * Either a section request — the shape every existing caller already sends
 * — or a panel gesture. Keeping both in one action type is what lets the
 * dispatch stand in for the old section setter unchanged.
 */
export type InspectorPanelAction =
  | SetStateAction<InspectorSection>
  | { panel: "toggle" | "hide" };

/**
 * Hidden is "until asked for": every explicit request for a section —
 * ⌘1/2/3, Formula here, a Wrangle prompt — brings the panel back, so a
 * hidden inspector can never swallow the editor a gesture just opened.
 */
export function inspectorPanelReducer(
  state: InspectorPanel,
  action: InspectorPanelAction
): InspectorPanel {
  if (typeof action === "object")
    return { ...state, hidden: action.panel === "toggle" ? !state.hidden : true };
  const section = typeof action === "function" ? action(state.section) : action;
  return { hidden: false, section };
}

/** The `inspector-*` application shortcuts, as panel actions. */
export function inspectorShortcutAction(shortcut: string): InspectorPanelAction {
  return shortcut === "inspector-toggle"
    ? { panel: "toggle" }
    : (shortcut.replace("inspector-", "") as InspectorSection);
}

/**
 * One answer to "is the inspector there", used by the panel and by the
 * canvas that makes room for it. Two answers is how you get a 360px strip
 * of nothing: the canvas insetting for a panel that decided not to draw.
 */
export function inspectorShown(
  panel: InspectorPanel,
  selected: { kind: string } | null | undefined,
  selection: unknown
): boolean {
  return !panel.hidden && inspectorAvailable(selected, selection);
}

/** Whether the current selection has anything the inspector can show. */
export function inspectorAvailable(
  selected: { kind: string } | null | undefined,
  selection: unknown
): boolean {
  return Boolean(selected && selection && selected.kind !== "block");
}
