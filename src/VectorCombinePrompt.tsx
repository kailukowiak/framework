import { Columns3, Rows3 } from "lucide-react";
export type VectorCombineState = {
  frameId: string;
  viewId: string;
  vector: {
    formula: string;
    name: string;
    length: number;
  };
};

/**
 * A dropped vector can extend a one-column frame in two honest directions.
 * Asking here costs one compact click and avoids encoding position as intent:
 * beside and below are different models even when the pointer landed in the
 * same 26-pixel edge.
 */
export function VectorCombinePrompt({
  state,
  onChoose,
  onClose,
}: {
  state: VectorCombineState;
  onChoose: (mode: "hstack" | "vstack") => void;
  onClose: () => void;
}) {
  return (
    <aside
      className="vector-combine-prompt"
      role="dialog"
      aria-modal="false"
      aria-label={`Place ${state.vector.name}`}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <span className="eyebrow">PLACE {state.vector.name}</span>
      <button onClick={() => onChoose("hstack")}>
        <Columns3 size={15} />
        <span><strong>Beside · HStack</strong><small>Pair by row as another column</small></span>
      </button>
      <button onClick={() => onChoose("vstack")}>
        <Rows3 size={15} />
        <span><strong>Below · VStack</strong><small>Append these values as more rows</small></span>
      </button>
      <button className="vector-combine-cancel" onClick={onClose}>Cancel</button>
    </aside>
  );
}
