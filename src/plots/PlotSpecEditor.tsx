import { useEffect, useState } from "react";
import { CircleAlert, Play } from "lucide-react";
import type { PlotObject } from "../lib/types";
import type { OperationHandler } from "../lib/handlers";

export function PlotSpecEditor({ plot, onOperation }: { plot: PlotObject; onOperation: OperationHandler }) {
  const [draft, setDraft] = useState(() => JSON.stringify(plot.spec, null, 2));
  const [specError, setSpecError] = useState<string | null>(null);
  useEffect(() => setDraft(JSON.stringify(plot.spec, null, 2)), [plot.spec]);
  return <>
          <label className="inspector-field">
            Vega-Lite JSON
            <textarea
              className="plot-spec-editor"
              value={draft}
              spellCheck={false}
              onChange={(event) => setDraft(event.target.value)}
            />
          </label>
          {specError && (
            <div className="formula-error">
              <CircleAlert size={14} />
              {specError}
            </div>
          )}
          <button
            className="primary-action"
            onClick={() => {
              try {
                const parsed = JSON.parse(draft);
                if (!parsed || typeof parsed !== "object" || Array.isArray(parsed))
                  throw new Error("The specification must be a JSON object");
                setSpecError(null);
                void onOperation(
                  { type: "setPlotSpec", plotId: plot.id, spec: parsed },
                  { inlineError: true }
                ).then((failure) => failure && setSpecError(failure));
              } catch (reason) {
                setSpecError(String(reason).replace(/^SyntaxError:\s*/, ""));
              }
            }}
          >
            <Play size={14} /> Apply specification
          </button>
          <p className="plot-spec-note">
            FrameWork supplies the source frame at render time, so the top-level data
            property is not persisted into the chart preview.
          </p>
  </>;
}
