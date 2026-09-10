import { useState } from "react";
import { ArrowLeftRight } from "lucide-react";
import { ContextMenuGroup } from "./ContextMenuSurface";
import "./MappingFrameMenu.css";
import type { OperationHandler } from "./lib/handlers";
import { dictionaryColumns } from "./lib/dictionaries";
import { formulaToken } from "./lib/formulaReferences";
import type { Column, DocumentView, FrameObject } from "./lib/types";

/** Both actions use the ordinary frame grid. Value mapping stays a live
 * formula; header renaming resolves the mapping once as an undoable edit. */
export function MappingFrameMenu({ document, frame, column, position, onMap, run, close, rename = false }: {
  document: DocumentView; frame: FrameObject; column: Column | null;
  position: { x: number; y: number }; onMap: (formula: string) => void;
  run: OperationHandler; close: () => void; rename?: boolean;
}) {
  const [error, setError] = useState<string | null>(null);
  const [fallback, setFallback] = useState("keep");
  const [custom, setCustom] = useState("");
  const mappings = document.objects.filter((object): object is FrameObject =>
    object.kind === "frame" && object.id !== frame.id && dictionaryColumns(object) !== null);
  const choose = (mapping: FrameObject) => {
    const { key, value } = dictionaryColumns(mapping)!;
    if (rename) {
      void run({ type: "renameColumnsUsingMapping", frameId: frame.id,
        mappingFrameId: mapping.id, keyColumnId: key.id, valueColumnId: value.id }, { inlineError: true })
        .then((failure) => { if (failure) setError(failure); else close(); });
    } else if (column) {
      close();
      const prefix = formulaToken(mapping.name);
      const suffix = fallback === "keep" ? "" : `, ${fallback === "none" || custom === "" ? "None" : JSON.stringify(custom)}`;
      onMap(`map_values(${formulaToken(column.name)}, ${prefix}.${formulaToken(key.name)}, ${prefix}.${formulaToken(value.name)}${suffix})`);
    }
  };
  return <ContextMenuGroup collapsed label={rename ? "Rename columns…" : "Map values…"} Icon={ArrowLeftRight}>
    <button onClick={() => {
      close();
      void run({ type: "addDictionary", name: rename ? `${frame.name} names` : `${column?.name ?? frame.name} map`, ...position });
    }}>Create mapping frame…</button>
    <small className="mapping-hint">{rename ? "Match current names to replacements. Unmatched names stay unchanged."
      : "Blank keys match missing values. Blank replacements produce missing values."}</small>
    {!rename && <label className="mapping-fallback">Otherwise <select aria-label="Unmatched values" value={fallback}
      onChange={(event) => setFallback(event.target.value)}>
      <option value="keep">Keep original</option><option value="none">Leave blank</option>
      <option value="custom">Custom text</option>
    </select></label>}
    {!rename && fallback === "custom" && <input className="mapping-replacement" aria-label="Unmatched replacement" placeholder="OTHER"
      value={custom} onChange={(event) => setCustom(event.target.value)} />}
    {error && <p className="mapping-hint" role="alert">{error}</p>}
    {mappings.map((mapping) => <button key={mapping.id} onClick={() => choose(mapping)}>Using {mapping.name}</button>)}
  </ContextMenuGroup>;
}
