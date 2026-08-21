import {
  ArrowDownToLine,
  BookOpen,
  ChevronDown,
  ChevronRight,
  CircleAlert,
  Cloud,
  Code2,
  Database,
  FolderOpen,
  RotateCcw,
  Table2 as FrameIcon,
  X,
} from "lucide-react";
import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import {
  createTutorialDocuments,
  listRecentDocuments,
  listSampleDocuments,
  listTutorialDocuments,
  openDocument,
  openDocumentDialog,
  openSampleDocument,
  resetTutorialDocuments,
  type RecentDocument,
  type SampleDocument,
  type TutorialLibrary,
  type CliSourceInput,
  type DatabaseSourceInput,
} from "./lib/api";
import type { DocumentView, FrameObject } from "./lib/types";
import {
  CliConnectorDialog,
  type CommandSourceKind,
} from "./CliConnectorDialog";
import { connectorSourceLabel } from "./lib/dataSources";
import { DatabaseConnectorDialog } from "./DatabaseConnectorDialog";

type AddDataSourceKind = "database" | CommandSourceKind;

async function runImportPicker(
  key: string,
  open: () => Promise<boolean>,
  setOpening: Dispatch<SetStateAction<string | null>>,
  setError: Dispatch<SetStateAction<string | null>>
) {
  setOpening(key);
  setError(null);
  try {
    if (!(await open())) setOpening(null);
  } catch (reason) {
    setError(String(reason).replace(/^Error:\s*/, ""));
    setOpening(null);
  }
}

function DatasetImportActions({
  disabled,
  onImportFile,
  onImportExcelFile,
  onImportCommandSource,
}: {
  disabled: boolean;
  onImportFile: () => void;
  onImportExcelFile: () => void;
  onImportCommandSource: (kind: AddDataSourceKind) => void;
}) {
  return (
    <div className="dataset-import-row">
      <div>
        <strong>Add data</strong>
      </div>
      <div className="dataset-import-actions">
        <button className="secondary-action" disabled={disabled} onClick={onImportFile}>
          <FolderOpen size={14} /> Flat…
        </button>
        <button className="secondary-action" disabled={disabled} onClick={onImportExcelFile}>
          <FrameIcon size={14} /> Excel…
        </button>
        <button className="secondary-action" disabled={disabled} onClick={() => onImportCommandSource("database")}>
          <Database size={14} /> DB…
        </button>
        <button className="secondary-action" disabled={disabled} onClick={() => onImportCommandSource("api")}>
          <Cloud size={14} /> API…
        </button>
        <button className="secondary-action" disabled={disabled} onClick={() => onImportCommandSource("script")}>
          <Code2 size={14} /> Script…
        </button>
      </div>
    </div>
  );
}

function connectedFrames(document: DocumentView): FrameObject[] {
  return document.objects.filter(
    (object): object is FrameObject => object.kind === "frame" && Boolean(object.connector)
  );
}

function ConnectedSources({
  frames,
  opening,
  onChange,
}: {
  frames: FrameObject[];
  opening: string | null;
  onChange: (frame: FrameObject) => void;
}) {
  if (frames.length === 0) return null;
  return (
    <>
      <div className="dataset-section-heading">
        <strong>Connected sources</strong>
        <span>In this document</span>
      </div>
      <div className="recent-document-list">
        {frames.map((frame) => (
          <div className="connected-source" key={frame.id}>
            <span className="sample-icon"><FrameIcon size={16} /></span>
            <span>
              <strong>{frame.name}</strong>
              <small title={connectorSourceLabel(frame.connector!)}>
                {connectorSourceLabel(frame.connector!)}
              </small>
            </span>
            {frame.connector!.kind === "file" && (
              <button
                className="secondary-action"
                disabled={opening !== null}
                title={`Read ${frame.name} from a different file`}
                onClick={() => onChange(frame)}
              >
                <FolderOpen size={13} /> {opening === frame.id ? "Opening…" : "Change…"}
              </button>
            )}
          </div>
        ))}
      </div>
    </>
  );
}

function TutorialWorkbookLibrary({
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
          Create the eight start and answer-key workbooks in{" "}
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
            Replaces only these eight workbooks and their histories. Notes or other
            files in this folder stay untouched.
          </p>
        )}
        {hasTutorials && (
          <div className="recent-document-list tutorial-document-list">
            {tutorials!.documents
              .filter((tutorial) => tutorial.exists)
              .map((tutorial) => (
                <button
                  className="recent-document"
                  key={tutorial.path}
                  disabled={opening !== null}
                  onClick={() => void onOpen(tutorial.path)}
                >
                  <span className="sample-icon">
                    <BookOpen size={16} />
                  </span>
                  <span>
                    <strong>{tutorial.title}</strong>
                    <small>{tutorial.path}</small>
                  </span>
                  <ChevronRight size={14} />
                </button>
              ))}
          </div>
        )}
      </div>
    </>
  );
}

