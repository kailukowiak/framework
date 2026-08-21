/**
 * The document model as TypeScript sees it.
 *
 * Most of this file is now a re-export. The types below live in
 * `src/lib/bindings/`, generated from the Rust structs by `ts-rs`, and are
 * regenerated with one command:
 *
 *     cargo test -p framework-core export_bindings
 *
 * There is nothing to remember beyond that: the output directory is set in
 * `.cargo/config.toml`, so the command writes into `src/lib/bindings/`
 * wherever it is run from. The generated files are committed, so a checkout
 * type-checks without a Rust toolchain and a drifting mirror shows up as a
 * diff in review rather than as a bug at runtime.
 *
 * This file exists on top of the generated files for three reasons, and each
 * one is marked where it applies below:
 *
 *   1. **The tag.** Rust models the canvas objects as one internally-tagged
 *      enum, so `kind` belongs to `DataObject` and the generated
 *      `ValueObject` is the struct's fields alone. The interface has always
 *      treated `kind` as part of each object type — it narrows a `DataObject`
 *      by `.kind` and hands the result to something typed `FrameObject` — so
 *      the tag is put back here, once, rather than at every call site.
 *
 *   2. **The names.** A few types are called something else on this side,
 *      because the name that reads best next to a Rust parser is not always
 *      the one that reads best next to a React component.
 *
 *   3. **The frontend's own types.** `Selection` and `TabObject` describe
 *      what the canvas is doing, not what the core sent. They have no Rust
 *      counterpart and are written out below.
 *
 * What is deliberately *not* generated: a formula's parsed expression tree.
 * It crosses the wire inside `Formula`, `FrameStep` and `FrameDerivation` as
 * `unknown`, because the interface reads and writes formulas as text and has
 * never looked inside one. The reasoning is written out at `Formula` in
 * `crates/framework-core/src/formula/ast.rs`.
 */

// ---------------------------------------------------------------------------
// Generated, re-exported under the name the core gives them.
// ---------------------------------------------------------------------------

export type { CanvasView } from "./bindings/CanvasView";
export type { Cell } from "./bindings/Cell";
export type { CellUpdate } from "./bindings/CellUpdate";
export type { Column } from "./bindings/Column";
export type { ColumnFormat } from "./bindings/ColumnFormat";
export type { ColumnFormatScale } from "./bindings/ColumnFormatScale";
export type { ColumnFormatStyle } from "./bindings/ColumnFormatStyle";
export type { CompletionResult } from "./bindings/CompletionResult";
export type { ComputedBlock } from "./bindings/ComputedBlock";
export type { ComputedBlockLine } from "./bindings/ComputedBlockLine";
export type { ComputedCell } from "./bindings/ComputedCell";
export type { ComputedMaterialization } from "./bindings/ComputedMaterialization";
export type { ComputedResult } from "./bindings/ComputedResult";
export type { ComputedText } from "./bindings/ComputedText";
export type { ComputedTextSegment } from "./bindings/ComputedTextSegment";
export type { ComputedFrame } from "./bindings/ComputedFrame";
export type { DataObject } from "./bindings/DataObject";
export type { DataType } from "./bindings/DataType";
export type { DocumentView } from "./bindings/DocumentView";
export type { FormulaArgument } from "./bindings/FormulaArgument";
export type { FormulaFunction } from "./bindings/FormulaFunction";
export type { Formula } from "./bindings/Formula";
export type { FrozenState } from "./bindings/FrozenState";
export type { Operation } from "./bindings/Operation";
export type { PivotAggregate } from "./bindings/PivotAggregate";
export type { RenderedFrameStep } from "./bindings/RenderedFrameStep";
export type { Row } from "./bindings/Row";
export type { ScalarValue } from "./bindings/ScalarValue";
export type { Summary } from "./bindings/Summary";
export type { SuggestionKind } from "./bindings/SuggestionKind";
export type { FrameCellStyle } from "./bindings/FrameCellStyle";
export type { FrameDerivation } from "./bindings/FrameDerivation";
export type { FrameDisplay } from "./bindings/FrameDisplay";
export type { CrosstabDisplay } from "./bindings/CrosstabDisplay";
export type { EntryColumn } from "./bindings/EntryColumn";
export type { FrameEditing } from "./bindings/FrameEditing";
export type { FrameJoin } from "./bindings/FrameJoin";
export type { FrameJoinType } from "./bindings/FrameJoinType";
export type { FrameStepInput } from "./bindings/FrameStepInput";
export type { FrameStyle } from "./bindings/FrameStyle";
export type { FrameStyleCase } from "./bindings/FrameStyleCase";
export type { FrameStyleMatch } from "./bindings/FrameStyleMatch";
export type { FrameStyleOutput } from "./bindings/FrameStyleOutput";
export type { FrameStyleRule } from "./bindings/FrameStyleRule";
export type { FrameStyleRuleInput } from "./bindings/FrameStyleRuleInput";
export type { FrameStyleScale } from "./bindings/FrameStyleScale";
export type { FrameStyleColorScale } from "./bindings/FrameStyleColorScale";
export type { FrameStyleScaleProperty } from "./bindings/FrameStyleScaleProperty";
export type { FrameStyleTarget } from "./bindings/FrameStyleTarget";
export type { UniqueKeyConstraint } from "./bindings/UniqueKeyConstraint";

