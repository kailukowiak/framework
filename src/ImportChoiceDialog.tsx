import { FolderOpen, X } from "lucide-react";
import { useEffect } from "react";
import type { ImportMode } from "./lib/preferences";

/**
 * What kind of import this is, asked before the file picker opens.
 *
 * The question is not about the file, it is about what this document
 * becomes: one that holds its own numbers, or one that depends on a file
 * somewhere else. That is worth a moment the first few times and worth
 * turning off once someone knows which one they want, which is what the
 * checkbox is for.
 *
 * The two options are stated by consequence rather than by mechanism.
 * "Keeps a connector" is true and means nothing; "a refresh replaces
 * anything you have typed" is the thing you would want to have known.
 */
export function ImportChoiceDialog({
  mode,
  onModeChange,
  askOnImport,
  onAskOnImportChange,
  onChoose,
  onCancel,
}: {
  mode: ImportMode;
  onModeChange: (mode: ImportMode) => void;
  askOnImport: boolean;
  onAskOnImportChange: (ask: boolean) => void;
  onChoose: (mode: ImportMode) => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onCancel]);

  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <div className="insert-dialog import-choice-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">IMPORT</span>
            <h2>How should this data be kept?</h2>
          </div>
          <button className="icon-button" onClick={onCancel}>
            <X size={18} />
          </button>
        </div>

        <div className="import-choices">
          {(
            [
              {
                value: "stored",
                title: "Static — hold the data in this document",
                detail:
                  "The values are copied beside the document and belong to it. Nothing will refresh over them, and you can type into them.",
              },
              {
                value: "linked",
                title: "Refreshable — keep reading the file",
                detail:
                  "The frame can be refreshed from the file whenever it changes. A refresh replaces its values, so it cannot be edited here.",
              },
            ] as const
          ).map((choice) => (
            <label
              key={choice.value}
              className={`import-choice ${mode === choice.value ? "selected" : ""}`}
            >
              <input
                type="radio"
                name="import-mode"
                checked={mode === choice.value}
                onChange={() => onModeChange(choice.value)}
              />
              <span>
                <strong>{choice.title}</strong>
                <small>{choice.detail}</small>
              </span>
            </label>
          ))}
        </div>

        <label className="preference-check">
          <input
            type="checkbox"
            checked={askOnImport}
            onChange={(event) => onAskOnImportChange(event.target.checked)}
          />
          <span>Ask every time I import</span>
        </label>
        <p className="preference-note">
          Whichever you pick becomes the one offered next time. Preferences has
          both settings if you change your mind.
        </p>

        <div className="dialog-actions">
          <button className="secondary-action" onClick={onCancel}>
            Cancel
          </button>
          <button className="primary-action" onClick={() => onChoose(mode)}>
            Choose a file…
            <FolderOpen size={15} />
          </button>
        </div>
      </div>
    </div>
  );
}
