import { X } from "lucide-react";
import { useEffect } from "react";

const GROUPS = [
  {
    name: "Canvas",
    shortcuts: [
      ["Move to card left, right, up, or down", "← → ↑ ↓"],
      ["Cycle cards forward or backward", "Tab / ⇧Tab"],
      ["Arrange cards left to right", "⇧⌘A"],
      ["Fit selected card to window", "⇧⌘F"],
      ["Collapse or expand selected card", "⇧⌘M"],
      ["Open or leave Scratchwork", "⌘J"],
      ["Zoom in", "⌘+"],
      ["Zoom out", "⌘−"],
      ["Actual size", "⌘0"],
    ],
  },
  {
    name: "Inspector",
    shortcuts: [
      ["Selection", "⌘1"],
      ["Format", "⌘2"],
      ["Wrangle", "⌘3"],
    ],
  },
  {
    name: "Insert",
    shortcuts: [
      ["Formula block", "⌥⌘B"],
      ["Text", "⌥⌘T"],
      ["Frame", "⌥⌘F"],
      ["Container", "⌥⌘G"],
    ],
  },
  {
    name: "Document",
    shortcuts: [
      ["New document", "⌘N"],
      ["New window", "⌘⇧N"],
      ["Open", "⌘O"],
      ["Save As", "⇧⌘S"],
      ["Undo", "⌘Z"],
      ["Redo", "⇧⌘Z"],
      ["Data library", "⇧⌘L"],
      ["Data panel", "⇧⌘D"],
      ["Settings", "⌘,"],
    ],
  },
  {
    name: "Grid",
    shortcuts: [
      ["Edit selected cell", "F2"],
      ["Clear contents", "Delete"],
      ["Fill down", "⌘D"],
      ["Fill right", "⌘R"],
      ["Copy", "⌘C"],
      ["Cut", "⌘X"],
      ["Paste", "⌘V"],
    ],
  },
] as const;

export function KeyboardShortcutsDialog({ onClose }: { onClose: () => void }) {
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="insert-dialog preferences-dialog shortcuts-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">HELP</span>
            <h2>Keyboard shortcuts</h2>
          </div>
          <button
            className="icon-button"
            aria-label="Close keyboard shortcuts"
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </div>
        {GROUPS.map((group) => (
          <section className="shortcut-group" key={group.name}>
            <strong>{group.name}</strong>
            <dl>
              {group.shortcuts.map(([label, shortcut]) => (
                <div key={label}>
                  <dt>{label}</dt>
                  <dd>
                    <kbd>{shortcut}</kbd>
                  </dd>
                </div>
              ))}
            </dl>
          </section>
        ))}
        <div className="dialog-actions">
          <button className="primary-action" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
