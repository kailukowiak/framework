import {
  insertFormulaReference,
  type FormulaReference,
} from "./formulaReferences";
import type { FrameStepInput } from "./types";

export type FormulaEditorKind = "formula" | "scratchwork";

export type FormulaSelection = {
  start: number;
  end: number;
};

export type FormulaCompletionContext = {
  references: FormulaReference[];
  frameId?: string;
  scope?: { steps: FrameStepInput[]; stepIndex: number };
  targetColumnId?: string;
  /** Token inserted when an earlier result in this same output is clicked. */
  previousResultToken?: string;
  /** The row that began a point-and-click formula gesture. */
  anchorRowIndex?: number;
  anchorFrameId?: string;
  /** Whether row-relative expressions have a stable order to read. */
  orderingDeclared?: boolean;
  /** A target column is also used for recurrence picking; scope stays explicit. */
  appliesToAllRows?: boolean;
};

export type ActiveFormulaEditor = {
  id: string;
  label: string;
  kind: FormulaEditorKind;
  draft: string;
  selection: FormulaSelection;
  focused: boolean;
  canCommit: boolean;
  completion: FormulaCompletionContext;
};

export type FormulaEditorBinding = {
  id: string;
  label: string;
  kind: FormulaEditorKind;
  draft: string;
  completion: FormulaCompletionContext;
  onChange: (draft: string, selection: FormulaSelection) => void;
  onSelection?: (selection: FormulaSelection) => void;
  onCommit?: (draft: string) => void | Promise<void>;
  onFocus: (selection: FormulaSelection) => void;
};

const clampSelection = (
  selection: FormulaSelection,
  length: number
): FormulaSelection => {
  const start = Math.max(0, Math.min(selection.start, length));
  const end = Math.max(start, Math.min(selection.end, length));
  return { start, end };
};

function sameActiveEditor(
  left: ActiveFormulaEditor | null,
  right: ActiveFormulaEditor | null
): boolean {
  if (!left || !right) return false;
  return [
    left.id === right.id,
    left.label === right.label,
    left.kind === right.kind,
    left.draft === right.draft,
    left.selection.start === right.selection.start,
    left.selection.end === right.selection.end,
    left.focused === right.focused,
    left.canCommit === right.canCommit,
    left.completion.references === right.completion.references,
    left.completion.frameId === right.completion.frameId,
    left.completion.targetColumnId === right.completion.targetColumnId,
    left.completion.previousResultToken === right.completion.previousResultToken,
    left.completion.anchorRowIndex === right.completion.anchorRowIndex,
    left.completion.anchorFrameId === right.completion.anchorFrameId,
    left.completion.orderingDeclared === right.completion.orderingDeclared,
    left.completion.appliesToAllRows === right.completion.appliesToAllRows,
    left.completion.scope?.steps === right.completion.scope?.steps,
    left.completion.scope?.stepIndex === right.completion.scope?.stepIndex,
  ].every(Boolean);
}

/**
 * The one logical formula cursor in the application.
 *
 * DOM focus cannot be that authority: the formula bar will take browser focus
 * while still editing the formula it mirrors, and pick-from-view will move the
 * pointer into another card while still inserting into this editor. Bindings
 * therefore stay private and replaceable while the serializable snapshot is
 * the stable public fact future surfaces subscribe to.
 */
export class ActiveFormulaEditorRegistry {
  private active: ActiveFormulaEditor | null = null;
  private bindings = new Map<string, FormulaEditorBinding>();
  private bindingOwners = new Map<string, object>();
  private listeners = new Set<() => void>();

  getSnapshot = (): ActiveFormulaEditor | null => this.active;
  getPresenceSnapshot = (): boolean => Boolean(this.active?.focused);

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  bind(binding: FormulaEditorBinding, owner?: object): void {
    this.bindings.set(binding.id, binding);
    if (owner) this.bindingOwners.set(binding.id, owner);
    else this.bindingOwners.delete(binding.id);
    if (this.active?.id !== binding.id) return;
    // An inspector may disappear because the pointer selected another object,
    // then return. The logical draft outlives that presentation surface: on
    // rebinding, restore it into the new owner instead of replacing it with
    // the last saved prop and silently discarding everything typed in the bar.
    if (this.active.draft !== binding.draft)
      binding.onChange(
        this.active.draft,
        clampSelection(this.active.selection, this.active.draft.length)
      );
    const selection = clampSelection(
      this.active.selection,
      this.active.draft.length
    );
    this.publish({
      ...this.active,
      label: binding.label,
      kind: binding.kind,
      selection,
      canCommit: Boolean(binding.onCommit),
      completion: binding.completion,
    });
  }