function SampleDocumentLibrary({
  samples,
  loading,
  opening,
  onOpen,
}: {
  samples: SampleDocument[];
  loading: boolean;
  opening: string | null;
  onOpen: (sample: SampleDocument) => Promise<void>;
}) {
  return (
    <>
      <div className="dataset-section-heading dataset-nested-heading">
        <strong>Example documents</strong>
        <span>.framework-samples/*.fw</span>
      </div>
      <div className="sample-grid">
        <p className="sample-note">
          Opening one makes a fresh working copy; the file on disk is left alone.
        </p>
        {loading && <div className="sample-loading">Looking for local samples…</div>}
        {!loading && samples.length === 0 && (
          <div className="sample-loading">
            No sample files found in .framework-samples.
          </div>
        )}
        {samples.map((sample) => (
          <button
            className="sample-card"
            key={sample.fileName}
            disabled={opening !== null}
            onClick={() => void onOpen(sample)}
          >
            <span className="sample-icon">
              <FrameIcon size={17} />
            </span>
            <span>
              <strong>{sample.title}</strong>
              <small>
                {sample.category} · {sample.frameCount} frame
                {sample.frameCount === 1 ? "" : "s"} · {sample.fileName}
              </small>
            </span>
            <ArrowDownToLine size={14} />
          </button>
        ))}
      </div>
    </>
  );
}

function LearningLibrary({
  samples,
  tutorials,
  loading,
  opening,
  confirmReset,
  onCreateTutorials,
  onResetTutorials,
  onRequestReset,
  onOpenTutorial,
  onOpenSample,
}: {
  samples: SampleDocument[];
  tutorials: TutorialLibrary | null;
  loading: boolean;
  opening: string | null;
  confirmReset: boolean;
  onCreateTutorials: () => Promise<void>;
  onResetTutorials: () => Promise<void>;
  onRequestReset: () => void;
  onOpenTutorial: (path: string) => Promise<void>;
  onOpenSample: (sample: SampleDocument) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button
        type="button"
        className="dataset-section-heading dataset-section-toggle"
        aria-expanded={open}
        onClick={() => setOpen((expanded) => !expanded)}
      >
        <span className="dataset-section-title">
          {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <strong>Tutorials and examples</strong>
        </span>
        <span>Learn and explore</span>
      </button>
      {open && (
        <>
          <TutorialWorkbookLibrary
            tutorials={tutorials}
            opening={opening}
            confirmReset={confirmReset}
            onCreate={onCreateTutorials}
            onReset={onResetTutorials}
            onRequestReset={onRequestReset}
            onOpen={onOpenTutorial}
          />
          <SampleDocumentLibrary
            samples={samples}
            loading={loading}
            opening={opening}
            onOpen={onOpenSample}
          />
        </>
      )}
    </>
  );
}

export function DatasetDialog({
  document,
  onClose,
  onImportFile,
  onImportExcelFile,
  onImportCliSource,
  onImportDatabaseSource,
  onSourceChanged,
  onOpened,
}: {
  document: DocumentView;
  onClose: () => void;
  onImportFile: () => Promise<boolean>;
  onImportExcelFile: () => Promise<boolean>;
  onImportCliSource: (source: CliSourceInput) => Promise<void>;
  onImportDatabaseSource: (source: DatabaseSourceInput) => Promise<void>;
  onSourceChanged: (frameId: string) => Promise<string | null>;
  onOpened: (opened: { document: DocumentView; path: string | null }) => void;
}) {
  const [samples, setSamples] = useState<SampleDocument[]>([]);
  const [recents, setRecents] = useState<RecentDocument[]>([]);
  const [tutorials, setTutorials] = useState<TutorialLibrary | null>(null);
  const [loading, setLoading] = useState(true);
  const [opening, setOpening] = useState<string | null>(null);
  const [libraryError, setLibraryError] = useState<string | null>(null);
  const [confirmTutorialReset, setConfirmTutorialReset] = useState(false);
  const [commandSource, setCommandSource] = useState<AddDataSourceKind | null>(null);
  // Every frame in this document that reads from a file. Listing them
  // together is the difference between fixing one moved file and fixing a
  // project someone moved wholesale.
  const documentConnections = connectedFrames(document);

  useEffect(() => {
    let disposed = false;
    void Promise.all([
      listSampleDocuments(),
      listRecentDocuments(),
      listTutorialDocuments(),
    ])
      .then(([sampleItems, recentItems, tutorialLibrary]) => {
        if (!disposed) {
          setSamples(sampleItems);
          setRecents(recentItems);
          setTutorials(tutorialLibrary);
        }
      })
      .catch((reason) => {
        if (!disposed) setLibraryError(String(reason).replace(/^Error:\s*/, ""));
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, []);

  const openSample = async (sample: SampleDocument) => {
    setOpening(sample.fileName);
    setLibraryError(null);
    try {
      onOpened(await openSampleDocument(sample.fileName));
    } catch (reason) {
      setLibraryError(String(reason).replace(/^Error:\s*/, ""));
      setOpening(null);
    }
  };

  const openRecent = async (recent: RecentDocument) => {
    setOpening(recent.path);
    setLibraryError(null);
    try {
      onOpened(await openDocument(recent.path));
    } catch (reason) {
      setLibraryError(String(reason).replace(/^Error:\s*/, ""));
      setOpening(null);
    }
  };

  const openTutorial = async (path: string) => {
    setOpening(path);
    setLibraryError(null);
    try {
      onOpened(await openDocument(path));
    } catch (reason) {
      setLibraryError(String(reason).replace(/^Error:\s*/, ""));
      setOpening(null);
    }
  };

  const createTutorials = async () => {
    setOpening("__tutorials__");
    setLibraryError(null);
    try {
      setTutorials(await createTutorialDocuments());
    } catch (reason) {
      setLibraryError(String(reason).replace(/^Error:\s*/, ""));
    } finally {
      setOpening(null);
    }
  };

  const resetTutorials = async () => {
    setOpening("__tutorial-reset__");
    setLibraryError(null);
    try {
      setTutorials(await resetTutorialDocuments());
      setConfirmTutorialReset(false);
    } catch (reason) {
      setLibraryError(String(reason).replace(/^Error:\s*/, ""));
    } finally {
      setOpening(null);
    }
  };

  const chooseDocument = async () => {
    setOpening("__path__");
    setLibraryError(null);
    try {
      const opened = await openDocumentDialog();
      if (opened) onOpened(opened);
      else setOpening(null);
    } catch (reason) {
      setLibraryError(String(reason).replace(/^Error:\s*/, ""));
      setOpening(null);
    }
  };

  const changeSource = async (frame: FrameObject) => {
    setOpening(frame.id);
    setLibraryError(null);
    setLibraryError(await onSourceChanged(frame.id));
    setOpening(null);
  };

  return (
    <>
    <div
      className="dialog-backdrop dataset-dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="insert-dialog dataset-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">DATA LIBRARY</span>
            <h2>Open a workspace or add data</h2>
          </div>
          <button className="icon-button" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        {/* These fixed actions belong before the variable-length document
            lists regardless of how the library was opened. */}
        <DatasetImportActions
          disabled={opening !== null}
          onImportFile={() => void runImportPicker("__import__", onImportFile, setOpening, setLibraryError)}
          onImportExcelFile={() => void runImportPicker("__excel__", onImportExcelFile, setOpening, setLibraryError)}
          onImportCommandSource={setCommandSource}
        />
        <button
          className="dataset-file-picker dataset-document-picker"
          disabled={opening !== null}
          onClick={() => void chooseDocument()}
        >
          <FrameIcon size={15} />
          <span>
            <strong>Choose another FrameWork document</strong>
          </span>
          <ArrowDownToLine size={14} />
        </button>
        <div className="dataset-section-heading">
          <strong>Recent documents</strong>
          <span>On this device</span>
        </div>
        <div className="recent-document-list">
          {loading && (
            <div className="sample-loading">Looking for recent documents…</div>
          )}
          {!loading && recents.length === 0 && (
            <div className="sample-loading">
              Documents you open or create will appear here.
            </div>
          )}
          {recents.map((recent) => (
            <button
              className="recent-document"
              key={recent.path}
              disabled={opening !== null}
              onClick={() => void openRecent(recent)}
            >
              <span className="sample-icon">
                <FolderOpen size={16} />
              </span>
              <span>
                <strong>{recent.title}</strong>
                <small>{recent.path}</small>
              </span>
              <ChevronRight size={14} />
            </button>
          ))}
        </div>
        <LearningLibrary
          samples={samples}
          tutorials={tutorials}
          loading={loading}
          opening={opening}
          confirmReset={confirmTutorialReset}
          onCreateTutorials={createTutorials}
          onResetTutorials={resetTutorials}
          onRequestReset={() => setConfirmTutorialReset(true)}
          onOpenTutorial={openTutorial}
          onOpenSample={openSample}
        />
        <ConnectedSources
          frames={documentConnections}
          opening={opening}
          onChange={(frame) => void changeSource(frame)}
        />
        {libraryError && (
          <div className="formula-editor-error">
            <CircleAlert size={12} />
            <span>{libraryError}</span>
          </div>
        )}
        <div className="dialog-actions">
          <button className="secondary-action" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
    {commandSource === "database" && (
      <DatabaseConnectorDialog
        onClose={() => setCommandSource(null)}
        onImport={onImportDatabaseSource}
      />
    )}
    {commandSource && commandSource !== "database" && (
      <CliConnectorDialog
        kind={commandSource}
        onClose={() => setCommandSource(null)}
        onImport={onImportCliSource}
      />
    )}
    </>
  );
}
