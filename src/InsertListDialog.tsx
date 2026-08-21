import { Plus, Sparkles, X } from "lucide-react";
import { useState } from "react";
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
  const [name, setName] = useState("New list");
  const [content, setContent] = useState("");
  const [column, setColumn] = useState("");
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
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="insert-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">ADD TO THIS CONTAINER</span>
            <h2>A list to reference</h2>
          </div>
          <button className="icon-button" onClick={onClose}>
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
          Values
          <textarea
            className="large-dialog-input series-values"
            value={content}
            spellCheck={false}
            onChange={(event) => setContent(event.target.value)}
            placeholder={"USD\nCAD\nEUR"}
          />
        </label>
        <div className="interpretation">
          <Sparkles size={15} />
          <span>
            One per line, or paste <code>[1, 2, 3]</code>,{" "}
            <code>array([1, 2, 3])</code>, <code>c(1, 2, 3)</code> — all read
            the same.
          </span>
        </div>
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
            Create list
            <Plus size={15} />
          </button>
        </div>
      </div>
    </div>
  );
}
