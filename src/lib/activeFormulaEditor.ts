import {
  insertFormulaReference,
  referenceInsertionRange,
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
 *
 * Session lifecycle, in one place because every surface must agree on it. A
 * session begins at `activate` and stays alive through DOM blur and through
 * its surface unmounting — that survival is the point of the registry, not a
 * leak. What ends a session:
 *
 * - A successful `commit` of a `"formula"` editor. Enter means "apply and be
 *   done"; leaving the session armed afterwards is how a stray cell click
 *   used to rewrite a formula that was already applied. A `"scratchwork"`
 *   editor is a multi-line document that commits continuously, so committing
 *   it keeps the session — its owner decides when editing is over.
 * - `cancel` (Escape). For a `"formula"` editor this also restores the draft
 *   the session opened with, because Escape promises the formula is as it
 *   was. A scratchwork block autosaves and has no "as it was" to return to,
 *   so cancel only ends its session.
 * - A click that is neither a reference pick nor inside an editing surface —
 *   routed here by the canvas pointer handler calling `clear`.
 *
 * `blur` and `disengage` deliberately end nothing.
 */
export class ActiveFormulaEditorRegistry {
  private active: ActiveFormulaEditor | null = null;
  private bindings = new Map<string, FormulaEditorBinding>();
  private bindingOwners = new Map<string, object>();
  private listeners = new Set<() => void>();
  /**
   * The active session's binding, kept after its surface unmounted. If the
   * draft outlives the surface — the survival the class comment promises —
   * then Enter and Escape must too: a bar that says "Edit Column 1" while
   * commit silently does nothing is a trap, not a session. The retained
   * callbacks close over the owner's last render, which stays valid exactly
   * as long as the document chain they would write hasn't moved; the owner
   * ends its sessions when it sees that chain move (see useDocumentChainSync
   * in PipelineEditor), which leaves one accepted residual: a chain that
   * moves while the owner is unmounted (an undo with the inspector closed)
   * can still be committed over from here.
   */
  private dormant: FormulaEditorBinding | null = null;
  /**
   * The draft as it stood when this session began — what Escape restores.
   * Captured at activation rather than read from the binding at cancel time,
   * because bindings re-bind on every render with the *current* draft, so by
   * the time Escape arrives the binding no longer remembers the saved text.
   */
  private sessionOpeningDraft: string | null = null;

  getSnapshot = (): ActiveFormulaEditor | null => this.active;
  getPresenceSnapshot = (): boolean => Boolean(this.active?.focused);

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  /** The active session's binding: live if mounted, else the retained one. */
  private bindingFor(id: string): FormulaEditorBinding | undefined {
    return this.bindings.get(id) ?? (this.dormant?.id === id ? this.dormant : undefined);
  }

  bind(binding: FormulaEditorBinding, owner?: object): void {
    this.bindings.set(binding.id, binding);
    if (this.dormant?.id === binding.id) this.dormant = null;
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
    const departing = this.bindings.get(id);
    this.bindings.delete(id);
    this.bindingOwners.delete(id);
    if (this.active?.id !== id) return;
    if (departing) this.dormant = departing;
    if (this.active.focused) this.publish({ ...this.active, focused: false });
  }

  activate(id: string, selection: FormulaSelection): void {
    const binding = this.bindings.get(id);
    if (!binding) return;
    // A new session starts here; re-focusing the surface of the session
    // already underway must not move the Escape point mid-edit.
    if (this.active?.id !== id) this.sessionOpeningDraft = binding.draft;
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

  /** A document rewrite is not typing and must not take focus. Keep the
   * retained draft in sync before its surface rebinds; otherwise bind()
   * correctly restores the retained draft, but restores an obsolete value.
   */
  reconcile(id: string, draft: string, selection: FormulaSelection): void {
    if (this.active?.id !== id) return;
    this.publish({ ...this.active, draft, selection: clampSelection(selection, draft.length) });
  }

  setDraft(draft: string, selection: FormulaSelection): void {
    if (!this.active) return;
    const binding = this.bindingFor(this.active.id);
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
    this.bindingFor(this.active.id)?.onSelection?.(nextSelection);
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
    const { draft } = this.active;
    // Scratchwork is a document of many named lines, not one named command,
    // so the name-preserving narrowing below is not its rule to follow.
    const selection =
      this.active.kind === "formula"
        ? referenceInsertionRange(draft, this.active.selection)
        : this.active.selection;
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
    const binding = this.bindingFor(this.active.id);
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

  async commit(options: { keepEditing?: boolean } = {}): Promise<void> {
    if (!this.active) return;
    const committed = this.active;
    const binding = this.bindingFor(committed.id);
    if (!binding?.onCommit) return;
    await binding.onCommit(this.active.draft);
    // A commit that resolved ends a formula session (see the class comment).
    // A commit that threw leaves the session alive for the surface that has
    // to show the failure. `keepEditing` is for gestures that persist as a
    // side effect of something else — Format tidies and saves the draft, but
    // nobody pressing Format meant "I am finished editing". Re-read `active`
    // after the await: the commit handler may have moved or ended the
    // session itself, and ending its successor would punish it for that.
    if (options.keepEditing || committed.kind !== "formula") return;
    if (this.active?.id === committed.id) this.clear(committed.id);
  }

  /**
   * Escape: put the draft back the way the session found it, then end the
   * session. The revert applies to formula editors only — see the class
   * comment for why scratchwork keeps its text.
   */
  cancel(id?: string): void {
    if (!this.active || (id !== undefined && this.active.id !== id)) return;
    const binding = this.bindingFor(this.active.id);
    const opening = this.sessionOpeningDraft;
    if (
      this.active.kind === "formula" &&
      binding &&
      opening !== null &&
      opening !== this.active.draft
    )
      binding.onChange(
        opening,
        clampSelection(this.active.selection, opening.length)
      );
    this.clear(this.active.id);
  }

  clear(id?: string): void {
    if (!this.active || (id !== undefined && this.active.id !== id)) return;
    this.sessionOpeningDraft = null;
    this.dormant = null;
    this.publish(null);
  }

  private publish(next: ActiveFormulaEditor | null): void {
    if (next === this.active || sameActiveEditor(next, this.active)) return;
    this.active = next;
    for (const listener of this.listeners) listener();
  }
}
