import { sampleFrameStep } from "./api";

/**
 * Works out a reference-free expression to the raw text a cell stores.
 *
 * A seam rather than a direct call so the fold's decisions can be tested
 * with a handed answer: what the arithmetic *equals* is the engine's
 * question, never the interface's.
 */
export type EvaluateLiteralCell = (
  frameId: string,
  column: { id: string; name: string },
  expression: string
) => Promise<string>;

/**
 * The engine answers through the step sample, the one existing command that
 * runs a draft formula and hands back values: a one-step chain that
 * overwrites the target column in place with the expression, sampled for a
 * single row. The chain replaces the saved one for this read only, and the
 * expression reads nothing, so every row's answer is the same and the first
 * row's is the answer. Overwriting the target column itself, rather than
 * minting a scratch output, is what keeps this from ever colliding with a
 * column name; the value comes back in `scalar_value_to_raw`'s spelling,
 * which is the spelling a cell write takes.
 */
export const evaluateWithStepSample: EvaluateLiteralCell = async (
  frameId,
  column,
  expression
) => {
  let sample;
  try {
    sample = await sampleFrameStep(
      frameId,
      [
        {
          kind: "withColumns",
          columns: [{ outputColumnId: column.id, name: column.name, formula: expression }],
        },
      ],
      0,
      1
    );
  } catch (error) {
    throw new Error(error instanceof Error ? error.message : String(error), {
      cause: error,
    });
  }
  const position = sample.columns.findIndex((candidate) => candidate.id === column.id);
  const row = sample.rows[0];
  if (position < 0 || !row)
    throw new Error(`${column.name} has no row to work ${expression} out against.`);
  return row[position] ?? "";
};
