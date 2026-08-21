use crate::Id;
use crate::formula::ast::Expr;
use crate::model::data_artifact::DataArtifact;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Materialization {
    pub artifact: DataArtifact,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum FrameStep {
    Filter {
        #[ts(type = "Array<unknown>")]
        predicates: Vec<Expr>,
        #[serde(default = "default_true")]
        match_all: bool,
    },
    WithColumns {
        columns: Vec<DerivedExpression>,
    },
    Select {
        column_ids: Vec<Id>,
    },
    Summarize {
        group_keys: Vec<DerivedExpression>,
        aggregates: Vec<DerivedExpression>,
        #[serde(default = "default_true")]
        maintain_order: bool,
    },
    Join {
        join: FrameJoin,
    },
    Sort {
        keys: Vec<DerivedSort>,
    },
    /// Another frame's rows stacked under this one's, lined up by the
    /// mapping. The mapping is resolved from column *names* when the step
    /// is written and kept as ids from then on, so renaming a column
    /// afterwards does not silently re-route which values stack where —
    /// the step keeps meaning what it meant until it is edited again.
    Union {
        frame_id: Id,
        mapping: Vec<UnionColumn>,
    },
    /// Every row here paired with every row of another frame. Unlike a
    /// join, expansion has no key and deliberately multiplies rows: it is
    /// the table-shaped `for each` used for calendars, scenarios, and other
    /// small generator lists. Outputs are baked so later formulas keep their
    /// addresses when the step is refreshed.
    Expand {
        frame_id: Id,
        outputs: Vec<ExpandOutput>,
    },
    /// One column's values turned into columns. The outputs are discovered
    /// from the data when the step is written and baked in, the way a join
    /// bakes its outputs: a pivot's schema is not a property of the plan,
    /// and a frame whose columns change under it whenever the data does is
    /// not a frame anyone can build on. New values appearing upstream wait
    /// until the step is saved again.
    Pivot {
        names_column_id: Id,
        values_column_id: Id,
        aggregate: PivotAggregate,
        outputs: Vec<PivotOutput>,
    },
    /// Chosen columns melted into name/value rows. Each label is the
    /// column's display name captured when the step was written — the plan
    /// knows columns only by id, and the whole point of the name column is
    /// to hold the word a person had at the top of the column.
    Unpivot {
        columns: Vec<UnpivotColumn>,
        name_column_id: Id,
        value_column_id: Id,
    },
    /// A remark standing in the chain, saying nothing to the engine.
    ///
    /// A chain of six steps is a program, and programs earn explanations
    /// where they get strange. The comment is a *step* rather than an
    /// annotation hung off one, so it survives everything a step survives —
    /// reordering, undo, projection into text — and owns a position: it
    /// speaks about the steps below it, the way a comment line in a formula
    /// block speaks about the lines below.
    ///
    /// `text` is markdown, never parsed as formula, and cannot error. The
    /// engine compiles it to nothing; every schema walk treats it as if it
    /// were not there.
    Comment {
        text: String,
    },
}

impl FrameStep {
    /// The other frame this step reads, when it reads one at all.
    ///
    /// A join's lookup and a union's stacked frame are lineage edges
    /// exactly like a derivation's source, and every walk that follows
    /// lineage — staleness, liveness, refresh order, layout depth, delete
    /// guards — asks here, so a future two-input step cannot be added
    /// without answering for all of them at once.
    pub fn lookup_frame_id(&self) -> Option<&Id> {
        match self {
            FrameStep::Join { join } => Some(&join.lookup_frame_id),
            FrameStep::Union { frame_id, .. } | FrameStep::Expand { frame_id, .. } => {
                Some(frame_id)
            }
            _ => None,
        }
    }
}

/// One output column of a union: where the stacked frame's rows get their
/// value for `column_id`. `None` means the stacked frame had no column
/// with a matching name when the step was written, so its rows hold
/// nothing there.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UnionColumn {
    pub column_id: Id,
    pub source_column_id: Option<Id>,
}

