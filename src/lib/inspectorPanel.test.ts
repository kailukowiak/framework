import { describe, expect, it } from "vitest";
import {
  INITIAL_INSPECTOR_PANEL,
  inspectorAvailable,
  inspectorPanelReducer,
  inspectorShortcutAction,
  inspectorShown,
  type InspectorPanel,
} from "./inspectorPanel";

const shown: InspectorPanel = { hidden: false, section: "selection" };
const hidden: InspectorPanel = { hidden: true, section: "wrangle" };

describe("inspector panel", () => {
  it("starts collapsed and prepares a grid formula without opening the panel", () => {
    expect(INITIAL_INSPECTOR_PANEL.hidden).toBe(true);
    expect(inspectorPanelReducer(INITIAL_INSPECTOR_PANEL, { panel: "prepare", section: "wrangle" }))
      .toEqual({ hidden: true, section: "wrangle" });
    expect(inspectorPanelReducer(shown, { panel: "prepare", section: "wrangle" }))
      .toEqual({ hidden: false, section: "wrangle" });
  });
  it("toggles and hides without forgetting the section", () => {
    expect(inspectorPanelReducer(shown, { panel: "toggle" })).toEqual({
      hidden: true,
      section: "selection",
    });
    expect(inspectorPanelReducer(hidden, { panel: "toggle" })).toEqual({
      hidden: false,
      section: "wrangle",
    });
    expect(inspectorPanelReducer(hidden, { panel: "hide" })).toEqual(hidden);
  });

  it("brings a hidden panel back whenever a section is asked for", () => {
    expect(inspectorPanelReducer(hidden, "format")).toEqual({
      hidden: false,
      section: "format",
    });
    expect(inspectorPanelReducer(hidden, (current) => current)).toEqual({
      hidden: false,
      section: "wrangle",
    });
  });

  it("reads the inspector shortcuts as panel actions", () => {
    expect(inspectorShortcutAction("inspector-toggle")).toEqual({ panel: "toggle" });
    expect(inspectorShortcutAction("inspector-wrangle")).toBe("wrangle");
  });

  it("shows the panel only for a selected non-block object that is not hidden", () => {
    const frame = { kind: "frame" };
    expect(inspectorShown(shown, frame, { objectId: "f" })).toBe(true);
    expect(inspectorShown(hidden, frame, { objectId: "f" })).toBe(false);
    expect(inspectorShown(shown, { kind: "block" }, { objectId: "b" })).toBe(false);
    expect(inspectorShown(shown, null, null)).toBe(false);
  });

  it("keeps an inspector affordance available while the panel is hidden", () => {
    expect(inspectorAvailable({ kind: "frame" }, { objectId: "f" })).toBe(true);
    expect(inspectorAvailable({ kind: "block" }, { objectId: "b" })).toBe(false);
    expect(inspectorAvailable(null, null)).toBe(false);
  });
});
