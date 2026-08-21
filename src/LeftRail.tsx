import {
  Database,
  FileCog,
  FolderPlus,
  Frame,
  Library,
  Network,
  SquareFunction,
  Table2 as FrameIcon,
  Type,
} from "lucide-react";
import type { OperationHandler } from "./lib/handlers";
import type { LeftPanel } from "./App";

/**
 * The far-left navigation rail: which panel is open beside the canvas, and
 * the four kinds of object the canvas can hold. Pinned to the foot, below
 * everything it acts on, is Arrange — it rearranges the canvas rather than
 * opening or adding to it.
 */
export function LeftRail({
  leftPanel,
  setLeftPanel,
  toggleLeftPanel,
  onOpenLibrary,
  addBlock,
  addText,
  addEmptyFrame,
  addContainer,
  viewCount,
  onOperation,
}: {
  leftPanel: LeftPanel;
  setLeftPanel: (panel: LeftPanel) => void;
  toggleLeftPanel: (panel: Exclude<LeftPanel, null>) => void;
  onOpenLibrary: () => void;
  addBlock: (position?: { x: number; y: number }) => unknown;
  addText: (position?: { x: number; y: number }) => unknown;
  addEmptyFrame: (position?: { x: number; y: number }) => unknown;
  addContainer: (position?: { x: number; y: number }) => unknown;
  viewCount: number;
  onOperation: OperationHandler;
}) {
  return (
    <aside className="left-rail">
      {/* Decorative: the window's title bar already names the application,
          and every button below says what it is. */}
      <img className="rail-mark" src="/icon.svg" alt="" />
      <div className="rail-group">
        <span className="rail-group-label">View</span>
        <button
          className={`rail-button ${leftPanel === null ? "active" : ""}`}
          onClick={() => setLeftPanel(null)}
          title="The canvas on its own, with no panel beside it"
        >
          <Frame size={19} />
          <span>Canvas</span>
        </button>
        <button
          className={`rail-button ${leftPanel === "data" ? "active" : ""}`}
          onClick={() => toggleLeftPanel("data")}
          title="Every frame in this document, and where each one reads from (⇧⌘D)"
        >
          <Database size={19} />
          <span>Data</span>
        </button>
        <button
          className="rail-button"
          onClick={onOpenLibrary}
          title="Open a document, a sample workspace, or a data file (⇧⌘L)"
        >
          <Library size={19} />
          <span>Library</span>
        </button>
        <button
          className={`rail-button ${leftPanel === "project" ? "active" : ""}`}
          onClick={() => toggleLeftPanel("project")}
          title="Rename this document, and see where it is saved"
        >
          <FileCog size={19} />
          <span>Project</span>
        </button>
      </div>
      <div className="rail-rule" />
      <div className="rail-group">
        <span className="rail-group-label">Add</span>
        <button
          className="rail-button"
          onClick={() => void addBlock()}
          title="A page of formula lines: constants, calculations, and their answers (⌥⌘B)"
        >
          <SquareFunction size={19} />
          <span>Block</span>
        </button>
        <button
          className="rail-button"
          onClick={() => void addText()}
          title="A card of prose: markdown, with {{formula}} holes that print live values (⌥⌘T)"
        >
          <Type size={19} />
          <span>Text</span>
        </button>
        <button
          className="rail-button"
          onClick={() => void addEmptyFrame()}
          title="An empty frame to paste or type rows into (⌥⌘F)"
        >
          <FrameIcon size={19} />
          <span>Frame</span>
        </button>
        <button
          className="rail-button"
          onClick={() => void addContainer()}
          title="A resizable group for values, results, and lists (⌥⌘G)"
        >
          <FolderPlus size={19} />
          <span>Container</span>
        </button>
      </div>
      <div className="rail-spacer" />
      {/* Pinned to the foot, below everything it acts on: this one arranges
          the canvas rather than opening or adding to it. */}
      <button
        className="rail-button"
        disabled={viewCount < 2}
        onClick={() => void onOperation({ type: "tidyLayout" })}
        title="Arrange cards left to right by dependency, with each source before what it feeds (⇧⌘A)"
      >
        <Network size={19} />
        <span>Arrange</span>
      </button>
    </aside>
  );
}