// The rest of the model, exported so that nothing in the mirror is reachable
// only by reaching into `bindings/` directly. These are the pieces the types
// above are built out of.
export type { ArtifactFormat } from "./bindings/ArtifactFormat";
export type { ConnectorRecipe } from "./bindings/ConnectorRecipe";
export type { DataArtifact } from "./bindings/DataArtifact";
export type { DerivedExpression } from "./bindings/DerivedExpression";
export type { DerivedSort } from "./bindings/DerivedSort";
export type { Document } from "./bindings/Document";
export type { FrozenValue } from "./bindings/FrozenValue";
export type { JoinColumnInput } from "./bindings/JoinColumnInput";
export type { JoinOutput } from "./bindings/JoinOutput";
export type { Materialization } from "./bindings/Materialization";
export type { NamedFormulaInput } from "./bindings/NamedFormulaInput";
export type { PivotOutput } from "./bindings/PivotOutput";
export type { RenderedDerivedExpression } from "./bindings/RenderedDerivedExpression";
export type { RenderedFrameDerivation } from "./bindings/RenderedFrameDerivation";
export type { SortInput } from "./bindings/SortInput";
export type { SummaryOperation } from "./bindings/SummaryOperation";
export type { FrameCellAlignment } from "./bindings/FrameCellAlignment";
export type { FrameLineStyle } from "./bindings/FrameLineStyle";
export type { FrameStep } from "./bindings/FrameStep";
export type { UnionColumn } from "./bindings/UnionColumn";
export type { UnpivotColumn } from "./bindings/UnpivotColumn";

// ---------------------------------------------------------------------------
// Generated, renamed. (Reason 2 in the header.)
// ---------------------------------------------------------------------------

/** One completion offered for a half-typed formula. */
export type { Suggestion as CompletionSuggestion } from "./bindings/Suggestion";

/**
 * A formula written against a column that already exists, which is what the
 * editor sends for every step that names its outputs.
 */
export type { ExistingFormulaInput as NamedFormula } from "./bindings/ExistingFormulaInput";

/** Whether a frame is drawn a record per row or a field per row. */
export type {
  FrameViewOrientation as FrameOrientation,
  FrameViewOrientation,
} from "./bindings/FrameViewOrientation";

/** One column of a sort, and which way it runs. */
export type { DerivedSort as SortKey } from "./bindings/DerivedSort";

// ---------------------------------------------------------------------------
// Generated, with the enum's tag put back. (Reason 1 in the header.)
// ---------------------------------------------------------------------------

import type { BlockObject as BlockObjectFields } from "./bindings/BlockObject";
import type { ContainerObject as ContainerObjectFields } from "./bindings/ContainerObject";
import type { PlotObject as PlotObjectFields } from "./bindings/PlotObject";
import type { ResultObject as ResultObjectFields } from "./bindings/ResultObject";
import type { SeriesObject as SeriesObjectFields } from "./bindings/SeriesObject";
import type { FrameObject as FrameObjectFields } from "./bindings/FrameObject";
import type { TextObject as TextObjectFields } from "./bindings/TextObject";
import type { ValueObject as ValueObjectFields } from "./bindings/ValueObject";

/** A number, date or name somebody typed. */
export type ValueObject = { kind: "value" } & ValueObjectFields;

/**
 * A computed value: the formula is the object, the answer is worked out
 * live. `formula` here is the persisted expression tree — read
 * `ComputedResult.formula` for the text to show or edit.
 */
export type ResultObject = { kind: "result" } & ResultObjectFields;

/**
 * An ordered scratchpad of expression lines. Lines read each other bare and
 * only upward; everything else reaches a line as `` `Block`.`line` ``.
 */
export type BlockObject = { kind: "block" } & BlockObjectFields;

/** A named list on the canvas. Values are raw text, as typed or pasted. */
export type SeriesObject = { kind: "series" } & SeriesObjectFields;

/** A heading and the things kept under it. Members are drawn on its card. */
export type ContainerObject = { kind: "container" } & ContainerObjectFields;

/** A grid of values, and the chain that produced them. */
export type FrameObject = { kind: "frame" } & FrameObjectFields;

export type TextObject = { kind: "text" } & TextObjectFields;

export type PlotObject = { kind: "plot" } & PlotObjectFields;

/**
 * One line of a formula block. `formula` is the persisted expression tree,
 * absent on a line that does not parse — read `ComputedBlock.source` for the
 * text to edit and `ComputedBlockLine` for what to show beside it.
 *
 * Not tagged: a line is a plain struct in Rust too, and is re-exported here
 * only to sit beside the block it belongs to.
 */
export type { BlockLine } from "./bindings/BlockLine";

// ---------------------------------------------------------------------------
// The frontend's own. (Reason 3 in the header.)
// ---------------------------------------------------------------------------

export type Id = string;

/**
 * A Vega-Lite specification. Named rather than spelled out because the shape
 * belongs to Vega, not to this document model: it arrives from the core as an
 * opaque bag of keys and goes straight to `vega-embed`.
 */
export type VegaLiteSpec = Record<string, unknown>;

/**
 * What a card can offer as a tab. A plot of a frame reads the same data
 * through the same lineage, so the two belong on the same card — which is why
 * this is a pair rather than just a frame.
 */
export type TabObject = FrameObject | PlotObject;

/**
 * What is selected on the canvas right now. Purely an interface concern: the
 * core is never told, and nothing about it is persisted.
 */
export interface Selection {
  objectId: Id;
  viewId?: Id;
  columnId?: Id;
  rowId?: Id;
}
