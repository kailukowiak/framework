import { ShieldAlert } from "lucide-react";
import { exitSafeMode } from "./lib/api";
import type { DocumentView } from "./lib/types";

/**
 * The recovery banner for a document opened in safe mode (⌥-open in the Data
 * Library): nothing is evaluated and no data is read until the person turns
 * evaluation back on. Leaving safe mode recomputes the document in place —
 * the live moment the repair is tested, so failures surface as the ordinary
 * app error rather than a hang on open.
 */
export function SafeModeBanner({
  document,
  onDocument,
  onError,
}: {
  document: DocumentView | null;
  onDocument: (view: DocumentView) => void;
  onError: (message: string | null) => void;
}) {
  // The banner owns its own visibility: safe mode is its whole subject, and
  // keeping the check here spares the app shell another render conditional.
  if (!document?.safeMode) return null;
  const turnOnEvaluation = async () => {
    try {
      onDocument(await exitSafeMode());
      onError(null);
    } catch (reason) {
      onError(String(reason).replace(/^Error:\s*/, ""));
    }
  };
  return (
    <div className="safe-mode-banner" role="status">
      <span>
        <ShieldAlert size={14} />
        Safe mode — nothing is evaluated and no data is loaded. Delete or fix
        what’s wrong, save, then turn evaluation back on.
      </span>
      <button
        className="secondary-action"
        onClick={() => void turnOnEvaluation()}
      >
        Turn on evaluation
      </button>
    </div>
  );
}
