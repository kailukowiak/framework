import { BookOpen, ChevronRight, RotateCcw } from "lucide-react";
import type { TutorialLibrary } from "./lib/api";
import { libraryEntryState } from "./lib/datasetLibraryEntries";

/**
 * The twelve tutorial start/answer-key workbooks, copied into a visible
 * Documents folder on request. A workbook that exists but that FrameWork
 * cannot open — most often a stored macOS TCC deny under `tauri dev` — stays
 * in the list rather than vanishing, disabled and labelled instead.
 */
export function TutorialWorkbookLibrary({
  tutorials,
  opening,
  confirmReset,
  onCreate,
  onReset,
  onRequestReset,
  onOpen,
}: {
  tutorials: TutorialLibrary | null;
  opening: string | null;
  confirmReset: boolean;
  onCreate: () => Promise<void>;
  onReset: () => Promise<void>;
  onRequestReset: () => void;
  onOpen: (path: string) => Promise<void>;
}) {
  const hasTutorials = tutorials?.documents.some((tutorial) => tutorial.exists);
  return (
    <>
      <div className="dataset-section-heading dataset-nested-heading">
        <strong>Tutorial workbooks</strong>
        <span>Editable copies</span>
      </div>
      <div className="tutorial-library">
        <p className="tutorial-library-note">
          Create the twelve start and answer-key workbooks in{" "}
          <code>{tutorials?.directory ?? "your Documents folder"}</code>.
        </p>
        <div className="tutorial-library-actions">
          <button
            className="secondary-action"
            disabled={opening !== null}
            onClick={() => void onCreate()}
          >
            <BookOpen size={14} />
            {opening === "__tutorials__" ? "Creating…" : "Create tutorials"}
          </button>
          {hasTutorials && (
            <button
              className={confirmReset ? "danger-action" : "secondary-action"}
              disabled={opening !== null}
              onClick={() => {
                if (confirmReset) void onReset();
                else onRequestReset();
              }}
            >
              <RotateCcw size={14} />
              {opening === "__tutorial-reset__"
                ? "Resetting…"
                : confirmReset
                  ? "Replace all tutorial workbooks"
                  : "Reset tutorials…"}
            </button>
          )}
        </div>
        {confirmReset && (
          <p className="tutorial-reset-note">
            Replaces only these twelve workbooks and their histories. Notes or other
            files in this folder stay untouched.
          </p>
        )}
        {hasTutorials && (
          <div className="recent-document-list tutorial-document-list">
            {tutorials!.documents
              .filter((tutorial) => tutorial.exists)
              .map((tutorial) => {
                const state = libraryEntryState(tutorial);
                return (
                  <button
                    className="recent-document"
                    key={tutorial.path}
                    disabled={opening !== null || state.disabled}
                    onClick={() => void onOpen(tutorial.path)}
                  >
                    <span className="sample-icon">
                      <BookOpen size={16} />
                    </span>
                    <span>
                      <strong>
                        {tutorial.title}
                        {state.suffix && (
                          <span className="library-entry-suffix"> — {state.suffix}</span>
                        )}
                      </strong>
                      <small>{tutorial.path}</small>
                    </span>
                    <ChevronRight size={14} />
                  </button>
                );
              })}
          </div>
        )}
      </div>
    </>
  );
}
