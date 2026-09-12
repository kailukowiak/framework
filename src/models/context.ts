import { createContext, useContext } from "react";
import type { DocumentView } from "../lib/types";
import type { OperationHandler } from "../lib/handlers";

export type ModelDialogState = {
  kind: "create" | "edit" | "predict" | "statistics";
  modelId?: string;
  sourceFrameId?: string;
  x: number;
  y: number;
};

export const ModelContext = createContext<{
  document: DocumentView;
  onOperation: OperationHandler;
  open: (state: ModelDialogState) => void;
  position: () => { x: number; y: number };
} | null>(null);

export function useModels() {
  const context = useContext(ModelContext);
  if (!context) throw new Error("Model controls require a workbook");
  return context;
}
