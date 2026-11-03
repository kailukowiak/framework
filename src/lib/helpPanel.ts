import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type MutableRefObject,
} from "react";

/**
 * What help should show: the words to search, and the answer to open if one
 * is known. `keepFocus` is for a request made mid-formula by pointing — the
 * formula bar's Help button — where the next keystroke still belongs to the
 * formula; a keyboard request (⌘K) moves focus to help's search instead.
 */
export type HelpTarget = { query: string; entryId?: string; keepFocus?: boolean };

/**
 * One request to the help panel. `serial` changes on every request, so asking
 * again while the panel is open retargets it rather than being ignored as the
 * same state.
 */
export type HelpRequest = HelpTarget & { serial: number };

export type HelpPanel = {
  request: HelpRequest | null;
  open: (target?: HelpTarget) => void;
  toggle: (target: HelpTarget) => void;
  close: () => void;
  /**
   * What the formula bar's cursor is on, kept current by the bar. ⌘K reaches
   * the application through the native menu, which cannot see the cursor, so
   * the bar writes its answer here and the menu action reads it. That is what
   * makes ⌘K and the bar's Help button open the same entry.
   */
  formulaTarget: MutableRefObject<HelpTarget | undefined>;
};

/**
 * The help panel's state, out of App so the surfaces that open help — the
 * menu, Quick Commands, the formula bar and the completion menus — share one
 * request rather than each keeping a flag and a query of its own.
 */
export function useHelpPanel(): HelpPanel {
  const [request, setRequest] = useState<HelpRequest | null>(null);
  const serial = useRef(0);
  const formulaTarget = useRef<HelpTarget | undefined>(undefined);
  const open = useCallback((target: HelpTarget = { query: "" }) => {
    serial.current += 1;
    setRequest({ ...target, serial: serial.current });
  }, []);
  const close = useCallback(() => setRequest(null), []);
  const toggle = useCallback(
    (target: HelpTarget) => {
      if (request) close();
      else open(target);
    },
    [close, open, request]
  );
  return useMemo(
    () => ({ request, open, toggle, close, formulaTarget }),
    [close, open, request, toggle]
  );
}

export const HelpPanelContext = createContext<HelpPanel | null>(null);

/** Null outside an application window, where there is no help panel to open. */
export function useHelpPanelContext(): HelpPanel | null {
  return useContext(HelpPanelContext);
}
