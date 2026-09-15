import { KeyRound } from "lucide-react";
import { Field } from "./Field";
import { formulaToken } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type { Column, ComputedFrame, DataType, FrameObject } from "./lib/types";

/** Decimal places an accounting column may declare. The engine backs an
 * amount with a 38-digit decimal, so 38 is the arithmetic ceiling and not a
 * number anyone should have to know to use the control. */
const MAX_ACCOUNTING_SCALE = 38;
const DEFAULT_ACCOUNTING_SCALE = 2;

/**
 * The type of the selected column, and the few settings a type implies.
 *
 * A literal frame sets its type directly; a read or derived frame has no
 * literal column to retype, so the same choice is authored as the cast it
 * really is and lands in the Wrangle chain. Accounting is the one type that
 * carries a number with it — the places its exact values hold — so that
 * number sits next to the type rather than in Format, which is presentation
 * and would be a second gesture for something the type already decided.
 */
export function ColumnTypeFields({
  frame,
  column,
  computed,
  onOperation,
  onTransformColumn,
}: {
  frame: FrameObject;
  column: Column;
  computed: ComputedFrame;
  onOperation: OperationHandler;
  onTransformColumn: (column: Column, formula: string, focus?: boolean) => void;
}) {
  const literal = Boolean(computed.editing.rows);
  return (
    <>
      <label className="inspector-field">
        Column type
        <select
          value={column.dataType}
          onChange={(event) => {
            const dataType = event.target.value as DataType;
            if (literal) {
              void onOperation({
                type: "setColumnType",
                frameId: frame.id,
                columnId: column.id,
                dataType,
              });
            } else {
              onTransformColumn(
                column,
                `${formulaToken(column.name)}.cast("${dataType}")`
              );
            }
          }}
        >
          <option value="string">Text</option>
          <option value="categorical" disabled={!literal}>
            Categorical
          </option>
          <option value="integer">Integer</option>
          <option value="number">Number</option>
          <option value="currency" disabled={!literal}>
            Currency
          </option>
          <option value="accounting">Accounting</option>
          <option value="percentage" disabled={!literal}>
            Percentage
          </option>
          <option value="boolean">Boolean</option>
          <option value="date">Date</option>
        </select>
      </label>
      {literal && (
        <>
          {column.dataType === "accounting" && (
            <label className="inspector-field">
              Decimal places
              <input
                type="number"
                min={0}
                max={MAX_ACCOUNTING_SCALE}
                value={column.scale ?? DEFAULT_ACCOUNTING_SCALE}
                onChange={(event) => {
                  const scale = Math.min(
                    Math.max(Math.trunc(Number(event.target.value)), 0),
                    MAX_ACCOUNTING_SCALE
                  );
                  if (!Number.isFinite(scale)) return;
                  void onOperation({
                    type: "setColumnType",
                    frameId: frame.id,
                    columnId: column.id,
                    dataType: "accounting",
                    scale,
                  });
                }}
              />
            </label>
          )}
          {column.dataType === "categorical" && (
            <Field
              label="Allowed values"
              help="In order — this is how the column sorts and compares."
              initial={(column.categories ?? []).join(", ")}
              onCommit={(raw) =>
                onOperation({
                  type: "setColumnCategories",
                  frameId: frame.id,
                  columnId: column.id,
                  categories: raw
                    .split(",")
                    .map((category) => category.trim())
                    .filter(Boolean),
                })
              }
            />
          )}
          <button
            className="secondary-action key-action"
            onClick={() =>
              onOperation({
                type: "setUniqueKey",
                frameId: frame.id,
                columnIds: [column.id],
                enabled: !frame.uniqueKeys.some(
                  (key) =>
                    key.columnIds.length === 1 && key.columnIds[0] === column.id
                ),
              })
            }
          >
            <KeyRound size={14} />{" "}
            {frame.uniqueKeys.some(
              (key) => key.columnIds.length === 1 && key.columnIds[0] === column.id
            )
              ? "Remove unique key"
              : "Mark as unique key"}
          </button>
        </>
      )}
    </>
  );
}
