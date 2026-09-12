import { useState, type ReactNode } from "react";
import type { DocumentView } from "../lib/types";
import type { OperationHandler } from "../lib/handlers";
import { ModelContext, type ModelDialogState } from "./context";
import { ModelDialog } from "./ModelDialog";
import { ModelPredictionDialog } from "./ModelPredictionDialog";
import { StatisticalDialog } from "./StatisticalDialog";
import "./models.css";

/** One authoring session shared by the rail, context menu and fitted cards. */
export function ModelWorkbench({ document, onOperation, children, position = () => ({ x: 80, y: 120 }) }: {
  document: DocumentView;
  onOperation: OperationHandler;
  children: ReactNode;
  position?: () => { x: number; y: number };
}) {
  const [dialog, setDialog] = useState<ModelDialogState | null>(null);
  return <ModelContext.Provider value={{ document, onOperation, open: setDialog, position }}>
    {children}
    {dialog && (dialog.kind === "statistics"
      ? <StatisticalDialog state={dialog} onClose={() => setDialog(null)} />
      : dialog.kind === "predict"
      ? <ModelPredictionDialog state={dialog} onClose={() => setDialog(null)} />
      : <ModelDialog state={dialog} onClose={() => setDialog(null)} />)}
  </ModelContext.Provider>;
}
