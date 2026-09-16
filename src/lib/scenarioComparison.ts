import { formattedFormula } from "../PipelineFormulaFormatting";
import { nextObjectName } from "../hooks/useCanvasObjectCreation";
import { drawnCanvasCards, viewHolding } from "./canvasCards";
import { frameCardSize, placeNewCard } from "./cardPlacement";
import { formulaToken } from "./formulaReferences";
import { mintColumnId } from "./pipelineSteps";
import type {
  BlockLine,
  BlockObject,
  DocumentView,
  FrameObject,
  FrameStepInput,
  Operation,
} from "./types";

/**
 * "Compare scenarios": a block's answers across, the scenarios down — the
 * table a finance person draws by hand on the back of the model.
 *
 * Until `under(...)` existed this table could not be written down at all.
 * The document holds one active scenario, so seeing Upside next to Downside
 * meant switching the whole document twice and copying numbers out, which is
 * the spreadsheet habit this application exists to replace. With `under` the
 * comparison is an ordinary frame: one text column naming the outputs, and
 * one calculated column per scenario reading the same block through that
 * scenario's assumptions. Nothing here is a new object kind, a new panel, or
 * a live link — it is a frame somebody could have typed, which means every
 * gesture a frame already has (sort, format, plot, chain further) applies.
 */

/** The column naming which of the block's lines each row is about. */
export const OUTPUT_COLUMN_NAME = "Output";

/** The column reading the block as the document itself reads it. */
export const BASE_COLUMN_NAME = "Base";

/**
 * The lines this table can have a row for.
 *
 * Every block line carries a name so that it is addressable at all, but only
 * a *typed* name is one the author chose. A row labelled `line_4` would be a
 * name put in their mouth, and a column of them would be a table nobody can
 * read, so the untyped ones are left out — the same rule the block's own
 * gutter uses when it decides which names to show.
 */
export function comparableBlockLines(block: BlockObject): BlockLine[] {
  return block.lines.filter((line) => line.named && line.name.trim() !== "");
}

/** Whether the gesture has anything to build: scenarios, and lines to read. */
export function canCompareScenarios(
  document: DocumentView,
  block: BlockObject
): boolean {
  return (
    document.scenarios.length > 0 && comparableBlockLines(block).length > 0
  );
}

/** How this comparison frame is named, from the block it compares. */
export function comparisonFrameName(blockName: string): string {
  return `${blockName} by scenario`;
}

/** `` `Block`.`line` `` — the qualified spelling the engine renders back. */
function lineReference(blockName: string, lineName: string): string {
  return `${formulaToken(blockName)}.${formulaToken(lineName)}`;
}

/**
 * One column's formula: the block's lines selected by the `Output` label,
 * each read plainly for Base and wrapped in `under(...)` for a scenario.
 *
 * A `when`/`then` ladder rather than a lookup because the outputs are the
 * block's own expressions, not data — there is nothing to join against. The
 * last line is the `otherwise`, so a one-line block is a bare read with no
 * ladder at all, which is both correct and what somebody would have typed.
 *
 * The spelling is the engine's canonical one (backticked names, `==`,
 * quoted literals) and the text is run through the chain formatter, because
 * the Wrangle editor reconciles saves by comparing this text against the
 * text the engine echoes. A formula spelled any other way would read as an
 * outside edit the moment it was saved.
 */
export function scenarioColumnFormula(
  blockName: string,
  lineNames: string[],
  scenarioName: string | null
): string {
  const read = (lineName: string) => {
    const reference = lineReference(blockName, lineName);
    return scenarioName === null
      ? reference
      : `under(${formulaToken(scenarioName)}, ${reference})`;
  };
  const last = lineNames[lineNames.length - 1];
  const branches = lineNames
    .slice(0, -1)
    .map(
      (lineName) =>
        `when(${formulaToken(OUTPUT_COLUMN_NAME)} == ${JSON.stringify(
          lineName
        )}).then(${read(lineName)})`
    );
  const source =
    branches.length === 0
      ? read(last)
      : `${branches.join(".")}.otherwise(${read(last)})`;
  return formattedFormula(source);
}

/** The `Output` column and its one row per named line. */
export function comparisonGrid(lineNames: string[]): string[][] {
  return [[OUTPUT_COLUMN_NAME], ...lineNames.map((name) => [name])];
}

/** Base first, then every scenario in document order. */
export function comparisonColumns(
  blockName: string,
  lineNames: string[],
  scenarioNames: string[]
): FrameStepInput {
  const names = [BASE_COLUMN_NAME, ...scenarioNames];
  return {
    kind: "withColumns",
    columns: names.map((name, index) => ({
      outputColumnId: mintColumnId(name),
      name,
      formula: scenarioColumnFormula(
        blockName,
        lineNames,
        index === 0 ? null : name
      ),
    })),
  };
}

/**
 * Build the comparison beside the block, as two operations.
 *
 * Two, because the frame has to exist before its chain can name it, and
 * `addFrame` answers with a document rather than an id. That makes this two
 * undo steps today, which is accepted: undo twice and the canvas is back
 * where it was, with nothing half-built in between. Should it become one,
 * the change is entirely inside this function — everything else asks only
 * for "compare this block".
 */
export async function compareScenariosFromBlock(
  initial: DocumentView,
  block: BlockObject,
  apply: (operation: Operation) => Promise<DocumentView>
): Promise<DocumentView> {
  const lineNames = comparableBlockLines(block).map((line) => line.name);
  if (lineNames.length === 0)
    throw new Error(`${block.name} has no named lines to compare`);
  const scenarioNames = initial.scenarios.map((scenario) => scenario.name);
  if (scenarioNames.length === 0)
    throw new Error("This document has no scenarios to compare");

  const name = nextObjectName(initial.objects, comparisonFrameName(block.name));
  const card = viewHolding(initial, block.id);
  const size = frameCardSize(1 + scenarioNames.length, lineNames.length);
  // Beside the block it reads, not on top of whatever is already there:
  // the same "anchor, then step aside" rule every other spawn path uses.
  const position = placeNewCard(
    drawnCanvasCards(initial),
    card
      ? { x: card.x + card.width + 64, y: card.y }
      : { x: 0, y: 0 },
    size
  );

  const before = new Set(initial.objects.map((object) => object.id));
  const next = await apply({
    type: "addFrame",
    name,
    grid: comparisonGrid(lineNames),
    x: position.x,
    y: position.y,
  });
  const created = next.objects.find(
    (object): object is FrameObject =>
      object.kind === "frame" && !before.has(object.id)
  );
  if (!created) throw new Error("The comparison table was not created");

  return apply({
    type: "setFramePipeline",
    frameId: created.id,
    steps: [comparisonColumns(block.name, lineNames, scenarioNames)],
  });
}
