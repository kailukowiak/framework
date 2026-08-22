use crate::Id;
use crate::model::derivation::PivotAggregate;
use crate::model::frame::FrameStyleOutput;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a broadcast list does to each column it meets.
///
/// Arithmetic only, and deliberately not [`crate::formula::ast::BinaryOperator`]:
/// the gesture is "scale these" or "shift these", and a broadcast that
/// answered True or False would be a filter wearing the wrong clothes.
/// Anything past these four is a formula, and a formula has a column
/// editor already.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BroadcastOperator {
    Multiply,
    Divide,
    Add,
    Subtract,
}

impl BroadcastOperator {
    pub(crate) fn binary(self) -> crate::formula::ast::BinaryOperator {
        use crate::formula::ast::BinaryOperator;
        match self {
            BroadcastOperator::Multiply => BinaryOperator::Multiply,
            BroadcastOperator::Divide => BinaryOperator::Divide,
            BroadcastOperator::Add => BinaryOperator::Add,
            BroadcastOperator::Subtract => BinaryOperator::Subtract,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NamedFormulaInput {
    pub name: String,
    pub formula: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExistingFormulaInput {
    pub output_column_id: Id,
    pub name: String,
    pub formula: String,
}

/// A conditional-formatting rule as the inspector supplies it: the hidden
/// column still as text, and the reading of its answer already typed.
/// Existing ids preserve rule identity while editing; an omitted id is
/// minted during preparation so replicas receive the same ordered rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameStyleRuleInput {
    #[serde(default)]
    #[ts(optional = nullable)]
    pub id: Option<Id>,
    pub formula: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub column_id: Option<Id>,
    pub output: FrameStyleOutput,
}

/// A step as the editor supplies it: formulas still text, output columns
/// named rather than typed. Parsing and typing happen against the schema at
/// the step's own position, which is why the chain arrives whole rather than
/// a step at a time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum FrameStepInput {
    Filter {
        predicates: Vec<String>,
        #[serde(default = "default_true")]
        match_all: bool,
    },
    WithColumns {
        columns: Vec<ExistingFormulaInput>,
    },
    Select {
        column_ids: Vec<Id>,
    },
    Summarize {
        group_keys: Vec<ExistingFormulaInput>,
        aggregates: Vec<ExistingFormulaInput>,
        #[serde(default = "default_true")]
        maintain_order: bool,
    },
    Sort {
        keys: Vec<SortInput>,
    },
    // The editor names only the frame; which columns line up under which
    // is worked out from their names when the chain is saved, so the
    // person stacking two imports never has to write the mapping a
    // matching pair of headers already implies.
    Union {
        frame_id: Id,
    },
    // Pair every current row with every row of the named frame. Column
    // outputs are discovered and assigned stable ids when the chain saves.
    Expand {
        frame_id: Id,
    },
    // No outputs here: they are discovered from the data at save time.
    // The editor cannot know what values a column holds, and asking it to
    // would put the data-dependent part of a pivot in the one place that
    // never sees data.
    Pivot {
        names_column_id: Id,
        values_column_id: Id,
        aggregate: PivotAggregate,
    },
    // The melt list arrives as written text — `` `Jan`, `Feb`,
    // starts_with("Q") `` — and is resolved to concrete columns against
    // the schema at the step's position when the chain is saved, the same
    // moment a formula's names become ids. See `formula::column_list` for
    // the notation and for why a selector bakes rather than staying live.
    Unpivot {
        columns: String,
        name_column_id: Id,
        name_column_name: String,
        value_column_id: Id,
        value_column_name: String,
    },
    // A list spread across a set of columns: the first value meets the
    // first column, the second the second. This is the drag-across gesture
    // — the one Excel spells `=B2*B$1` and then repeats in every cell —
    // said once, with the direction stated instead of implied by which
    // half of the address carries the `$`.
    //
    // Written with the melt list's notation, for the same reason: the
    // frames this exists for are the wide ones, and forty checkboxes is
    // not a way to say "the quarter columns". The list is resolved against
    // the schema at this step's position when the chain saves.
    //
    // It leaves no step kind of its own. Saving expands it into an
    // ordinary `WithColumns` — one readable formula per column, each
    // naming the list and the position it took — which is the same bargain
    // pivot and union make: the shape is settled when the step is written,
    // and what is settled is legible afterwards. The *values* stay live,
    // because each formula still names the list rather than the number it
    // held at save time.
    Broadcast {
        columns: String,
        vector: String,
        operator: BroadcastOperator,
    },
    // Markdown, taken exactly as typed. The one step input with nothing to
    // resolve: no formula to parse, no column to look up, nothing that can
    // fail at save.
    Comment {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SortInput {
    pub column_id: Id,
    pub descending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JoinColumnInput {
    pub source_frame_id: Id,
    pub source_column_id: Id,
    pub name: String,
}

fn default_true() -> bool {
    true
}