/// One column brought in by an expansion. The source id says what the
/// generator frame contributes; the output id is this frame's stable name
/// for it and prevents collisions with its own columns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExpandOutput {
    pub output_column_id: Id,
    pub source_column_id: Id,
}

/// How a pivot settles several rows landing in one cell. There is always
/// a policy, even over clean data: a pivot's cell is defined by two
/// coordinates that nothing guarantees are unique. `None` is the policy
/// of refusing — the cell holds the one row it gets, and a second row
/// landing there is an error rather than something quietly combined,
/// which is the behaviour to pick when the data is supposed to already
/// be one row per cell and being told otherwise is the point.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PivotAggregate {
    Sum,
    Count,
    Mean,
    Min,
    Max,
    First,
    None,
}

/// One column a pivot produces: the value of the names column it stands
/// for, and the id the rest of the document knows it by. Keeping the id
/// here is what lets a re-saved pivot hand the same value the same column
/// identity, so formats and references on it survive the edit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PivotOutput {
    pub output_column_id: Id,
    pub value: String,
}

/// One column an unpivot melts, with the display name its rows carry into
/// the name column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UnpivotColumn {
    pub column_id: Id,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum RenderedFrameStep {
    Filter {
        predicates: Vec<String>,
        match_all: bool,
    },
    WithColumns {
        columns: Vec<RenderedDerivedExpression>,
    },
    Select {
        column_ids: Vec<Id>,
    },
    Summarize {
        group_keys: Vec<RenderedDerivedExpression>,
        aggregates: Vec<RenderedDerivedExpression>,
        maintain_order: bool,
    },
    Join {
        join: FrameJoin,
    },
    Sort {
        keys: Vec<DerivedSort>,
    },
    Union {
        frame_id: Id,
        mapping: Vec<UnionColumn>,
    },
    Expand {
        frame_id: Id,
        outputs: Vec<ExpandOutput>,
    },
    Pivot {
        names_column_id: Id,
        values_column_id: Id,
        aggregate: PivotAggregate,
        outputs: Vec<PivotOutput>,
    },
    // The two output columns' names travel with the rendered step because
    // the editor shows them in text fields, and by the time it asks, later
    // steps may have dropped the columns that would otherwise carry them.
    Unpivot {
        columns: Vec<UnpivotColumn>,
        name_column_id: Id,
        name_column_name: String,
        value_column_id: Id,
        value_column_name: String,
    },
    Comment {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderedDerivedExpression {
    pub output_column_id: Id,
    pub formula: String,
}

/// What a derived frame reads, and what it does to it.
///
/// A derivation is one of two things, and they are alternatives rather than
/// stages. A **join** is held flat, in `join`, with `steps` empty: it is a
/// single two-input operation whose output columns are minted when it is
/// written, and the join editor reads and writes that field directly.
/// Everything else is a **pipeline**: an ordered `steps` chain, with `join`
/// empty. Nothing creates both, and `steps()` hands back one list either
/// way, so almost nothing downstream has to ask which shape it is holding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase", from = "StoredFrameDerivation")]
#[ts(export)]
pub struct FrameDerivation {
    pub source_frame_id: Id,
    #[serde(default)]
    pub join: Option<FrameJoin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<FrameStep>,
}

/// A derivation as it may appear on disk, including the flat field layout
/// that documents were written in before a chain became a list of steps.
///
/// This type exists only to be converted. `FrameDerivation` deserializes
/// through it, so an old document arrives already in the modern shape and
/// nothing past this boundary has to know the old one existed. The `From`
/// below *is* the synthesis that `steps()` used to perform on every read,
/// moved to the one place where it can still see the fields it needs — and
/// with it moved, those fields stop being a second way to say the same
/// thing that every reader had to remember to check.
///
/// Compatibility runs in both directions, unevenly, and it is worth being
/// exact about which way is safe:
///
///   - **New code reads old documents.** Every legacy field is optional
///     here, so a document from any earlier build converts, and the
///     precedence below reproduces what the old `steps()` computed from it.
///   - **Old builds reading new documents** is the direction that does not
///     fully hold. A new document omits all seven legacy keys, and five of
///     them were `#[serde(default)]` in the old struct and so would be
///     missed harmlessly — but `groupKeys` and `aggregates` were required
///     there, so an older build refuses a document written by this one.
///     That is a one-way door, and it is the reason the fields are read
///     here rather than simply deleted: the documents already in the world
///     have to keep opening, even though the ones written from now on will
///     not open in yesterday's build.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredFrameDerivation {
    source_frame_id: Id,
    #[serde(default)]
    steps: Vec<FrameStep>,
    #[serde(default)]
    join: Option<FrameJoin>,
    #[serde(default)]
    filters: Vec<Expr>,
    #[serde(default = "default_true")]
    filter_match_all: bool,
    #[serde(default)]
    projections: Vec<DerivedExpression>,
    #[serde(default)]
    group_keys: Vec<DerivedExpression>,
    #[serde(default)]
    aggregates: Vec<DerivedExpression>,
    #[serde(default)]
    sorts: Vec<DerivedSort>,
    #[serde(default = "default_true")]
    maintain_order: bool,
}

impl From<StoredFrameDerivation> for FrameDerivation {
    fn from(stored: StoredFrameDerivation) -> Self {
        let StoredFrameDerivation {
            source_frame_id,
            steps,
            join,
            filters,
            filter_match_all,
            projections,
            group_keys,
            aggregates,
            sorts,
            maintain_order,
        } = stored;
        // An explicit chain is the whole answer, and a join is the whole
        // answer when there is no chain — in both cases the flat fields
        // are ignored rather than merged in. That is not tidiness: it is
        // the precedence the old `steps()` had, and a document that
        // somehow carries both a join and stray flat fields has to keep
        // meaning what it meant, which was the join alone.
        if !steps.is_empty() || join.is_some() {
            return FrameDerivation {
                source_frame_id,
                join,
                steps,
            };
        }
        let mut steps = Vec::new();
        if !filters.is_empty() {
            steps.push(FrameStep::Filter {
                predicates: filters,
                match_all: filter_match_all,
            });
        }
        if !projections.is_empty() {
            let column_ids = projections
                .iter()
                .map(|projection| projection.output_column_id.clone())
                .collect();
            steps.push(FrameStep::WithColumns {
                columns: projections,
            });
            steps.push(FrameStep::Select { column_ids });
        } else if !aggregates.is_empty() || !group_keys.is_empty() {
            steps.push(FrameStep::Summarize {
                group_keys,
                aggregates,
                maintain_order,
            });
        }
        if !sorts.is_empty() {
            steps.push(FrameStep::Sort { keys: sorts });
        }
        FrameDerivation {
            source_frame_id,
            join: None,
            steps,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UniqueKeyConstraint {
    pub id: Id,
    pub column_ids: Vec<Id>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FrameJoinType {
    Left,
    Inner,
    Anti,
    Semi,
}

impl FrameJoinType {
    pub fn keeps_lookup_columns(self) -> bool {
        matches!(self, FrameJoinType::Left | FrameJoinType::Inner)
    }

    pub fn label(self) -> &'static str {
        match self {
            FrameJoinType::Left => "left",
            FrameJoinType::Inner => "inner",
            FrameJoinType::Anti => "anti",
            FrameJoinType::Semi => "semi",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameJoin {
    pub lookup_frame_id: Id,
    pub primary_key_column_ids: Vec<Id>,
    pub lookup_key_column_ids: Vec<Id>,
    pub join_type: FrameJoinType,
    pub outputs: Vec<JoinOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JoinOutput {
    pub output_column_id: Id,
    pub source_frame_id: Id,
    pub source_column_id: Id,
}

impl FrameDerivation {
    /// The chain this derivation runs, whichever of the two shapes holds it.
    ///
    /// A join is not kept as a step, so it is turned into one here. That
    /// costs a clone on every ask, and it buys every reader downstream —
    /// planning, lineage, rendering, validation — the right to think in
    /// steps and nothing else.
    pub fn steps(&self) -> Cow<'_, [FrameStep]> {
        match &self.join {
            Some(join) if self.steps.is_empty() => {
                Cow::Owned(vec![FrameStep::Join { join: join.clone() }])
            }
            _ => Cow::Borrowed(&self.steps),
        }
    }

    pub fn step_expressions(&self) -> Vec<Expr> {
        self.steps()
            .iter()
            .flat_map(|step| match step {
                FrameStep::Filter { predicates, .. } => predicates.clone(),
                FrameStep::WithColumns { columns } => {
                    columns.iter().map(|item| item.expression.clone()).collect()
                }
                FrameStep::Summarize {
                    group_keys,
                    aggregates,
                    ..
                } => group_keys
                    .iter()
                    .chain(aggregates.iter())
                    .map(|item| item.expression.clone())
                    .collect(),
                FrameStep::Select { .. }
                | FrameStep::Join { .. }
                | FrameStep::Sort { .. }
                | FrameStep::Union { .. }
                | FrameStep::Expand { .. }
                | FrameStep::Pivot { .. }
                | FrameStep::Unpivot { .. }
                | FrameStep::Comment { .. } => Vec::new(),
            })
            .collect()
    }

    pub fn join_step(&self) -> Option<FrameJoin> {
        self.steps().iter().find_map(|step| match step {
            FrameStep::Join { join } => Some(join.clone()),
            _ => None,
        })
    }

    /// Every other frame this chain reads through a two-input step, in
    /// chain order. The flat join is covered because `steps()` synthesizes
    /// it.
    pub fn lookup_frame_ids(&self) -> Vec<Id> {
        self.steps()
            .iter()
            .filter_map(|step| step.lookup_frame_id().cloned())
            .collect()
    }

    pub fn references_frame(&self, frame_id: &str) -> bool {
        self.source_frame_id == frame_id
            || self
                .lookup_frame_ids()
                .iter()
                .any(|lookup_id| lookup_id == frame_id)
    }

    pub fn references_input_column(&self, frame_id: &str, column_id: &str) -> bool {
        if let Some(join) = self.join_step()
            && ((self.source_frame_id == frame_id
                && join.primary_key_column_ids.iter().any(|id| id == column_id))
                || (join.lookup_frame_id == frame_id
                    && join.lookup_key_column_ids.iter().any(|id| id == column_id))
                || join.outputs.iter().any(|output| {
                    output.source_frame_id == frame_id && output.source_column_id == column_id
                }))
        {
            return true;
        }
        // Union and expansion read foreign columns by id, the way a join
        // reads its outputs — dropping one out from under either has to be
        // refused the same way.
        if self.steps().iter().any(|step| match step {
            FrameStep::Union {
                frame_id: stacked_frame_id,
                mapping,
            } => {
                stacked_frame_id == frame_id
                    && mapping
                        .iter()
                        .any(|column| column.source_column_id.as_deref() == Some(column_id))
            }
            FrameStep::Expand {
                frame_id: expanded_frame_id,
                outputs,
            } => {
                expanded_frame_id == frame_id
                    && outputs
                        .iter()
                        .any(|output| output.source_column_id == column_id)
            }
            _ => false,
        }) {
            return true;
        }
        self.source_frame_id == frame_id
            && self
                .step_expressions()
                .iter()
                .any(|expression| expression.references_column(column_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DerivedExpression {
    pub output_column_id: Id,
    #[ts(type = "unknown")]
    pub expression: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DerivedSort {
    pub column_id: Id,
    pub descending: bool,
}

fn default_true() -> bool {
    true
}