  unbind(id: string, owner?: object): void {
    // Moving one logical editor between two surfaces mounts the replacement
    // before React necessarily runs the old surface's passive cleanup. That
    // cleanup belongs to the old binding and must not erase the new one just
    // because both correctly share an editor id.
    if (owner && this.bindingOwners.get(id) !== owner) return;
    this.bindings.delete(id);
    this.bindingOwners.delete(id);
    if (this.active?.id === id && this.active.focused)
      this.publish({ ...this.active, focused: false });
  }

  activate(id: string, selection: FormulaSelection): void {
    const binding = this.bindings.get(id);
    if (!binding) return;
    this.publish({
      id,
      label: binding.label,
      kind: binding.kind,
      draft: binding.draft,
      selection: clampSelection(selection, binding.draft.length),
      focused: true,
      canCommit: Boolean(binding.onCommit),
      completion: binding.completion,
    });
  }

  /** Activate a logical editor that has no local text box of its own. */
  activateAndFocus(id: string, selection: FormulaSelection): void {
    const binding = this.bindings.get(id);
    if (!binding) return;
    this.activate(id, selection);
    binding.onFocus(clampSelection(selection, binding.draft.length));
  }

  blur(id: string): void {
    if (this.active?.id !== id || !this.active.focused) return;
    // Keep the logical editor alive. The formula bar necessarily blurs
    // the source textarea when it takes the keyboard, but must keep editing
    // this exact draft rather than creating a second one.
    this.publish({ ...this.active, focused: false });
  }

  updateFromEditor(
    id: string,
    draft: string,
    selection: FormulaSelection
  ): void {
    if (this.active?.id !== id) return;
    this.publish({
      ...this.active,
      draft,
      selection: clampSelection(selection, draft.length),
      focused: true,
    });
  }

  updateSelection(id: string, selection: FormulaSelection): void {
    if (this.active?.id !== id) return;
    this.publish({
      ...this.active,
      selection: clampSelection(selection, this.active.draft.length),
    });
  }

  setDraft(draft: string, selection: FormulaSelection): void {
    if (!this.active) return;
    const binding = this.bindings.get(this.active.id);
    const nextSelection = clampSelection(selection, draft.length);
    this.publish({
      ...this.active,
      draft,
      selection: nextSelection,
    });
    binding?.onChange(draft, nextSelection);
  }

  setSelection(selection: FormulaSelection): void {
    if (!this.active) return;
    const nextSelection = clampSelection(selection, this.active.draft.length);
    this.publish({
      ...this.active,
      selection: nextSelection,
    });
    this.bindings.get(this.active.id)?.onSelection?.(nextSelection);
  }

  replaceSelection(text: string): void {
    if (!this.active) return;
    const { draft, selection } = this.active;
    const next = `${draft.slice(0, selection.start)}${text}${draft.slice(
      selection.end
    )}`;
    const cursor = selection.start + text.length;
    this.setDraft(next, { start: cursor, end: cursor });
    this.focus();
  }

  insertReference(token: string, refocus = true): void {
    if (!this.active) return;
    const { draft, selection } = this.active;
    if (selection.start !== selection.end) {
      const next = `${draft.slice(0, selection.start)}${token}${draft.slice(
        selection.end
      )}`;
      const cursor = selection.start + token.length;
      this.setDraft(next, { start: cursor, end: cursor });
      if (refocus) this.focus();
      return;
    }
    const inserted = insertFormulaReference(draft, selection.end, token);
    this.setDraft(inserted.source, {
      start: inserted.cursor,
      end: inserted.cursor,
    });
    if (refocus) this.focus();
  }

  focus(): void {
    if (!this.active) return;
    const binding = this.bindings.get(this.active.id);
    if (!binding) return;
    this.publish({ ...this.active, focused: true });
    binding.onFocus(this.active.selection);
  }

  engage(): void {
    if (this.active) this.publish({ ...this.active, focused: true });
  }

  disengage(): void {
    if (this.active) this.publish({ ...this.active, focused: false });
  }

  async commit(): Promise<void> {
    if (!this.active) return;
    const binding = this.bindings.get(this.active.id);
    if (!binding?.onCommit) return;
    await binding.onCommit(this.active.draft);
  }

  clear(id?: string): void {
    if (!this.active || (id !== undefined && this.active.id !== id)) return;
    this.publish(null);
  }

  private publish(next: ActiveFormulaEditor | null): void {
    if (next === this.active || sameActiveEditor(next, this.active)) return;
    this.active = next;
    for (const listener of this.listeners) listener();
  }
}
