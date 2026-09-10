import { DatabaseConnectorDialog } from "./DatabaseConnectorDialog";
import { useState } from "react";
import type { SetFrameSourceHandler } from "./FrameGrid";
import { SourceSchemaChanges } from "./FrameSourcePanel";
import { ReadColumnTypes } from "./ReadColumnTypes";
import { ReadColumnsEditor } from "./ReadColumnsEditor";
import { connectorSourceLabel } from "./lib/dataSources";
import type { OperationHandler } from "./lib/handlers";
import { stepsFromRendered } from "./lib/pipelineChainEdits";
import { stepFormulas, STEP_LABELS } from "./lib/pipelineSteps";
import type { Column, ComputedFrame, FrameObject } from "./lib/types";

/** Read is always visible, including for entered data. A disconnected recipe
 * describes future work; the table below continues to show the saved result. */
export function FrameReadStep({ frame, computed, input, onSourceChanged, onOperation }: {
  frame: FrameObject;
  input?: { columns: Column[] } | null;
  computed: ComputedFrame;
  onSourceChanged: SetFrameSourceHandler;
  onOperation: OperationHandler;
}) {
  const [databaseOpen, setDatabaseOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const recipe = frame.disconnectedRead;
  const source = readSourceLabel(frame);
  return <section className="pipeline-step" aria-label="Read source">
    <div className="pipeline-step-heading"><span className="pipeline-step-index">0</span><strong>Read</strong></div>
    <div title={source}>{source.split(/[\\/]/).pop()}</div>
    {recipe && <p>Disconnected · saved result shown. Choose a new source to reuse the recipe.</p>}
    {!frame.derivation && !frame.generator && <button
      className="secondary-action" disabled={busy}
      onClick={() => {
        setBusy(true);
        void onSourceChanged(frame.id).then(setError).finally(() => setBusy(false));
      }}
    >{busy ? "Opening…" : "Replace source…"}</button>}
    {!frame.derivation && !frame.generator && <button className="secondary-action"
      onClick={() => setDatabaseOpen(true)}>Use database…</button>}
    {databaseOpen && <DatabaseConnectorDialog onClose={() => setDatabaseOpen(false)}
      onImport={async (source) => {
        const failure = await onSourceChanged(frame.id, source);
        if (failure) throw new Error(failure);
        setDatabaseOpen(false);
      }} />}
    {recipe && recipe.steps.length > 0 && <details>
      <summary>{recipe.steps.length} retained transformations</summary>
      <ol>{stepsFromRendered(computed.disconnectedSteps ?? [], frame, recipe.baseColumns.length ? recipe.baseColumns : recipe.columns).map((step) =>
        <li key={step.id}>{STEP_LABELS[step.kind]}{stepFormulas(step).map((formula) =>
          <div key={formula}>{formula}</div>)}</li>)}</ol>
    </details>}
    <ReadColumnTypes frame={frame} computed={computed}
      columns={input?.columns ?? (frame.baseColumns?.length ? frame.baseColumns : frame.columns)} onOperation={onOperation} />
    <ReadColumnsEditor key={JSON.stringify(frame.columns.map((c) => [c.id, c.name]))}
      frame={frame} onOperation={onOperation} />
    <SourceSchemaChanges frameId={frame.id} />
    {error && <p role="alert">{error}</p>}
  </section>;
}

function readSourceLabel(frame: FrameObject): string {
  const recipe = frame.disconnectedRead;
  return recipe?.source ?? (frame.connector ? connectorSourceLabel(frame.connector)
    : frame.fileOrigin?.path ?? frame.sourceFile ?? frame.artifact?.sourceName
    ?? (frame.derivation ? "Frame" : frame.generator ? "Sequence" : "Entered data"));
}
