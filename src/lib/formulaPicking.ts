import type { ActiveFormulaEditor } from "./activeFormulaEditor";
import {
  columnReferenceForPick,
  columnTokenForCellPick,
  scalarTokenForCellPick,
} from "./formulaReferences";
import type { SummaryOperation } from "./types";

export type FormulaColumnPick =
  | { kind: "insert"; token: string }
  | { kind: "recurrence" }
  | { kind: "refuse"; message: string | null };

type FormulaSummaryPick = Exclude<FormulaColumnPick, { kind: "recurrence" }>;

const SUMMARY_FORMULA_SUFFIX: Record<SummaryOperation, string> = {
  sum: ".sum()",
  mean: ".mean()",
  quartile25: ".quantile(0.25)",
  median: ".median()",
  quartile75: ".quantile(0.75)",
  min: ".min()",
  max: ".max()",
  count: ".count()",
  missing: ".null_count()",
  countDistinct: ".drop_nulls().n_unique()",
  mode: ".drop_nulls().mode(True).first()",
};

/** The ordinary formula expression represented by a visible profile cell. */
export function summaryFormulaToken(
  operation: SummaryOperation,
  columnToken: string
): string {
  return `${columnToken}${SUMMARY_FORMULA_SUFFIX[operation]}`;
}

/** A profile click is aggregate syntax over the editor's own column token. */
export function formulaSummaryPick(
  active: ActiveFormulaEditor,
  operation: SummaryOperation,
  columnId: string
): FormulaSummaryPick {
  const reference = columnReferenceForPick(active.completion.references, columnId);
  return reference
    ? { kind: "insert", token: summaryFormulaToken(operation, reference.token) }
    : {
        kind: "refuse",
        message: "That summary would make this formula read its own result.",
      };
}

/** Translate one source-cell gesture without giving coordinates persistence. */
export function formulaColumnPick(
  active: ActiveFormulaEditor,
  columnId: string,
  frameId: string,
  rowIndex: number | undefined,
  stableCellAddress: boolean
): FormulaColumnPick {
  if (active.completion.targetColumnId === columnId) {
    if (
      active.completion.anchorRowIndex !== undefined &&
      rowIndex !== undefined &&
      rowIndex < active.completion.anchorRowIndex
    ) {
      return active.completion.previousResultToken
        ? { kind: "insert", token: active.completion.previousResultToken }
        : { kind: "recurrence" };
    }
    return {
      kind: "refuse",
      message:
        rowIndex !== undefined &&
        active.completion.anchorRowIndex !== undefined &&
        rowIndex > active.completion.anchorRowIndex
          ? "A calculate-down formula can read an earlier row, not a later one."
          : "A calculated column cannot read itself on the same row.",
    };
  }
  const reference = columnReferenceForPick(
    active.completion.references,
    columnId
  );
  if (!reference) return { kind: "refuse", message: null };
  if (active.kind === "scratchwork" && rowIndex !== undefined) {
    if (!stableCellAddress) {
      return {
        kind: "refuse",
        message:
          "Specific cells can only be referenced from an internal dataset with a stable row address. Click the column header or a summary statistic instead.",
      };
    }
    return {
      kind: "insert",
      token: scalarTokenForCellPick(reference.token, rowIndex),
    };
  }
  const anchored = active.completion.anchorFrameId === frameId;
  return {
    kind: "insert",
    token: columnTokenForCellPick(
      reference.token,
      anchored ? active.completion.anchorRowIndex : undefined,
      anchored && Number.isFinite(rowIndex) ? rowIndex : undefined
    ),
  };
}

/** Keep a drag from quietly degrading to whichever cell received the press. */
export function formulaCellRangePick(
  active: ActiveFormulaEditor,
  columnId: string,
  frameId: string,
  firstRowIndex: number,
  lastRowIndex: number,
  stableCellAddress: boolean
): FormulaSummaryPick {
  if (firstRowIndex === lastRowIndex) {
    const picked = formulaColumnPick(
      active,
      columnId,
      frameId,
      firstRowIndex,
      stableCellAddress
    );
    return picked.kind === "recurrence"
      ? { kind: "refuse", message: "A Scratchwork slice cannot seed recurrence." }
      : picked;
  }
  return {
    kind: "refuse",
    message:
      "A row slice would change meaning after a sort or refresh. Click the column header for the live whole column, or filter it in Wrangle.",
  };
}
