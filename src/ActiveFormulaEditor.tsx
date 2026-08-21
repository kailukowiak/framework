import {
  createContext,
  type ReactNode,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useSyncExternalStore,
} from "react";
import {
  ActiveFormulaEditorRegistry,
  type FormulaEditorBinding,
  type FormulaSelection,
} from "./lib/activeFormulaEditor";

const ActiveFormulaEditorContext =
  createContext<ActiveFormulaEditorRegistry | null>(null);

export function ActiveFormulaEditorProvider({ children }: { children: ReactNode }) {
  const registry = useRef<ActiveFormulaEditorRegistry>(null);
  if (!registry.current) registry.current = new ActiveFormulaEditorRegistry();
  return (
    <ActiveFormulaEditorContext.Provider value={registry.current}>
      {children}
    </ActiveFormulaEditorContext.Provider>
  );
}

function useRegistry(): ActiveFormulaEditorRegistry {
  const registry = useContext(ActiveFormulaEditorContext);
  if (!registry)
    throw new Error("Formula editors must live inside ActiveFormulaEditorProvider");
  return registry;
}

/** Read and edit the shared draft from a mirror such as the formula bar. */
export function useActiveFormulaEditor() {
  const registry = useRegistry();
  const active = useSyncExternalStore(
    registry.subscribe,
    registry.getSnapshot,
    registry.getSnapshot
  );
  return {
    active,
    setDraft: registry.setDraft.bind(registry),
    setSelection: registry.setSelection.bind(registry),
    replaceSelection: registry.replaceSelection.bind(registry),
    insertReference: registry.insertReference.bind(registry),
    focus: registry.focus.bind(registry),
    engage: registry.engage.bind(registry),
    disengage: registry.disengage.bind(registry),
    commit: registry.commit.bind(registry),
    clear: registry.clear.bind(registry),
  };
}

/** Presence without subscribing a large parent to every draft keystroke. */
export function useActiveFormulaEditorPresence() {
  const registry = useRegistry();
  return useSyncExternalStore(
    registry.subscribe,
    registry.getPresenceSnapshot,
    registry.getPresenceSnapshot
  );
}

/** Commands for a surface that does not need every draft keystroke rendered. */
export function useActiveFormulaEditorCommands() {
  const registry = useRegistry();
  return useMemo(
    () => ({
      setDraft: registry.setDraft.bind(registry),
      setSelection: registry.setSelection.bind(registry),
      replaceSelection: registry.replaceSelection.bind(registry),
      insertReference: registry.insertReference.bind(registry),
      focus: registry.focus.bind(registry),
      engage: registry.engage.bind(registry),
      disengage: registry.disengage.bind(registry),
      commit: registry.commit.bind(registry),
      clear: registry.clear.bind(registry),
      activateAndFocus: registry.activateAndFocus.bind(registry),
      getActive: registry.getSnapshot,
    }),
    [registry]
  );
}

/**
 * Connect one rendered editing surface to the app-level registry. Rebinding
 * after every render is intentional: React callbacks close over their current
 * parent draft, while the logical editor identity must remain unchanged.
 */
export function useFormulaEditorRegistration(binding: FormulaEditorBinding) {
  const registry = useRegistry();
  const owner = useRef<object>({});
  useLayoutEffect(() => registry.bind(binding, owner.current));
  useEffect(
    () => () => registry.unbind(binding.id, owner.current),
    [binding.id, registry]
  );

  const selectionOf = (
    target: HTMLTextAreaElement | HTMLInputElement
  ): FormulaSelection => ({
    start: target.selectionStart ?? target.value.length,
    end: target.selectionEnd ?? target.selectionStart ?? target.value.length,
  });

  return {
    activateAt(selection: FormulaSelection) {
      registry.activateAndFocus(binding.id, selection);
    },
    activate(target: HTMLTextAreaElement | HTMLInputElement) {
      registry.activate(binding.id, selectionOf(target));
    },
    blur() {
      registry.blur(binding.id);
    },
    change(
      draft: string,
      target: HTMLTextAreaElement | HTMLInputElement
    ) {
      registry.updateFromEditor(binding.id, draft, selectionOf(target));
    },
    update(draft: string, selection: FormulaSelection) {
      registry.updateFromEditor(binding.id, draft, selection);
    },
    select(target: HTMLTextAreaElement | HTMLInputElement) {
      registry.updateSelection(binding.id, selectionOf(target));
    },
    commit() {
      return registry.commit();
    },
  };
}
