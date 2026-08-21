import { CircleAlert, X } from "lucide-react";
import { useState } from "react";
import { newDocumentDialog } from "./lib/api";
import type { DocumentView } from "./lib/types";

export function NewDocumentDialog({
  onClose,
  onOpened,
}: {
  onClose: () => void;
  onOpened: (opened: { document: DocumentView; path: string }) => void;
}) {
  const [name, setName] = useState("Untitled");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  const create = async () => {
    if (!name.trim() || creating) return;
    setCreating(true);
    setCreateError(null);
    try {
      const opened = await newDocumentDialog(name.trim());
      if (opened) onOpened(opened);
      else setCreating(false);
    } catch (reason) {
      setCreateError(String(reason).replace(/^Error:\s*/, ""));
      setCreating(false);
    }
  };

  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget && !creating) onClose();
      }}
    >
      <form
        className="insert-dialog new-document-dialog"
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        <div className="dialog-header">
          <div>
            <span className="eyebrow">NEW DOCUMENT</span>
            <h2>Create a blank workspace</h2>
          </div>
          <button
            type="button"
            className="icon-button"
            disabled={creating}
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </div>
        <label>
          Document name
          <input
            className="large-dialog-input"
            autoFocus
            value={name}
            onFocus={(event) => event.currentTarget.select()}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <p className="new-document-note">
          You’ll choose where to save the .fw document before it opens, so imported data
          has a home from the start.
        </p>
        {createError && (
          <div className="formula-editor-error">
            <CircleAlert size={12} />
            <span>{createError}</span>
          </div>
        )}
        <div className="dialog-actions">
          <button
            type="button"
            className="secondary-action"
            disabled={creating}
            onClick={onClose}
          >
            Cancel
          </button>
          <button
            type="submit"
            className="primary-action"
            disabled={creating || !name.trim()}
          >
            {creating ? "Choosing location…" : "Choose location…"}
          </button>
        </div>
      </form>
    </div>
  );
}
