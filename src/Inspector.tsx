import { Info, X, PanelRightClose } from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import { Field } from "./Field";
import { FrameInspector } from "./FrameInspector";
import type {
  AddCalculatedColumnEditorRequest,
  ApplyVectorEditorRequest,
  FilterColumnEditorRequest,
  HidePipelineColumnEditorRequest,
  PairVectorEditorRequest,
  RearrangeColumnsEditorRequest,
  TransformColumnEditorRequest,
} from "./hooks/usePipelineColumnRequests";
import type {
  FreezeCopyHandler,
  SetFrameCachedHandler,
  SetFrameSourceHandler,
  TakeOwnershipHandler,
} from "./FrameGrid";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  ComputedFrame,
  DataObject,
  FormulaFunction,
  FrameObject,
  Scenario,
  Selection,
} from "./lib/types";
import { PlotInspector, ValueInspector } from "./PlotInspector";

export type InspectorSection = "selection" | "format" | "wrangle";

const sectionLabels: Record<InspectorSection, string> = {
  selection: "Selection",
  format: "Format",
  wrangle: "Wrangle",
};

type InspectorProps = {
  hidden?: boolean;
  documentId: string;
  object: DataObject;
  objects: DataObject[];
  /** Every scenario in the document, for a selected value's override table. */
  scenarios: Scenario[];
  formulaFunctions: FormulaFunction[];
  selection: Selection;
  /** Columns under the grid selection, when it spans more than the active one. */
  selectedColumnIds?: string[];
  computed?: ComputedFrame;
  suggestedPosition: { x: number; y: number };
  onClose: () => void;
  /** Hides the panel without dropping the selection; ⌘⇧I or the menu brings it back. */
  onHide: () => void;
  section: InspectorSection;
  onSectionChange: Dispatch<SetStateAction<InspectorSection>>;
  addCalculatedColumnRequest?: AddCalculatedColumnEditorRequest;
  onAddCalculatedColumnRequestHandled: () => void;
  transformColumnRequest?: TransformColumnEditorRequest;
  onTransformColumnRequestHandled: () => void;
  filterColumnRequest?: FilterColumnEditorRequest;
  onFilterColumnRequestHandled: () => void;
  hidePipelineColumnRequest?: HidePipelineColumnEditorRequest;
  onHidePipelineColumnRequestHandled: () => void;
  rearrangeColumnsRequest?: RearrangeColumnsEditorRequest;
  onRearrangeColumnsRequestHandled: () => void;
  applyVectorRequest?: ApplyVectorEditorRequest;
  onApplyVectorRequestHandled: () => void;
  pairVectorRequest?: PairVectorEditorRequest;
  onPairVectorRequestHandled: () => void;
  onOperation: OperationHandler;
  onSourceChanged: SetFrameSourceHandler;
  onSetCached: SetFrameCachedHandler;
  onTakeOwnership: TakeOwnershipHandler;
  onFreezeCopy: FreezeCopyHandler;
  onJoin: () => void;
  onTransformColumn: (column: Column, formula: string, focus?: boolean) => void;
};

