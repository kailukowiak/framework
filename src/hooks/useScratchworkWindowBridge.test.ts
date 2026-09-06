// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ActiveFormulaEditor } from "../lib/activeFormulaEditor";
import type { BlockObject } from "../lib/types";
import { useScratchworkWindowBridge } from "./useScratchworkWindowBridge";

type Listener = (event: { payload: unknown }) => void;
const listeners = new Map<string, Listener>();
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    listen: (name: string, handler: Listener) => {
      listeners.set(name, handler);
      return Promise.resolve(() => listeners.delete(name));
    },
  }),
}));
const editScratchworkWindow = vi.fn(async (_edit: unknown) => true);
const clearScratchworkWindowEditor = vi.fn(async () => true);
vi.mock("../lib/api", () => ({
  editScratchworkWindow: (edit: unknown) => editScratchworkWindow(edit),
  clearScratchworkWindowEditor: () => clearScratchworkWindowEditor(),
}));

const block = { id: "scratch", kind: "block", name: "Scratchwork", lines: [] } as unknown as BlockObject;

function localEditor(focused: boolean): ActiveFormulaEditor {
  return {
    id: "scratchwork:scratch",
    label: "Scratchwork",
    kind: "scratchwork",
    draft: "canvas = ",
    selection: { start: 9, end: 9 },
    focused,
    canCommit: true,
    completion: { references: [] },
  };
}

function localCommands(active: ActiveFormulaEditor | null) {
  return {
    getActive: () => active,
    insertReference: vi.fn(),
    replaceSelection: vi.fn(),
    cancel: vi.fn(),
    clear: vi.fn(),
    disengage: vi.fn(),
  };
}

async function popOutEditing() {
  await act(async () => {
    await Promise.resolve();
  });
  await act(async () => {
    listeners.get("framework-scratchwork-editor-state")?.({
      payload: {
        revision: 1,
        active: true,
        draft: "popped = ",
        selectionStart: 9,
        selectionEnd: 9,
      },
    });
  });
}

describe("useScratchworkWindowBridge", () => {
  beforeEach(() => {
    Object.assign(window, { __TAURI_INTERNALS__: {} });
  });
  afterEach(() => {
    listeners.clear();
    vi.clearAllMocks();
  });

  it("routes a pick to the pop-out when the local session is only dormant", async () => {
    // A canvas-block session outlives its focus by design, so it is still
    // "active" here while the pop-out holds the caret. The canvas pointer
    // handler only picks for a focused editor, so answering with the blurred
    // local session meant no reference ever reached the pop-out.
    const local = localCommands(localEditor(false));
    const { result } = renderHook(() =>
      useScratchworkWindowBridge({ local, localFocused: false, block, references: [] })
    );
    await popOutEditing();

    expect(result.current.getActive()).toMatchObject({ draft: "popped = ", focused: true });
    act(() => result.current.insertReference("`Sales`.`Amount`"));
    expect(editScratchworkWindow).toHaveBeenCalledWith({
      text: "`Sales`.`Amount`",
      replaceSelection: false,
      refocus: false,
    });
    expect(local.insertReference).not.toHaveBeenCalled();
  });

  it("keeps a focused local editor ahead of the pop-out", async () => {
    const local = localCommands(localEditor(true));
    const { result } = renderHook(() =>
      useScratchworkWindowBridge({ local, localFocused: true, block, references: [] })
    );
    await popOutEditing();

    expect(result.current.getActive()).toMatchObject({ draft: "canvas = " });
    act(() => result.current.insertReference("`Sales`.`Amount`"));
    expect(local.insertReference).toHaveBeenCalledWith("`Sales`.`Amount`");
    expect(editScratchworkWindow).not.toHaveBeenCalled();
  });
});
