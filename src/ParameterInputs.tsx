import { createContext, useContext, useRef, useState, type CSSProperties, type ReactNode } from "react";
import type { ParameterInput } from "./lib/bindings/ParameterInput";
import type { ScalarValue } from "./lib/bindings/ScalarValue";
import type { DocumentView } from "./lib/types";
import type { OperationHandler } from "./lib/handlers";
import "./parameter-inputs.css";

const ParameterContext = createContext<{
  inputs: ParameterInput[];
  onOperation: OperationHandler;
} | null>(null);

export function useParameterInputs() {
  return useContext(ParameterContext)?.inputs ?? [];
}

export function ParameterInputsProvider({ document, onOperation, children }: {
  document: DocumentView;
  onOperation: OperationHandler;
  children: ReactNode;
}) {
  return <ParameterContext.Provider value={{ inputs: document.parameterInputs ?? [], onOperation }}>
    {children}
  </ParameterContext.Provider>;
}

/** The engine projects constructor arguments; the frontend never decodes the AST.
 * Replacing a control's domain starts a fresh draft, but changing its value must
 * preserve focus so successive arrow-key adjustments remain one fluid interaction.
 */
export function ParameterAnswer({ id, children, className, style }: { id: string; children: ReactNode; className?: string; style?: CSSProperties }) {
  const context = useContext(ParameterContext);
  const input = context?.inputs.find((candidate) => candidate.id === id);
  return context && input
    ? <div className={className} style={style}><ParameterEditor key={JSON.stringify({ id, name: input.name, ...input.control, value: undefined })} input={input} onOperation={context.onOperation} /></div>
    : children;
}

function ParameterEditor({ input, onOperation }: { input: ParameterInput; onOperation: OperationHandler }) {
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const submitted = useRef<string | null>(null);
  const commit = async (value: ScalarValue) => {
    const serialized = JSON.stringify(value);
    if (serialized === submitted.current) return;
    submitted.current = serialized;
    setPending(true);
    try {
      const failure = await onOperation({ type: "setParameterValue", objectId: input.id, value }, { inlineError: true });
      setError(failure);
      if (failure) submitted.current = null;
    } catch (reason) {
      setError(String(reason));
      submitted.current = null;
    } finally { submitted.current = null; setPending(false); }
  };
  const control = input.control;
  return <span className="parameter-input" onPointerDown={(event) => event.stopPropagation()} onClick={(event) => event.stopPropagation()}>
    {control.type === "slider" ? <SliderInput name={input.name} control={control} disabled={pending} commit={commit} />
      : control.type === "dropdown" ? <select aria-label={`${input.name} choice`} aria-disabled={pending}
          value={control.options.findIndex((option) => JSON.stringify(option) === JSON.stringify(control.value))}
          onChange={(event) => { if (!pending) void commit(control.options[Number(event.target.value)]); }}>
          {control.options.map((option, index) => <option key={index} value={index}>{scalarLabel(option)}</option>)}
        </select>
      : <DateInput name={input.name} value={control.value} pending={pending} commit={commit} />}
    {error && <span className="parameter-error" role="status">{error}</span>}
  </span>;
}

function scalarLabel(value: ScalarValue): string {
  return value.type === "null" ? "" : String(value.value);
}

function useParameterDraft(value: string) {
  const [stored, setStored] = useState(value);
  const [draft, setDraft] = useState(value);
  if (stored !== value) { setStored(value); setDraft(value); }
  return [draft, setDraft] as const;
}

function DateInput({ name, value, pending, commit }: { name: string; value: string; pending: boolean; commit: (value: ScalarValue) => Promise<void> }) {
  const [draft, setDraft] = useParameterDraft(value);
  return <input type="date" aria-label={`${name} date`} value={draft} readOnly={pending}
    onChange={(event) => setDraft(event.target.value)}
    onBlur={() => { if (!draft) setDraft(value); else if (draft !== value) void commit({ type: "date", value: draft }); }}
    onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }} />;
}

function SliderInput({ name, control, disabled, commit }: {
  name: string;
  control: Extract<ParameterInput["control"], { type: "slider" }>;
  disabled: boolean;
  commit: (value: ScalarValue) => Promise<void>;
}) {
  const [draft, setDraft] = useParameterDraft(String(control.value));
  const submit = () => {
    const value = Number(draft);
    if (!draft.trim() || !Number.isFinite(value)) { setDraft(String(control.value)); return; }
    if (value !== control.value) void commit({ type: "number", value });
  };
  return <>
    <input type="range" aria-label={`${name} slider`} min={control.start} max={control.stop} step={control.step}
      value={Number(draft)} aria-disabled={disabled} onChange={(event) => { if (!disabled) setDraft(event.target.value); }}
      onPointerUp={submit} onKeyUp={submit} onBlur={submit}
      onPointerCancel={() => setDraft(String(control.value))} />
    <input type="number" aria-label={`${name} value`} min={control.start} max={control.stop} step="any"
      value={draft} readOnly={disabled} onChange={(event) => setDraft(event.target.value)} onBlur={submit}
      onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }} />
  </>;
}