export function Inspector({
  hidden,
  documentId,
  object,
  objects,
  scenarios,
  formulaFunctions,
  selection,
  selectedColumnIds,
  computed,
  suggestedPosition,
  onClose,
  onHide,
  section,
  onSectionChange,
  addCalculatedColumnRequest,
  onAddCalculatedColumnRequestHandled,
  transformColumnRequest,
  onTransformColumnRequestHandled,
  filterColumnRequest,
  onFilterColumnRequestHandled,
  hidePipelineColumnRequest,
  onHidePipelineColumnRequestHandled,
  rearrangeColumnsRequest,
  onRearrangeColumnsRequestHandled,
  applyVectorRequest,
  onApplyVectorRequestHandled,
  pairVectorRequest,
  onPairVectorRequestHandled,
  onOperation,
  onSourceChanged,
  onSetCached,
  onTakeOwnership,
  onFreezeCopy,
  onJoin,
  onTransformColumn,
}: InspectorProps) {
  return (
    // Keep the editor owner mounted while collapsed: its PipelineCommands
    // register the shared draft edited by the visible top bar. Unmounting
    // the panel would force every grid formula gesture to reopen it.
    <aside className="inspector" hidden={hidden}>
      <InspectorHeader name={object.name} onHide={onHide} onClose={onClose} />
      {object.kind === "frame" && (
        <nav className="inspector-nav" aria-label="Inspector sections">
          {(["selection", "format", "wrangle"] as InspectorSection[]).map(
            (candidate) => (
              <button
                key={candidate}
                className={section === candidate ? "active" : ""}
                aria-label={sectionLabels[candidate]}
                aria-pressed={section === candidate}
                onClick={() => onSectionChange(candidate)}
                data-shortcut={`⌘${
                  candidate === "selection" ? "1" : candidate === "format" ? "2" : "3"
                }`}
                title={`${sectionLabels[candidate]} (⌘${
                  candidate === "selection" ? "1" : candidate === "format" ? "2" : "3"
                })`}
              >
                {sectionLabels[candidate]}
              </button>
            )
          )}
        </nav>
      )}
      {object.kind === "value" && (
        <ValueInspector
          value={object}
          scenarios={scenarios}
          onOperation={onOperation}
        />
      )}
      {object.kind === "container" && (
        <section className="inspector-section">
          <Field
            label="Name"
            initial={object.name}
            help="Members are written as `Name`.`Member` in formulas."
            onCommit={(name) =>
              void onOperation({ type: "renameObject", objectId: object.id, name })
            }
          />
          <p className="inspector-note">
            {object.memberIds.length === 0
              ? "Nothing in here yet. Use + Value, + Result, or + Vector on the card."
              : `${object.memberIds.length} ${object.memberIds.length === 1 ? "member" : "members"}. Click one on the card to inspect it.`}
          </p>
        </section>
      )}
      {/* A vector is edited on its card, where the whole of it is visible. The
          inspector says the two things the card cannot: what it is called in
          a formula, and how to use it. */}
      {object.kind === "series" && (
        <section className="inspector-section">
          <h3>Vector</h3>
          <p className="inspector-note">
            {object.values.length} {object.values.length === 1 ? "value" : "values"} ·{" "}
            {object.dataType}
          </p>
          <p className="inspector-note">
            Write <code>`{object.name}`</code> in a formula to pass it to something that
            takes a vector, like <code>.is_in()</code>.
          </p>
        </section>
      )}
      {object.kind === "frame" && (
        <FrameInspector
          documentId={documentId}
          frame={object}
          objects={objects}
          formulaFunctions={formulaFunctions}
          selection={selection}
          selectedColumnIds={selectedColumnIds}
          computed={computed!}
          suggestedPosition={suggestedPosition}
          section={section}
          addCalculatedColumnRequest={addCalculatedColumnRequest}
          onAddCalculatedColumnRequestHandled={onAddCalculatedColumnRequestHandled}
          transformColumnRequest={transformColumnRequest}
          onTransformColumnRequestHandled={onTransformColumnRequestHandled}
          filterColumnRequest={filterColumnRequest}
          onFilterColumnRequestHandled={onFilterColumnRequestHandled}
          hidePipelineColumnRequest={hidePipelineColumnRequest}
          onHidePipelineColumnRequestHandled={onHidePipelineColumnRequestHandled}
          rearrangeColumnsRequest={rearrangeColumnsRequest}
          onRearrangeColumnsRequestHandled={onRearrangeColumnsRequestHandled}
          applyVectorRequest={applyVectorRequest}
          onApplyVectorRequestHandled={onApplyVectorRequestHandled}
          pairVectorRequest={pairVectorRequest}
          onPairVectorRequestHandled={onPairVectorRequestHandled}
          onOperation={onOperation}
          onSourceChanged={onSourceChanged}
          onSetCached={onSetCached}
          onTakeOwnership={onTakeOwnership}
          onFreezeCopy={onFreezeCopy}
          onJoin={onJoin}
          onTransformColumn={onTransformColumn}
        />
      )}
      {object.kind === "plot" && (
        <PlotInspectorForSource
          object={object}
          objects={objects}
          onOperation={onOperation}
        />
      )}
    </aside>
  );
}

/**
 * The inspector collapses back into the edge it came from. Keeping this as a
 * full-height sliver makes the hidden state visible without putting a second
 * inspector control in unrelated navigation on the other side of the canvas.
 */
export function CollapsedInspector({ onShow }: { onShow: () => void }) {
  return (
    <aside className="inspector-collapsed" aria-label="Collapsed inspector">
      <button
        className="inspector-collapsed-toggle"
        aria-label="Show inspector"
        data-shortcut="⇧⌘I"
        title="Show inspector (⌘⇧I)"
        onClick={onShow}
      >
        <Info size={16} />
      </button>
    </aside>
  );
}

function PlotInspectorForSource({
  object,
  objects,
  onOperation,
}: {
  object: Extract<DataObject, { kind: "plot" }>;
  objects: DataObject[];
  onOperation: OperationHandler;
}) {
  const frame = objects.find(
    (candidate): candidate is FrameObject =>
      candidate.kind === "frame" && candidate.id === object.sourceFrameId
  );
  return frame ? (
    <PlotInspector plot={object} frame={frame} onOperation={onOperation} />
  ) : null;
}

function InspectorHeader({ name, onHide, onClose }: {
  name: string;
  onHide: () => void;
  onClose: () => void;
}) {
  return (
    <div className="inspector-header">
      <div>
        <span className="eyebrow">INSPECTOR</span>
        <h2>{name || "Unnamed object"}</h2>
      </div>
      <button
        className="icon-button"
        aria-label="Hide inspector"
        data-shortcut="⇧⌘I"
        title="Hide inspector (⌘⇧I)"
        onClick={onHide}
      >
        <PanelRightClose size={14} />
      </button>
      <button className="icon-button" aria-label="Close inspector" onClick={onClose}>
        <X size={17} />
      </button>
    </div>
  );
}
