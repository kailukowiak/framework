import { Plus, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { Operation } from "./lib/types";

/**
 * Making a list.
 *
 * The box takes whatever shape the list is already in — a pasted spreadsheet
 * column, `[1, 2, 3]`, a NumPy or R repr, a comma-separated line — because
 * nobody has a list they are willing to retype, and the core reads all of
 * them. Reading one out of a file is the same dialog's other half: name a
 * column and Polars does it.
 */
export function InsertListDialog({
  state,
  onClose,
  onCreate,
  onPickFile,
}: {
  state: { containerId: string };
  onClose: () => void;
  onCreate: (operation: Operation) => void;
  onPickFile: () => Promise<string | null>;
}) {
  const [name, setName] = useState("New vector");
  const [content, setContent] = useState("");
  const [column, setColumn] = useState("");
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);
  // The container places its own members, so the position is nothing but a
  // field the operation still asks for.
  const create = () =>
    onCreate({
      type: "addSeries",
      name,
      values: content,
      x: 0,
      y: 0,
      containerId: state.containerId,
    });
  const fromFile = async () => {
    const path = await onPickFile();
    if (!path) return;
    onCreate({
      type: "importSeriesFromFile",
      name,
      path,
      column: column.trim() || null,
      x: 0,
      y: 0,
      containerId: state.containerId,
    });
  };

  return (
    <aside
      className="insert-dialog insert-list-popover"
      role="dialog"
      aria-modal="false"
      aria-labelledby="insert-list-heading"
      onPointerDown={(event) => event.stopPropagation()}
    >
      <div className="dialog-header">
        <div>
          <span className="eyebrow">ADD TO THIS CONTAINER</span>
          <h2 id="insert-list-heading">A vector to reference</h2>
        </div>
        <button
          className="icon-button"
          aria-label="Close vector editor"
          onClick={onClose}
        >
          <X size={18} />
        </button>
      </div>
      <label>
        Name
        <input
          autoFocus
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <label>
        Values — paste a column or write a comma-separated vector
        <textarea
          className="large-dialog-input series-values"
          value={content}
          spellCheck={false}
          onChange={(event) => setContent(event.target.value)}
          placeholder="USD, CAD, EUR"
        />
      </label>
      <label>
        Or read a column out of a file
        <input
          value={column}
          onChange={(event) => setColumn(event.target.value)}
          placeholder="Column name — blank takes the first"
        />
      </label>
      <div className="dialog-actions">
        <button className="secondary-action" onClick={onClose}>
          Cancel
        </button>
        <button className="secondary-action" onClick={() => void fromFile()}>
          Choose file…
        </button>
        <button
          className="primary-action"
          disabled={!content.trim()}
          onClick={create}
        >
          Create vector
          <Plus size={15} />
        </button>
      </div>
    </aside>
  );
}
