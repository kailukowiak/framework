import type { SetStateAction } from "react";
import type { InspectorSection } from "../Inspector";

export type InspectorPanel = { hidden: boolean; section: InspectorSection };
export const INITIAL_INSPECTOR_PANEL: InspectorPanel = { hidden: true, section: "selection" };

/**
 * Either a section request — the shape every existing caller already sends
 * — or a panel gesture. Keeping both in one action type is what lets the
 * dispatch stand in for the old section setter unchanged.
 */
export type InspectorPanelAction =
  | SetStateAction<InspectorSection>
  | { panel: "toggle" | "hide" }
  | { panel: "prepare"; section: InspectorSection };

/**
 * Hidden is "until asked for": every explicit request for a section —
 * ⌘1/2/3, Formula here, a Wrangle prompt — brings the panel back, so a
 * hidden inspector can never swallow the editor a gesture just opened.
 */
export function inspectorPanelReducer(
  state: InspectorPanel,
  action: InspectorPanelAction
): InspectorPanel {
  if (typeof action === "object") {
    // Grid edits use the top bar. Prepare its owning section without making
    // the inspector visible; an explicit section request still opens it.
    if (action.panel === "prepare") return { ...state, section: action.section };
    return { ...state, hidden: action.panel === "toggle" ? !state.hidden : true };
  }
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
 * One answer to "is the inspector there", used by the panel and the status
 * controls that move out of its way. The canvas itself stays full width so
 * opening the inspector does not change the space available to fit a frame.
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
