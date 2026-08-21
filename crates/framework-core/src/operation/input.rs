use crate::Id;
use crate::model::derivation::PivotAggregate;
use crate::model::frame::FrameStyleOutput;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
