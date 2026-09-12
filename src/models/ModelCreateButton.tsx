import { useContext } from "react";
import { ChartNoAxesCombined } from "lucide-react";
import { ModelContext } from "./context";

export function ModelCreateButton({ sourceFrameId, x, y, rail = false, onChoose, statistics = false }: {
  sourceFrameId?: string; x?: number; y?: number; rail?: boolean; onChoose?: () => void; statistics?: boolean;
}) {
  const context = useContext(ModelContext);
  if (!context) return null;
  return <button className={rail ? "rail-button" : undefined}
    title={statistics ? "Compare means or measure association" : "Fit a regression or import a trained model"}
    onClick={() => {
      onChoose?.();
      const position = x !== undefined && y !== undefined ? { x, y } : context.position();
      context.open({ kind: statistics ? "statistics" : "create", sourceFrameId, ...position });
    }}>
    <ChartNoAxesCombined size={rail ? 19 : 14} /><span>{statistics ? rail ? "Stats" : "Statistics…" : rail ? "Model" : "Create model…"}</span>
  </button>;
}
