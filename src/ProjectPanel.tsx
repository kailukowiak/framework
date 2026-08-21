import { FileSpreadsheet, Package, Save, SaveAll, Trash2, X } from "lucide-react";
import { useState } from "react";
import { ExcelExportDialog } from "./ExcelExportDialog";
import { Field } from "./Field";
import { exportDocumentExcel } from "./lib/api";
import type { OperationHandler } from "./lib/handlers";
import type { DocumentView } from "./lib/types";

/**
 * The document itself: its name, its file, and the two things you can do to
 * the whole of it.
 *
 * The name used to be an input in the top-left corner of the window, which is
 * the most expensive space there is for something you set once and then read.
 * Down here it has room for what was missing beside it — where the file
 * actually is, and whether there is one at all. That last question is the
 * only one the status chip up top can still answer on its own, and pressing
 * the chip lands here.
 */
export function ProjectPanel({
  document,
  path,
  onClose,
  onOperation,
  onSaveAs,
  onPackage,
  onCompact,
}: {
  document: DocumentView;
  path: string | null;
  onClose: () => void;
  onOperation: OperationHandler;
  onSaveAs: () => Promise<void>;
  onPackage: () => Promise<void>;
  onCompact: () => Promise<void>;
}) {
  const [excelExport, setExcelExport] = useState(false);
  const frames = document.objects.filter((object) => object.kind === "frame");
  const linked = frames.filter(
    (frame) => frame.kind === "frame" && frame.connector
  ).length;

  return (
    <aside className="data-sidebar project-panel">
      <div className="data-sidebar-header">
        <div>
          <span className="eyebrow">PROJECT</span>
          <h2>{document.name || "Untitled"}</h2>
        </div>
        <button className="icon-button" onClick={onClose} aria-label="Close">
          <X size={17} />
        </button>
      </div>

      <Field
        label="Document name"
        initial={document.name}
        onCommit={(name) => {
          const trimmed = name.trim();
          // A blank name would leave the window titled after nothing at all,
          // and the field re-keys off the document, so refusing it here puts
          // the old one straight back.
          if (trimmed && trimmed !== document.name)
            void onOperation({ type: "renameDocument", name: trimmed });
        }}
      />

      <div className={`project-file ${path ? "" : "unsaved"}`}>
        <Save size={15} />
        <span>
          <strong>{path ? "Saved locally" : "Not saved to a file"}</strong>
          <small title={path ?? undefined}>
            {path ?? "Everything here lives in a scratch document until you save it"}
          </small>
        </span>
      </div>
      {!path && (
        <button className="secondary-action project-action" onClick={() => void onSaveAs()}>
          <SaveAll size={14} /> Save As…
        </button>
      )}

      <div className="project-counts">
        <div>
          <strong>{frames.length.toLocaleString()}</strong>
          <span>{frames.length === 1 ? "frame" : "frames"}</span>
        </div>
        <div>
          <strong>{linked.toLocaleString()}</strong>
          <span>linked to files</span>
        </div>
        <div>
          <strong>{document.views.length.toLocaleString()}</strong>
          <span>{document.views.length === 1 ? "card" : "cards"}</span>
        </div>
      </div>

      {/* Both act on the document as a whole rather than on anything you can
          select, which is why neither belongs in the inspector. */}
      <div className="data-sidebar-group-heading">
        <strong>Whole document</strong>
      </div>
      <button
        className="secondary-action project-action"
        onClick={() => setExcelExport(true)}
      >
        <FileSpreadsheet size={14} /> Export to Excel…
      </button>
      <p className="project-note">
        Writes selected tables and named answers to an Excel workbook.
      </p>
      <button className="secondary-action project-action" onClick={() => void onPackage()}>
        <Package size={14} /> Package this document
      </button>
      <p className="project-note">
        Copies every file this document reads into its own folder, so nothing it
        needs lives anywhere else.
      </p>
      <button className="secondary-action project-action" onClick={() => void onCompact()}>
        <Trash2 size={14} /> Reclaim unused data files
      </button>
      <p className="project-note">
        Deletes staged data no frame points at any more. Nothing on the canvas
        changes.
      </p>
      {excelExport && (
        <ExcelExportDialog
          document={document}
          onClose={() => setExcelExport(false)}
          onExport={async (frameIds) => Boolean(await exportDocumentExcel(frameIds))}
        />
      )}
    </aside>
  );
}
