import { describe, expect, it, vi } from "vitest";
import {
  ActiveFormulaEditorRegistry,
  type FormulaEditorBinding,
} from "./activeFormulaEditor";

function binding(
  overrides: Partial<FormulaEditorBinding> = {}
): FormulaEditorBinding {
  return {
    id: "column:amount",
    label: "Column formula",
    kind: "formula",
    draft: "amount.sum()",
    completion: { references: [] },
    onChange: vi.fn(),
    onCommit: vi.fn(),
    onFocus: vi.fn(),
    ...overrides,
  };
}

describe("ActiveFormulaEditorRegistry", () => {
  it("publishes a changed row anchor for the same logical editor", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const first = binding({
      completion: {
        references: [],
        targetColumnId: "result",
        anchorRowIndex: 1,
        anchorFrameId: "frame",
      },
    });
    registry.bind(first);
    registry.activate(first.id, { start: 0, end: 0 });
    registry.bind({
      ...first,
      completion: { ...first.completion, anchorRowIndex: 4 },
    });
    expect(registry.getSnapshot()?.completion.anchorRowIndex).toBe(4);
  });

  it("publishes the active editor, draft, selection, and commit capability", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding();
    registry.bind(editor);
    registry.activate(editor.id, { start: 2, end: 6 });

    expect(registry.getSnapshot()).toEqual({
      id: editor.id,
      label: editor.label,
      kind: editor.kind,
      draft: editor.draft,
      selection: { start: 2, end: 6 },
      focused: true,
      canCommit: true,
      completion: { references: [] },
    });
  });

  it("routes an external replacement through the same draft and cursor", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding({ draft: "before + after" });
    registry.bind(editor);
    registry.activate(editor.id, { start: 0, end: 6 });

    registry.replaceSelection("revenue");

    expect(editor.onChange).toHaveBeenCalledWith("revenue + after", {
      start: 7,
      end: 7,
    });
    expect(editor.onFocus).toHaveBeenCalledWith({ start: 7, end: 7 });
    expect(registry.getSnapshot()?.draft).toBe("revenue + after");
  });

  it("replaces a partial token when a column is picked from the view", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding({ draft: "deb + 1" });
    registry.bind(editor);
    registry.activate(editor.id, { start: 3, end: 3 });

    registry.insertReference("`Debit`");

    expect(editor.onChange).toHaveBeenCalledWith("`Debit` + 1", {
      start: 7,
      end: 7,
    });
    expect(editor.onFocus).toHaveBeenCalledWith({ start: 7, end: 7 });
  });

  it("keeps a formula-bar pick in the bar", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding({ draft: "deb" });
    registry.bind(editor);
    registry.activate(editor.id, { start: 3, end: 3 });

    registry.insertReference("`Debit`", false);

    expect(editor.onChange).toHaveBeenCalledWith("`Debit`", {
      start: 7,
      end: 7,
    });
    expect(editor.onFocus).not.toHaveBeenCalled();
  });

  it("commits the latest registry draft rather than an activation-time closure", async () => {
    const onCommit = vi.fn();
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding({ onCommit });
    registry.bind(editor);
    registry.activate(editor.id, { start: 0, end: 0 });
    registry.setDraft("credit.sum()", { start: 12, end: 12 });

    await registry.commit();

    expect(onCommit).toHaveBeenCalledWith("credit.sum()");
  });

  it("lets a mirror change the shared draft without stealing DOM focus", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding();
    registry.bind(editor);
    registry.activate(editor.id, { start: 0, end: 0 });
    registry.blur(editor.id);

    registry.setDraft("from the bar", { start: 12, end: 12 });

    expect(editor.onChange).toHaveBeenCalledWith("from the bar", {
      start: 12,
      end: 12,
    });
    expect(editor.onFocus).not.toHaveBeenCalled();
    expect(registry.getSnapshot()?.focused).toBe(false);
  });

  it("lets a mirror move the logical cursor without refocusing the source", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const onSelection = vi.fn();
    const editor = binding({ onSelection });
    registry.bind(editor);
    registry.activate(editor.id, { start: 0, end: 0 });
    registry.blur(editor.id);

    registry.setSelection({ start: 4, end: 8 });

    expect(registry.getSnapshot()?.selection).toEqual({ start: 4, end: 8 });
    expect(onSelection).toHaveBeenCalledWith({ start: 4, end: 8 });
    expect(editor.onFocus).not.toHaveBeenCalled();
  });

  it("does not let a stale editor cleanup clear the editor that replaced it", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const first = binding({ id: "first" });
    const second = binding({ id: "second", draft: "2" });
    registry.bind(first);
    registry.bind(second);
    registry.activate(first.id, { start: 0, end: 0 });
    registry.activate(second.id, { start: 1, end: 1 });

    registry.unbind(first.id);

    expect(registry.getSnapshot()?.id).toBe(second.id);
  });

  it("does not let an old surface unregister the same editor in its new location", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const cardOwner = {};
    const drawerOwner = {};
    const card = binding({ id: "scratchwork:one", draft: "from card" });
    const drawer = binding({ id: "scratchwork:one", draft: "from drawer" });
    registry.bind(card, cardOwner);
    registry.activate(card.id, { start: 9, end: 9 });

    registry.bind(drawer, drawerOwner);
    registry.unbind(card.id, cardOwner);
    registry.setDraft("still upstairs", { start: 14, end: 14 });

    expect(drawer.onChange).toHaveBeenCalledWith("still upstairs", {
      start: 14,
      end: 14,
    });
  });

  it("retains logical ownership across DOM blur without leaving pick mode on", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding();
    registry.bind(editor);
    registry.activate(editor.id, { start: 3, end: 3 });

    registry.blur(editor.id);

    expect(registry.getSnapshot()).toMatchObject({
      id: editor.id,
      focused: false,
      selection: { start: 3, end: 3 },
    });
    expect(registry.getPresenceSnapshot()).toBe(false);
  });

  it("keeps an unsaved bar draft while its inspector surface is away", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const first = binding({ draft: "saved" });
    registry.bind(first);
    registry.activate(first.id, { start: 5, end: 5 });
    registry.setDraft("still drafting", { start: 14, end: 14 });

    registry.unbind(first.id);
    const replacement = binding({ draft: "saved" });
    registry.bind(replacement);

    expect(registry.getSnapshot()?.draft).toBe("still drafting");
    expect(replacement.onChange).toHaveBeenCalledWith("still drafting", {
      start: 14,
      end: 14,
    });
  });

  it("continues editing the retained draft while its surface is unmounted", () => {
    const registry = new ActiveFormulaEditorRegistry();
    const editor = binding({ draft: "before" });
    registry.bind(editor);
    registry.activate(editor.id, { start: 6, end: 6 });
    registry.unbind(editor.id);

    registry.setDraft("after", { start: 5, end: 5 });

    expect(registry.getSnapshot()).toMatchObject({
      draft: "after",
      selection: { start: 5, end: 5 },
    });
  });
});
