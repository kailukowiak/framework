use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DocumentView {
    #[serde(flatten)]
    pub document: Document,
    pub computed_frames: HashMap<Id, ComputedFrame>,
    pub computed_results: HashMap<Id, ComputedResult>,
    pub computed_blocks: HashMap<Id, ComputedBlock>,
    pub computed_texts: HashMap<Id, ComputedText>,
    pub formula_functions: Vec<FormulaFunction>,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// A result as the canvas shows it: the formula rendered back to text, and
/// the answer as it stands right now. Recomputed with every view, which is
/// what "live" means here — the card can never show yesterday's number
/// against today's inputs.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedResult {
    pub formula: String,
    pub data_type: DataType,
    /// Present when this answer was written down rather than worked out.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub frozen: Option<FrozenState>,
    #[serde(flatten)]
    pub cell: ComputedCell,
}

/// A frozen answer as the card reports it: when it was taken, and whether
/// anything it was taken from has changed since.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrozenState {
    pub taken_at: String,
    pub stale: bool,
}

/// A block as the canvas shows it: the text to edit, and one answer per
/// line of it, in the block's own order — because the order is part of what
/// a block says, and because the gutter has to line up with the text.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedBlock {
    /// Every line's text, newline-joined: the block as somebody typed it,
    /// with the names they typed still on the front of the lines they named.
    pub source: String,
    pub lines: Vec<ComputedBlockLine>,
}

/// One line's worth of [`ComputedResult`], carrying the line's identity so
/// the interface can match answers back to lines without counting.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedBlockLine {
    pub id: Id,
    pub name: String,
    /// This line's own text, so a gutter can be drawn from the answers
    /// alone.
    pub text: String,
    /// Prose rather than arithmetic: no name, no answer, nothing in the
    /// gutter.
    pub comment: bool,
    pub blank: bool,
    pub data_type: DataType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub frozen: Option<FrozenState>,
    #[serde(flatten)]
    pub cell: ComputedCell,
}

/// A text card as the canvas shows it: the source to edit, and each piece
/// in order — prose verbatim, and every `{{…}}` hole with its answer as it
/// stands right now. Recomputed with every view, like a result: the card
/// can never print yesterday's number inside today's sentence.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedText {
    /// The card's editable text, reconstructed from the segments: prose
    /// byte for byte, each hole rendered back from its formula — which is
    /// how a rename elsewhere shows up here without an edit.
    pub source: String,
    pub segments: Vec<ComputedTextSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ComputedTextSegment {
    Literal {
        text: String,
    },
    Value {
        formula: String,
        data_type: DataType,
        #[serde(flatten)]
        cell: ComputedCell,
    },
    Broken {
        source: String,
        error: String,
    },
}

/// The Polars plan behind a frame as it is displayed, as text.
///
/// `logical` is the plan as written -- wrangle chain, then the display
/// filter and sort -- and `optimized` is what Polars actually runs after
/// predicate pushdown and projection pruning. Reading both is how you
/// confirm a filter written against one frame reached the scan of the frame
/// it was derived from.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameQueryPlan {
    pub frame_id: Id,
    pub logical: String,
    pub optimized: String,
}

/// What a chain looks like at every position: the columns each step leaves
/// behind, and where the walk stopped if a step could not be worked out.
///
/// Answered from the query plan, which costs no scan — `collect_schema()`
/// resolves names and types without touching a row, and every step that
/// exists today is resolvable that way.
///
/// The pivot step is the one that tested this, and it chose to keep it
/// true: its output columns are named after the *values* in a column, so
/// it looks at the data once, when the step is written, and bakes what it
/// finds into the step — the way a join bakes its outputs. After that the
/// schema is a property of the plan again. A step that instead chooses to
/// be genuinely data-dependent — unnesting fields not known ahead of time
/// — cannot be answered without running the query, which is why a step's
/// schema stays a struct rather than a bare column list: such a step will
/// need to say so, so the editor can offer to run a sample instead of
/// quietly showing the wrong shape.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PipelineSchema {
    pub frame_id: Id,
    /// What the first step sees.
    pub input_columns: Vec<Column>,
    /// One entry per step the walk got through, in order.
    pub steps: Vec<StepSchema>,
    /// The step that stopped the walk, if one did. Steps at and after this
    /// index have no schema, because working one out needs the step before
    /// it to have parsed.
    pub failed_step: Option<usize>,
    pub error: Option<String>,
}

/// The columns visible after one step.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StepSchema {
    pub columns: Vec<Column>,
}

/// The first few rows as they stand partway down a chain.
///
/// Unlike the schema, this runs the query — bounded by `limit`, and with
/// the limit pushed into the plan, so a step over four million rows reads
/// what it needs rather than the lot. Still not free, which is why it is
/// fetched once when a step's preview is opened rather than on every
/// keystroke.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StepSample {
    pub frame_id: Id,
    /// The step this shows the output of.
    pub step_index: usize,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
    /// Whether the chain has more rows than were asked for. Not a count:
    /// counting is a scan, and the point of a sample is not doing one.
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FramePage {
    pub frame_id: Id,
    pub total_rows: usize,
    pub offset: usize,
    pub limit: usize,
    pub columns: Vec<Column>,
    /// The identity of each row on the page, in page order — what a cell
    /// edit names. Positional for rows that came straight out of a scan and
    /// so never had one.
    pub row_ids: Vec<Id>,
    pub rows: Vec<Vec<String>>,
    /// What the conditional-formatting rules make of each row on the page,
    /// in page order and in the frame's own rule order.
    ///
    /// Read on the page rather than on the frame because a paged frame's
    /// rows only exist as pages: the rules run over the thousand rows being
    /// looked at, not over the four million behind them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub style_matches: Vec<Vec<FrameStyleMatch>>,
}

/// One rule's answer for one row, already read as style.
///
/// The style rather than the value, because what a value means is the rule's
/// business: `true`, `"Refunded"`, and `-1284.20` are three different
/// questions with one answer shape. The rule id rides along so the interface
/// can tell which columns the answer is allowed to reach — the rule's own
/// scope — without knowing anything else about it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameStyleMatch {
    pub rule_id: Id,
    pub style: FrameCellStyle,
}

/// A bounded window into one scratchwork answer. Scalar answers have one
/// value; list answers page without forcing the document snapshot or webview
/// to carry the whole series merely because the block is visible.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlockLinePage {
    pub block_id: Id,
    pub line_id: Id,
    pub total_values: usize,
    pub offset: usize,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedFrame {
    /// Everything that decides the rows this frame would hand back, as one
    /// hash: its own definition and, recursively, every frame upstream.
    ///
    /// What a reader keys a cached page on. The document's revision number
    /// would do the same job and do it far too eagerly — it moves for every
    /// edit anywhere, so typing in a scratchpad on the other side of the
    /// canvas would throw away every page of a million-row frame and fetch
    /// them again. Lineage-scoped is the whole point of the hash.
    pub fingerprint: String,
    pub formulas: HashMap<Id, String>,
    pub override_formulas: HashMap<Id, HashMap<Id, String>>,
    pub rows: HashMap<Id, HashMap<Id, ComputedCell>>,
    pub summaries: HashMap<Id, ComputedCell>,
    pub derivation: Option<RenderedFrameDerivation>,
    /// The frame's transformation chain, every formula rendered back to the
    /// text that was written rather than an expression tree.
    ///
    /// One field for both kinds of frame: a derived frame's chain and a
    /// source frame's own chain are the same list of steps, and the editor
    /// that shows them does not need to care which it is looking at.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<RenderedFrameStep>,
    /// How many leading entries of `steps` are plumbing rather than
    /// transformation: the identity projection a linked frame carries so it
    /// owns its own column ids, and the select that adopts them.
    ///
    /// Nobody writes these, so the editor does not draw them — but it must
    /// keep them. Dropping the projection would strand every column id this
    /// frame has published to formulas, plots, and frames derived from it.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pass_through_steps: usize,
    /// The display layer's filter and sort, rendered the same way — what the
    /// View tab edits, against what the Wrangle tab edits above.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_steps: Vec<RenderedFrameStep>,
    /// Row count when it is known without running a query: an artifact
    /// records its own, and an in-memory frame is its rows. `None` means
    /// the count would cost a full pass over the lazy plan -- a derived
    /// frame read through pages -- so the caller takes it from the
    /// [`FramePage`] it was going to read anyway. Counting eagerly here
    /// would run a whole aggregation on every `view()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub total_rows: Option<usize>,
    // `default` means an absent key deserializes, and the hand-written mirror
    // this replaced spelled it `paged?:` — so `?:` is both true and the shape
    // the interface has been checked against. Serializing always writes it.
    #[serde(default)]
    #[ts(optional, as = "Option<bool>")]
    pub paged: bool,
    /// Present when this frame is cached to a snapshot rather than read
    /// live. `stale` means something upstream has changed since the
    /// snapshot was written — the rows on screen are still the snapshot's,
    /// and stay that way until it is refreshed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub materialization: Option<ComputedMaterialization>,
    /// A generated frame's rule, rendered back to the text a person would
    /// retype — the editable face of `FrameObject::generator`, whose stored
    /// expression tree the interface deliberately cannot read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub generator_rule: Option<String>,
    /// What this frame lets someone edit by hand, and why not when not.
    pub editing: FrameEditing,
    /// True when this frame's values can change without anyone editing the
    /// document, because it — or something it reads from — re-reads a file.
    #[serde(default, skip_serializing_if = "is_false")]
    pub live: bool,
    /// The file this frame reads from, named for showing: "ledger.csv".
    ///
    /// The document holds the full path, which is the wrong length for a
    /// list and the right length for a tooltip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_name: Option<String>,
    /// True when a frame somewhere up this one's lineage is serving a stale
    /// snapshot.
    ///
    /// Staleness inherits, because reading from old numbers produces old
    /// numbers. A live frame computes honestly from whatever it is given,
    /// and if what it is given is a snapshot nobody has refreshed, its own
    /// rows are as out of date as that snapshot — with nothing in its own
    /// definition to say so. A cached frame can carry both flags at once:
    /// its snapshot can match its own lineage exactly and still have been
    /// computed from a parent's stale one.
    #[serde(default, skip_serializing_if = "is_false")]
    pub upstream_stale: bool,
    /// What the conditional-formatting rules make of each row, by row id, in
    /// the frame's own rule order.
    ///
    /// Only the rows this frame holds itself. A paged frame's rows arrive a
    /// page at a time and carry their own matches on the page, because the
    /// rules are run over the rows being read rather than over the frame.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub style_matches: HashMap<Id, Vec<FrameStyleMatch>>,
    /// Rules that could not be run, by rule id, so the inspector can say so
    /// against the rule that broke rather than failing the frame.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub style_rule_errors: HashMap<Id, String>,
    /// Each rule's hidden column rendered back to the text somebody wrote,
    /// by rule id — the same service `formulas` performs for calculated
    /// columns, and for the same reason: the interface reads formulas as
    /// text and writes them back as text, and never opens an expression.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub style_rule_formulas: HashMap<Id, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedMaterialization {
    pub row_count: usize,
    pub stale: bool,
}

/// What this frame lets someone do to it by hand, and why not when not.
///
/// Reported rather than left for the interface to work out. The rule is
/// small — does the frame own its rows — but it was being re-derived from
/// the model in every place that had to ask, so a grid, a context menu and
/// a paste path each carried their own copy of it and could each be wrong
/// on their own. One answer, computed where the rule is enforced.
///
/// `reason` is written to be shown. A control that is simply missing, or
/// disabled without explanation, reads as a broken feature rather than a
/// frame that works differently — and "why can't I type here" has a real
/// answer worth giving.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameEditing {
    /// Whether a value can be typed into a cell. Still false per-column for
    /// a calculated column, which has a formula instead of a value.
    pub cells: bool,
    /// Whether rows can be added or deleted by hand.
    pub rows: bool,
    /// Whether a one-off override on a single cell takes effect.
    ///
    /// The narrower door in the wall, and it is not always open. An override
    /// is recorded against a row of the document, and a frame read a page at
    /// a time has no rows in the document to record it against — its values
    /// are scanned from a parquet on every read, and the override would sit
    /// there being nothing. Offering it anyway is how "the edit did nothing"
    /// happens twice in one product.
    pub overrides: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,
}

impl FrameEditing {
    /// `live` is whether the frame's values can change without anyone
    /// editing the document, and `paged` whether they are read from a file
    /// rather than held in it. Both are lineage facts rather than facts
    /// about this frame alone, which is why they arrive as arguments.
    pub(crate) fn for_frame(frame: &FrameObject, cells: bool, live: bool, paged: bool) -> Self {
        if cells {
            return Self {
                cells: true,
                // Rows can only be added where they are held. A parquet the
                // document owns can have its values rewritten in place, but
                // growing it is a different operation than editing it.
                rows: frame.owns_its_rows(),
                overrides: !paged,
                reason: (!frame.owns_its_rows()).then(|| {
                    "These rows are the document's own copy — type into them freely. \
                     Each edit rewrites the file they live in."
                        .to_string()
                }),
            };
        }
        let overrides = !paged;
        let mend = if overrides {
            " Edit the chain, or set a one-off override on a cell."
        } else if live {
            " Add a calculated column, or edit the source and refresh."
        } else {
            " Add a calculated column to work from them."
        };
        let origin = if frame.generator.is_some() {
            "These rows are grown by this frame's rule. Edit the rule to change them.".to_string()
        } else if frame.derivation.is_some() {
            "These rows are computed by the chain above them.".to_string()
        } else if live {
            match frame.source_name() {
                // Said in the future tense on purpose: the point is not that
                // the value came from a file, it is that the next refresh
                // will replace whatever was typed over it.
                Some(source) => {
                    format!("These rows are read from {source}, and refreshing replaces them.")
                }
                None => "These rows are read from this frame's source, and refreshing \
                         replaces them."
                    .to_string(),
            }
        } else {
            match frame.source_name() {
                Some(source) => format!(
                    "These rows are read from the copy of {source} imported into this \
                     document."
                ),
                None => {
                    "These rows are read from the copy imported into this document.".to_string()
                }
            }
        };
        Self {
            cells: false,
            rows: false,
            overrides,
            reason: Some(format!("{origin}{mend}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
/// That this frame is derived, and from what.
///
/// Deliberately thin: a derivation's chain travels as `ComputedFrame::steps`
/// like any other chain, so what is left here is the pair of facts the steps
/// do not carry — the frame the chain starts from, and the join, which is
/// held flat on `FrameDerivation` rather than as a step.
pub struct RenderedFrameDerivation {
    pub source_frame_id: Id,
    pub join: Option<FrameJoin>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedCell {
    /// Backward-compatible numeric projection. Non-numeric and null values use `None`.
    pub value: Option<f64>,
    pub typed_value: ScalarValue,
    pub display: String,
    pub error: Option<String>,
    pub is_override: bool,
}

fn is_zero(count: &usize) -> bool {
    *count == 0
}

fn is_false(flag: &bool) -> bool {
    !*flag
}

impl Document {
    pub(crate) fn compute_frames(&self) -> HashMap<Id, ComputedFrame> {
        self.objects
            .iter()
            .filter_map(|object| match object {
                DataObject::Frame(frame) => Some((frame.id.clone(), frame.compute(self))),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn compute_results(&self) -> HashMap<Id, ComputedResult> {
        self.objects
            .iter()
            .filter_map(|object| match object {
                DataObject::Result(result) => {
                    Some((result.id.clone(), self.compute_result(result)))
                }
                _ => None,
            })
            .collect()
    }

    pub(crate) fn compute_blocks(&self) -> HashMap<Id, ComputedBlock> {
        self.objects
            .iter()
            .filter_map(|object| match object {
                DataObject::Block(block) => Some((block.id.clone(), self.compute_block(block))),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn compute_texts(&self) -> HashMap<Id, ComputedText> {
        self.objects
            .iter()
            .filter_map(|object| match object {
                DataObject::Text(text) => Some((text.id.clone(), self.compute_text(text))),
                _ => None,
            })
            .collect()
    }

    fn compute_text(&self, text: &TextObject) -> ComputedText {
        let mut source = String::new();
        let segments = text
            .effective_segments()
            .into_iter()
            .map(|segment| match segment {
                TextSegment::Literal { text } => {
                    source.push_str(&text);
                    ComputedTextSegment::Literal { text }
                }
                TextSegment::Formula { formula } => {
                    let rendered = self.render_formula_scalar(&formula.expression);
                    source.push_str("{{");
                    source.push_str(&rendered);
                    source.push_str("}}");
                    let (data_type, evaluated) = self.evaluate_value(&text.id, &formula.expression);
                    ComputedTextSegment::Value {
                        formula: rendered,
                        data_type,
                        cell: computed_cell(evaluated, data_type, false),
                    }
                }
                TextSegment::Broken {
                    source: hole,
                    error,
                } => {
                    source.push_str("{{");
                    source.push_str(&hole);
                    source.push_str("}}");
                    ComputedTextSegment::Broken {
                        source: hole,
                        error,
                    }
                }
            })
            .collect();
        ComputedText { source, segments }
    }

    /// Each line evaluated on its own, the way a result is. A line that
    /// fails reports its error in its own gutter and the lines below keep
    /// computing — until one reads the broken line, at which point the
    /// failure arrives where it is felt, with the message intact.
    fn compute_block(&self, block: &BlockObject) -> ComputedBlock {
        let text = |line: &BlockLine| {
            // A line stored before a block was text — parsed, with no source
            // kept — is written back out from its formula. It becomes real
            // text the first time the block is edited, so this is a reading
            // of the old shape rather than a migration to keep forever.
            match line.expression() {
                Some(expression) if line.source.is_empty() => {
                    let rendered =
                        expression.render_in_scope(&FrameObject::default(), self, Some(block), 0);
                    match line.name.is_empty() {
                        true => rendered,
                        false => format!("{} = {}", line.name, rendered),
                    }
                }
                _ => line.text(),
            }
        };
        ComputedBlock {
            source: block.lines.iter().map(text).collect::<Vec<_>>().join("\n"),
            lines: block
                .lines
                .iter()
                .map(|line| {
                    let mut list = None;
                    let (data_type, evaluated) = match line.expression() {
                        Some(expression) => {
                            let answer = self.evaluate_line(&line.id, expression);
                            list = answer.2;
                            (answer.0, answer.1)
                        }
                        // A line with no formula has a reason instead: the
                        // parser's, or the edit's. Either way it belongs in
                        // this line's gutter and nowhere else.
                        None => (
                            DataType::String,
                            Err(line.error.clone().unwrap_or_default()),
                        ),
                    };
                    ComputedBlockLine {
                        id: line.id.clone(),
                        name: line.name.clone(),
                        text: text(line),
                        comment: line.is_comment(),
                        blank: line.is_blank(),
                        data_type,
                        frozen: line
                            .expression()
                            .and_then(|expression| self.frozen_state(&line.id, expression)),
                        cell: match list {
                            // A list has no single value to project, so the
                            // cell carries only what it reads as.
                            Some(display) => ComputedCell {
                                value: None,
                                typed_value: ScalarValue::Null,
                                display,
                                error: None,
                                is_override: false,
                            },
                            None => computed_cell(evaluated, data_type, false),
                        },
                    }
                })
                .collect(),
        }
    }

    fn compute_result(&self, result: &ResultObject) -> ComputedResult {
        let (data_type, evaluated) = self.evaluate_value(&result.id, &result.formula.expression);
        ComputedResult {
            formula: self.render_formula_scalar(&result.formula.expression),
            data_type,
            frozen: self.frozen_state(&result.id, &result.formula.expression),
            cell: computed_cell(evaluated, data_type, false),
        }
    }

    /// A value's answer: the one written down if there is one, and otherwise
    /// the one worked out now.
    ///
    /// The order is the whole rule. A frozen answer wins even when the
    /// formula could be evaluated live, because that is what freezing means
    /// — the number on the card is the number that was taken, until somebody
    /// refreshes it.
    pub(crate) fn evaluate_value(
        &self,
        object_id: &str,
        expression: &Expr,
    ) -> (DataType, Result<ScalarValue, String>) {
        if let Some(frozen) = self.frozen_values.get(object_id) {
            return match read_frozen_answer(&frozen.artifact.path) {
                Ok((data_type, value)) => (data_type, Ok(value)),
                Err(error) => (DataType::String, Err(error)),
            };
        }
        self.evaluate_result_expression(expression)
    }

    /// What to say on the card about a written-down answer.
    pub(crate) fn frozen_state(&self, object_id: &str, expression: &Expr) -> Option<FrozenState> {
        let frozen = self.frozen_values.get(object_id)?;
        Some(FrozenState {
            taken_at: frozen.taken_at.clone(),
            stale: self.frozen_is_stale(object_id, expression).unwrap_or(false),
        })
    }

    /// Whether a frozen answer still describes the thing it was taken from.
    ///
    /// Stale is reported, never repaired: a recorded number quietly changing
    /// under someone is the failure this whole mechanism exists to avoid.
    pub(crate) fn frozen_is_stale(&self, object_id: &str, expression: &Expr) -> Option<bool> {
        let frozen = self.frozen_values.get(object_id)?;
        Some(frozen.fingerprint != self.value_fingerprint(expression))
    }

    /// Everything that decides what a value evaluates to: the formula, and
    /// the lineage of every frame it reads. Editing either is what makes a
    /// frozen answer stale.
    pub(crate) fn value_fingerprint(&self, expression: &Expr) -> String {
        let mut hasher = Sha256::new();
        hasher.update(
            serde_json::to_string(expression)
                .unwrap_or_default()
                .as_bytes(),
        );
        let mut frames = Vec::new();
        expression.foreign_frames(&mut frames);
        frames.sort_unstable();
        frames.dedup();
        for frame_id in frames {
            self.hash_frame_lineage(frame_id, &mut hasher, &mut HashSet::new(), true, false);
        }
        format!("{:x}", hasher.finalize())
    }

    /// A line's answer: frozen if the author explicitly wrote it down,
    /// otherwise worked out from the current document — and allowed to be a
    /// list either way.
    pub(crate) fn evaluate_line(
        &self,
        object_id: &str,
        expression: &Expr,
    ) -> (DataType, Result<ScalarValue, String>, Option<String>) {
        // Legacy and explicitly captured answers still read back at whatever
        // length was recorded. Ordinary Scratchwork never enters this branch:
        // it evaluates live below and may return the whole current list.
        if let Some(frozen) = self.frozen_values.get(object_id) {
            return match read_frozen_series(&frozen.artifact.path) {
                Ok((data_type, series)) if series.len() == 1 => {
                    (data_type, crate::polars_value_at(&series, 0), None)
                }
                Ok((data_type, series)) => (
                    data_type,
                    Ok(ScalarValue::Null),
                    Some(render_list(&series, data_type)),
                ),
                Err(error) => (DataType::String, Err(error), None),
            };
        }
        // Scratchwork is the place for live, ad-hoc calculation. Requiring a
        // line to freeze merely because it reads an unmaterialized frame made
        // an ordinary row edit leave the answer behind. The compiler already
        // knows how to evaluate both document-owned and derived frames lazily;
        // a frozen value above remains the explicit way to pin an answer.
        self.evaluate_line_expression(expression)
    }

    pub(crate) fn block_line_page(
        &self,
        block_id: &str,
        line_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<BlockLinePage, CoreError> {
        let line = self
            .block(block_id)?
            .lines
            .iter()
            .find(|line| line.id == line_id)
            .ok_or(CoreError::ObjectNotFound)?;
        let expression = line.expression().ok_or_else(|| {
            CoreError::InvalidOperation("That scratchwork line has no result".into())
        })?;
        let (data_type, series) = if let Some(frozen) = self.frozen_values.get(line_id) {
            read_frozen_series(&frozen.artifact.path).map_err(CoreError::Formula)?
        } else {
            self.evaluate_scratchwork_series(expression)
                .map_err(CoreError::Formula)?
        };
        let total_values = series.len();
        let start = offset.min(total_values);
        let end = start.saturating_add(limit).min(total_values);
        let values = (start..end)
            .map(|index| {
                crate::polars_value_at(&series, index)
                    .map(|value| crate::engine::values::format_scalar_value(&value, data_type))
                    .unwrap_or_else(|_| "—".into())
            })
            .collect();
        Ok(BlockLinePage {
            block_id: block_id.into(),
            line_id: line_id.into(),
            total_values,
            offset: start,
            values,
        })
    }

    /// A block line's answer, which may be a list.
    ///
    /// The one place in this document where an expression is not required to
    /// fold down to a single value. A scratchpad line holds whatever it
    /// evaluates to — `4 * `Rates`` is three numbers, and saying so is more
    /// use than refusing it — so the gutter shows the list and its length
    /// rather than an error telling somebody to aggregate something they did
    /// not want aggregated.
    pub(crate) fn evaluate_line_expression(
        &self,
        expression: &Expr,
    ) -> (DataType, Result<ScalarValue, String>, Option<String>) {
        let (data_type, series) = match self.evaluate_scratchwork_series(expression) {
            Ok(found) => found,
            Err(error) => return (DataType::String, Err(error), None),
        };
        if series.len() == 1 {
            return (data_type, crate::polars_value_at(&series, 0), None);
        }
        (
            data_type,
            Ok(ScalarValue::Null),
            Some(render_list(&series, data_type)),
        )
    }

    /// One scalar out of a formula that sits in no frame.
    ///
    /// A literal broadcasts to one value, an aggregate folds to one, and
    /// anything still list-shaped comes back as an error rather than a first
    /// element pretending to be the answer. The live dependency preparation
    /// is shared with Scratchwork: a semantic aggregate over a current frame,
    /// or over a live upstream Scratchwork line, remains a current answer.
    pub(crate) fn evaluate_result_expression(
        &self,
        expression: &Expr,
    ) -> (DataType, Result<ScalarValue, String>) {
        let (data_type, series) = match self.evaluate_scratchwork_series(expression) {
            Ok(evaluated) => evaluated,
            Err(error) => return (DataType::String, Err(error)),
        };
        if series.len() != 1 {
            return (
                DataType::String,
                Err(format!(
                    "This formula produced {} values rather than one. \
                     Fold the list down — .sum(), .mean(), .max() — to make it a result.",
                    series.len()
                )),
            );
        }
        (data_type, polars_value_at(&series, 0))
    }

    /// The same evaluation, stopping at the series rather than insisting it
    /// holds one value. What a block line is worked out with.
    /// A rule's rows: the expression evaluated in scalar scope, with a list
    /// answer opened out so `sequence(0, 16)` is sixteen rows rather than
    /// one row holding sixteen. What a generated frame's base plan is built
    /// from, and what preparing one evaluates to learn the column's type.
    pub(crate) fn evaluate_rule_series(
        &self,
        expression: &Expr,
    ) -> Result<(DataType, polars::prelude::Series), String> {
        use polars::prelude::IntoLazy;
        let compiled = expression.to_polars(self)?;
        let frame = polars::df!("__rule_probe" => [true])
            .map_err(|error| error.to_string())
            .and_then(|frame| {
                frame
                    .lazy()
                    .select([compiled.alias("__rule")])
                    .collect()
                    .map_err(|error| error.to_string())
            })
            .map_err(crate::engine::in_plain_words)?;
        let series = frame
            .column("__rule")
            .map_err(|error| error.to_string())?
            .as_materialized_series()
            .clone();
        let series = match series.dtype() {
            polars::prelude::DataType::List(_) => series
                .explode(polars::prelude::ExplodeOptions {
                    empty_as_null: false,
                    keep_nulls: false,
                })
                .map_err(|error| error.to_string())?,
            _ => series,
        };
        let data_type = framework_type_from_polars(series.dtype()).unwrap_or(DataType::String);
        Ok((data_type, series))
    }

    pub(crate) fn evaluate_to_series(
        &self,
        expression: &Expr,
    ) -> Result<(DataType, polars::prelude::Series), String> {
        use polars::prelude::IntoLazy;
        let compiled = expression.to_polars(self)?;
        let frame = polars::df!("__result_probe" => [true])
            .map_err(|error| error.to_string())
            .and_then(|frame| {
                frame
                    .lazy()
                    .select([compiled.alias("__result")])
                    .collect()
                    .map_err(|error| error.to_string())
            })
            .map_err(crate::engine::in_plain_words)?;
        let series = frame
            .column("__result")
            .map_err(|error| error.to_string())?
            .as_materialized_series()
            .clone();
        let found = framework_type_from_polars(series.dtype()).unwrap_or(DataType::String);
        Ok((self.written_type(expression, found), series))
    }

    /// The type an answer is *shown* as, which Polars cannot always supply.
    ///
    /// Polars knows a float. It does not know whether that float was written
    /// `0.0425`, `4.25%`, or `$4.25`, because money and percentages are one
    /// number and three ways of writing it — a distinction this document
    /// keeps and Polars has no place for. So where the expression itself
    /// says which, and Polars agrees it is a number at all, the expression
    /// is believed.
    ///
    /// Only in that direction. An expression that says percentage but comes
    /// back as text has been through a `cast` or a `format`, and the answer
    /// on the screen is the text.
    fn written_type(&self, expression: &Expr, found: DataType) -> DataType {
        written_type(found, expression.declared_type(self))
    }

    /// Whether `expression` reads `target_id`, looking *through* results and
    /// block lines: a formula that names either reads everything it reads.
    /// This is the check that keeps the document acyclic, asked before a
    /// result's or a line's formula is accepted.
    pub(crate) fn formula_reaches_object(&self, expression: &Expr, target_id: &str) -> bool {
        self.reaches(expression, target_id, None, &mut Vec::new())
    }

    /// The formula behind a value, whichever kind of holder it sits in.
    pub(crate) fn value_expression(&self, object_id: &str) -> Result<Expr, CoreError> {
        if let Ok(DataObject::Result(result)) = self.object(object_id) {
            return Ok(result.formula.expression.clone());
        }
        let (block, index) = self
            .block_line(object_id)
            .ok_or(CoreError::ObjectNotFound)?;
        block.lines[index]
            .expression()
            .cloned()
            .ok_or_else(|| CoreError::Formula("This line does not compute yet".into()))
    }

    /// What to call a value's answer on disk. Only ever a file name.
    pub(crate) fn value_name(&self, object_id: &str) -> String {
        if let Ok(object) = self.object(object_id) {
            return object.name().to_string();
        }
        self.block_line(object_id)
            .map(|(block, index)| format!("{}.{}", block.name, block.lines[index].name))
            .unwrap_or_else(|| "value".into())
    }

    /// The first frame this expression reads that holds no snapshot, if any.
    ///
    /// Used when a value is embedded into a frame, where an unrecorded live
    /// query could reach back into the frame compiling it, and when preparing
    /// live Scratchwork dependencies. Top-level scalar surfaces themselves do
    /// not use this as an evaluation gate: their graph is already cycle-checked.
    pub(crate) fn first_live_frame<'a>(&self, expression: &'a Expr) -> Option<&'a str> {
        let mut frames = Vec::new();
        expression.foreign_frames(&mut frames);
        frames.into_iter().find(|frame_id| {
            self.frame(frame_id)
                .is_ok_and(|frame| frame.materialization.is_none())
        })
    }

    /// What a frame formula is told when it tries to inline a value that reads
    /// live data: both explicit ways across that cycle boundary, named.
    pub(crate) fn freeze_required(&self, frame_id: &str) -> String {
        let name = self
            .frame(frame_id)
            .map(|frame| frame.name.clone())
            .unwrap_or_else(|_| frame_id.to_string());
        format!(
            "‘{name}’ has no snapshot, so this has to be written down before it can be read. \
             Freeze this answer — it is refreshable — or materialize ‘{name}’."
        )
    }

    /// The same question asked of a block that has not been applied yet.
    ///
    /// Retyping a scratchpad is what can newly close a loop, and the lines
    /// that would close it are the ones in hand rather than the ones in the
    /// document — so the walk has to be told to read the draft wherever it
    /// passes through that block.
    pub(crate) fn draft_formula_reaches_object(
        &self,
        expression: &Expr,
        target_id: &str,
        draft: (&str, &[BlockLine]),
    ) -> bool {
        self.reaches(expression, target_id, Some(draft), &mut Vec::new())
    }

    fn reaches(
        &self,
        expression: &Expr,
        target_id: &str,
        draft: Option<(&str, &[BlockLine])>,
        seen: &mut Vec<Id>,
    ) -> bool {
        let mut reached = false;
        expression.walk_values(&mut |object_id| {
            if reached || object_id == target_id {
                reached = true;
                return;
            }
            // Every id is followed once. The document is kept acyclic, so
            // this changes no answer — it is here so that a loop arriving
            // from disk or from a replica is a wrong answer rather than a
            // blown stack.
            if seen.iter().any(|visited| visited == object_id) {
                return;
            }
            seen.push(object_id.to_string());
            if let Ok(DataObject::Result(result)) = self.object(object_id) {
                if self.reaches(&result.formula.expression, target_id, draft, seen) {
                    reached = true;
                }
            } else if let Some(expression) = self.line_expression(object_id, draft)
                && self.reaches(&expression.clone(), target_id, draft, seen)
            {
                reached = true;
            }
        });
        reached
    }

    /// A block line's expression, preferring the draft when the line is one
    /// of the lines being rewritten.
    fn line_expression(&self, line_id: &str, draft: Option<(&str, &[BlockLine])>) -> Option<Expr> {
        if let Some((block_id, lines)) = draft {
            if let Some(line) = lines.iter().find(|line| line.id == line_id) {
                return line.expression().cloned();
            }
            // A line the draft dropped is gone, whatever the document still
            // says about it.
            if self
                .block_line(line_id)
                .is_some_and(|(owner, _)| owner.id == block_id)
            {
                return None;
            }
        }
        self.block_line(line_id)
            .and_then(|(block, index)| block.lines[index].expression().cloned())
    }

    pub(crate) fn materialized_for_view(&self) -> Document {
        let mut materialized = self.clone();
        let derived_ids = self
            .objects
            .iter()
            .filter_map(|object| match object {
                // A generated frame's rows are worked out the same way a
                // derived frame's are: from a rule, at view time.
                DataObject::Frame(frame)
                    if frame.derivation.is_some() || frame.generator.is_some() =>
                {
                    Some(frame.id.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for frame_id in derived_ids {
            if self.frame_depends_on_artifact(&frame_id, &mut HashSet::new()) {
                continue;
            }
            let Ok(data_frame) =
                self.materialize_frame_frame(&frame_id, Layer::Data, &mut HashSet::new())
            else {
                continue;
            };
            let Some(DataObject::Frame(frame)) = materialized
                .objects
                .iter_mut()
                .find(|object| object.id() == frame_id)
            else {
                continue;
            };
            frame.rows = frame_rows_from_polars(frame, &data_frame);
        }
        materialized
    }

    /// Hashes everything upstream that decides what `frame_id` evaluates to.
    ///
    /// The two flags apply to `frame_id` alone; the recursion fixes its own
    /// (snapshot yes, display no) for every frame upstream. An upstream
    /// snapshot *is* part of this frame's values — it is the rows this frame
    /// reads — while an upstream display layer never is, because display
    /// steps run at read time on whoever is doing the reading.
    ///
    /// What the callers vary is the treatment of `frame_id` itself, and they
    /// disagree on both flags: see [`Document::frame_fingerprint`] and
    /// [`Document::frame_fingerprint_string`] for why each wants what it
    /// wants.
    pub(crate) fn hash_frame_lineage(
        &self,
        frame_id: &str,
        hasher: &mut Sha256,
        visiting: &mut HashSet<Id>,
        include_snapshot: bool,
        include_display: bool,
    ) {
        if !visiting.insert(frame_id.to_string()) {
            return;
        }
        if let Ok(frame) = self.frame(frame_id) {
            hasher.update(frame_id.as_bytes());
            // Serializing is the point: any field that changes what the
            // frame produces changes the digest, with no hand-maintained
            // list of fields to quietly fall out of date.
            let mut parts = vec![
                serde_json::to_string(&frame.columns).ok(),
                serde_json::to_string(&frame.base_columns).ok(),
                serde_json::to_string(&frame.steps).ok(),
                serde_json::to_string(&frame.source_file).ok(),
                serde_json::to_string(&placeless(frame.artifact.as_ref())).ok(),
                serde_json::to_string(&frame.derivation).ok(),
                serde_json::to_string(&frame.generator).ok(),
                serde_json::to_string(&frame.entry_columns).ok(),
            ];
            if frame.derivation.is_none() && frame.generator.is_none() {
                // A derived frame's `rows` are a cache of its own output,
                // filled in only for the view. Hashing them would make the
                // fingerprint depend on which copy of the document asked.
                parts.push(serde_json::to_string(&frame.rows).ok());
            }
            if include_snapshot {
                parts.push(
                    serde_json::to_string(
                        &frame
                            .materialization
                            .as_ref()
                            .map(|cache| (placeless(Some(&cache.artifact)), &cache.fingerprint)),
                    )
                    .ok(),
                );
            }
            if include_display {
                parts.push(serde_json::to_string(&frame.display.steps).ok());
            }
            for part in parts.into_iter().flatten() {
                hasher.update(part.as_bytes());
            }
            if let Some(derivation) = &frame.derivation {
                self.hash_frame_lineage(&derivation.source_frame_id, hasher, visiting, true, false);
            }
            for lookup_id in frame.lookup_frame_ids() {
                self.hash_frame_lineage(&lookup_id, hasher, visiting, true, false);
            }
            // A formula naming another frame reads from it, which makes it
            // upstream in every sense this hash is for. Nothing about the
            // derivation records the edge, so it is collected from the
            // expressions themselves.
            for foreign_id in frame.foreign_frames() {
                self.hash_frame_lineage(foreign_id, hasher, visiting, true, false);
            }
            // A formula naming a value reads its *contents*, and the
            // serialized expression above holds only its id. Editing the
            // value changes every row this frame produces while leaving the
            // digest untouched — which is how a timesheet's Sum footer kept
            // showing the old period's answer after the anchor date moved.
            // So the contents get hashed too, chased through results and
            // block lines the same way compilation inlines them.
            self.hash_referenced_values(frame, hasher);
        }
        visiting.remove(frame_id);
    }

    /// Digests the current contents of every value object a frame's
    /// expressions read, looking through results and block lines to the
    /// values under them — the hash must move exactly when compilation
    /// would produce different rows.
    fn hash_referenced_values(&self, frame: &FrameObject, hasher: &mut Sha256) {
        fn collect(expression: &Expr, ids: &mut Vec<Id>) {
            expression.walk_values(&mut |object_id| ids.push(object_id.to_string()));
        }
        let mut ids = Vec::new();
        let derivation_steps = frame
            .derivation
            .as_ref()
            .map(|derivation| derivation.steps())
            .unwrap_or_default();
        for step in frame
            .steps
            .iter()
            .chain(frame.display.steps.iter())
            .chain(derivation_steps.iter())
        {
            for expression in crate::engine::frame::step_expressions(step) {
                collect(expression, &mut ids);
            }
        }
        if let Some(generator) = &frame.generator {
            collect(&generator.formula.expression, &mut ids);
        }
        for column in &frame.columns {
            if let Some(formula) = &column.formula {
                collect(&formula.expression, &mut ids);
            }
        }
        let mut visited = HashSet::new();
        while let Some(object_id) = ids.pop() {
            if !visited.insert(object_id.clone()) {
                continue;
            }
            match self.object(&object_id) {
                Ok(DataObject::Value(value)) => {
                    hasher.update(value.raw.as_bytes());
                    hasher.update([value.data_type as u8]);
                }
                Ok(DataObject::Result(result)) => collect(&result.formula.expression, &mut ids),
                _ => {
                    if let Some(expression) = self
                        .block_line(&object_id)
                        .and_then(|(block, index)| block.lines[index].expression())
                    {
                        collect(expression, &mut ids);
                    }
                }
            }
        }
    }

    /// The same lineage hash narrowed to a `u64` for keying in-memory
    /// caches, and taking this frame's own snapshot and display layer into
    /// account -- swapping a snapshot in or changing a display filter both
    /// change the rows a cache entry would serve.
    pub(crate) fn frame_fingerprint(&self, frame_id: &str) -> u64 {
        let mut hasher = Sha256::new();
        self.hash_frame_lineage(frame_id, &mut hasher, &mut HashSet::new(), true, true);
        let digest = hasher.finalize();
        u64::from_be_bytes(
            digest[..8]
                .try_into()
                .expect("a SHA-256 digest is 32 bytes"),
        )
    }

    /// A hash of everything that decides what `frame_id` evaluates to: its
    /// own definition and, recursively, the definitions of every frame it
    /// reads from.
    ///
    /// Deliberately not the document revision. An edit elsewhere in the
    /// document -- renaming another frame, moving a card, undoing either --
    /// leaves this unchanged, which is what lets a snapshot stay fresh and a
    /// cached sort stay valid across unrelated edits.
    ///
    /// The frame's *own* snapshot is excluded, because this is the value a
    /// snapshot records in order to detect staleness later: including it
    /// would make every snapshot stale the instant it was written.
    /// Snapshots upstream are included, since replacing one changes the
    /// values read here.
    ///
    /// The display layer is excluded too, and for the mirror-image reason: a
    /// snapshot holds data-layer rows, and display steps run on top of it at
    /// read time. Sorting a column must not mark a snapshot stale.
    pub(crate) fn frame_fingerprint_string(&self, frame_id: &str) -> String {
        let mut hasher = Sha256::new();
        self.hash_frame_lineage(frame_id, &mut hasher, &mut HashSet::new(), false, false);
        format!("{:x}", hasher.finalize())
    }

    /// Whether `frame_id`'s own snapshot has fallen behind its lineage.
    ///
    /// False for a frame with no snapshot: a live frame is never out of
    /// date, it just costs what it costs.
    pub(crate) fn snapshot_is_stale(&self, frame_id: &str) -> bool {
        self.frame(frame_id).is_ok_and(|frame| {
            frame
                .materialization
                .as_ref()
                .is_some_and(|materialization| {
                    self.frame_fingerprint_string(frame_id) != materialization.fingerprint
                })
        })
    }

    /// Whether this frame's values can change without anyone editing the
    /// document.
    ///
    /// True for a frame with a connector, which a refresh re-reads from its
    /// origin, and for one that scans a path on every read. It travels down
    /// the lineage the way staleness does, and for the same reason: a frame
    /// computed from live numbers is live, whatever its own definition says.
    /// A frame whose whole lineage is static is one nobody can change but
    /// the person editing the document.
    ///
    /// The distinction the interface needs it for is what a hand edit would
    /// mean. Typing over a live frame is writing something a refresh is
    /// going to discard; a static import merely holds values the document
    /// reads from a file rather than owns.
    pub(crate) fn frame_is_live(&self, frame_id: &str) -> bool {
        self.lineage_is_live(frame_id, &mut HashSet::new())
    }

    fn lineage_is_live(&self, frame_id: &str, visiting: &mut HashSet<Id>) -> bool {
        if !visiting.insert(frame_id.to_string()) {
            return false;
        }
        let live = self.frame(frame_id).is_ok_and(|frame| {
            frame.connector.is_some()
                || frame.source_file.is_some()
                || frame.derivation.as_ref().is_some_and(|derivation| {
                    self.lineage_is_live(&derivation.source_frame_id, visiting)
                })
                || frame
                    .lookup_frame_ids()
                    .iter()
                    .any(|lookup_id| self.lineage_is_live(lookup_id, visiting))
                || frame
                    .foreign_frames()
                    .into_iter()
                    .any(|foreign_id| self.lineage_is_live(foreign_id, visiting))
        });
        visiting.remove(frame_id);
        live
    }

    /// Whether a value can be typed into this frame's cells.
    ///
    /// Two ways to qualify, and one thing they have in common: the values on
    /// screen are values this document owns, so writing one is writing the
    /// thing itself rather than a note on top of something else.
    ///
    /// A frame that holds its rows qualifies. So does one whose rows live in
    /// a parquet the document owns — an import with no connector to refresh
    /// over it — because nothing else will ever write that file. What does
    /// not qualify is a frame with something above it: a chain or a
    /// derivation recomputes whatever is typed, and a live connector
    /// replaces it on the next refresh. For those, taking ownership is the
    /// way in, and it is a deliberate act rather than a silent one.
    pub(crate) fn frame_cells_are_editable(&self, frame_id: &str) -> bool {
        self.frame(frame_id).is_ok_and(|frame| {
            let row_preserving = frame.steps.iter().all(|step| {
                matches!(
                    step,
                    FrameStep::Filter { .. } | FrameStep::Sort { .. } | FrameStep::Comment { .. }
                )
            });
            if frame.derivation.is_some() || !row_preserving {
                return false;
            }
            frame.owns_its_rows() || (frame.artifact.is_some() && !self.frame_is_live(frame_id))
        })
    }

    /// Whether anything `frame_id` reads from is serving a stale snapshot.
    ///
    /// A frame's own fingerprint cannot answer this. It hashes the parent's
    /// snapshot *record* — the artifact it points at — which does not move
    /// while the parent sits there unrefreshed, so a child of a stale parent
    /// looks perfectly fresh by its own reckoning while reading numbers that
    /// are not.
    pub(crate) fn upstream_snapshot_is_stale(&self, frame_id: &str) -> bool {
        self.lineage_snapshot_is_stale(frame_id, &mut HashSet::new(), false)
    }

    fn lineage_snapshot_is_stale(
        &self,
        frame_id: &str,
        visiting: &mut HashSet<Id>,
        include_self: bool,
    ) -> bool {
        if !visiting.insert(frame_id.to_string()) {
            return false;
        }
        let stale = (include_self && self.snapshot_is_stale(frame_id))
            || self.frame(frame_id).is_ok_and(|frame| {
                frame.derivation.as_ref().is_some_and(|derivation| {
                    self.lineage_snapshot_is_stale(&derivation.source_frame_id, visiting, true)
                }) || frame
                    .lookup_frame_ids()
                    .iter()
                    .any(|lookup_id| self.lineage_snapshot_is_stale(lookup_id, visiting, true))
            });
        visiting.remove(frame_id);
        stale
    }

    /// Every cached frame, ordered so that each comes after everything it
    /// reads from.
    ///
    /// Order is the whole point of this list. Refreshing a parent rewrites
    /// the snapshot its children read, which moves their fingerprints and
    /// makes them stale in turn — so a pass that refreshes a child first
    /// recomputes it from numbers that are about to be replaced, and leaves
    /// it stale anyway. Walking parents first means each frame is recomputed
    /// once, from sources that are already final.
    pub(crate) fn snapshot_refresh_order(&self) -> Vec<Id> {
        let mut order = Vec::new();
        let mut done = HashSet::new();
        for object in &self.objects {
            if let DataObject::Frame(frame) = object {
                self.push_lineage_first(&frame.id, &mut done, &mut HashSet::new(), &mut order);
            }
        }
        order.retain(|frame_id| {
            self.frame(frame_id)
                .is_ok_and(|frame| frame.materialization.is_some())
        });
        order
    }

    fn push_lineage_first(
        &self,
        frame_id: &str,
        done: &mut HashSet<Id>,
        visiting: &mut HashSet<Id>,
        order: &mut Vec<Id>,
    ) {
        if done.contains(frame_id) || !visiting.insert(frame_id.to_string()) {
            return;
        }
        if let Ok(frame) = self.frame(frame_id) {
            if let Some(derivation) = &frame.derivation {
                self.push_lineage_first(&derivation.source_frame_id, done, visiting, order);
            }
            for lookup_id in frame.lookup_frame_ids() {
                self.push_lineage_first(&lookup_id, done, visiting, order);
            }
            // A frame read by formula is upstream too, so its snapshot has
            // to be rewritten before the snapshot of anything reading it.
            for foreign_id in frame.foreign_frames() {
                self.push_lineage_first(foreign_id, done, visiting, order);
            }
        }
        visiting.remove(frame_id);
        if done.insert(frame_id.to_string()) {
            order.push(frame_id.to_string());
        }
    }

    /// Every parquet file this document names: the imported bytes behind a
    /// frame, and the snapshot of a frame that is cached.
    ///
    /// Both are artifacts and both travel with the document, so anything
    /// that moves, copies, or rewrites a path has to touch both. Reaching
    /// for one and forgetting the other is how a copied document ends up
    /// reading its original's caches.
    pub(crate) fn artifacts(&self) -> impl Iterator<Item = &DataArtifact> {
        self.objects
            .iter()
            .filter_map(frame_of)
            .flat_map(|frame| {
                frame
                    .artifact
                    .iter()
                    .chain(frame.materialization.iter().map(|cache| &cache.artifact))
            })
            // A frozen answer is a file like any other, and one the sweeper
            // would otherwise decide nothing reads.
            .chain(self.frozen_values.values().map(|frozen| &frozen.artifact))
    }

    pub(crate) fn artifacts_mut(&mut self) -> impl Iterator<Item = &mut DataArtifact> {
        self.objects
            .iter_mut()
            .filter_map(frame_of_mut)
            .flat_map(|frame| {
                frame.artifact.iter_mut().chain(
                    frame
                        .materialization
                        .iter_mut()
                        .map(|cache| &mut cache.artifact),
                )
            })
            .chain(
                self.frozen_values
                    .values_mut()
                    .map(|frozen| &mut frozen.artifact),
            )
    }

    /// Records artifact paths relative to the document that names them.
    ///
    /// A document and its `.framework` sidecar are one thing on disk, and
    /// the whole point of writing paths this way is that they survive being
    /// moved together — `cp -r`, a synced folder, a different machine with a
    /// different home directory. An absolute path describes where the file
    /// was when it was written, which is a fact with a short shelf life.
    ///
    /// A path outside the document's directory stays absolute, because it is
    /// genuinely elsewhere: an import may legitimately point at a file the
    /// user keeps somewhere of their own, and rewriting that as a pile of
    /// `../..` would be a worse description of the same location.
    pub(crate) fn relativize_artifact_paths(&mut self, document_path: &Path) {
        let Some(directory) = document_directory(document_path) else {
            return;
        };
        for artifact in self.artifacts_mut() {
            if let Ok(relative) = Path::new(&artifact.path).strip_prefix(&directory) {
                artifact.path = relative.display().to_string();
            }
        }
    }

    /// Reads relative artifact paths back against the document's own
    /// directory, which is the only place they mean anything.
    pub(crate) fn resolve_artifact_paths(&mut self, document_path: &Path) {
        let Some(directory) = document_directory(document_path) else {
            return;
        };
        for artifact in self.artifacts_mut() {
            let path = Path::new(&artifact.path);
            if path.is_relative() {
                artifact.path = directory.join(path).display().to_string();
            }
        }
    }

    /// Points every recorded artifact path at the copy beside `path`.
    ///
    /// This is the fallback for documents whose paths are absolute — every
    /// document written before paths went relative, and any whose import
    /// points outside the sidecar. Such a path still names wherever the
    /// parquet used to live, so one that no longer resolves is retargeted
    /// into the sidecar by artifact id. A path that does resolve is left
    /// alone: pointing outside the sidecar is allowed.
    pub(crate) fn relink_artifacts(&mut self, path: &Path) {
        let Ok(paths) = CollaborationPaths::for_document(path, &self.id) else {
            return;
        };
        let data_directory = paths.root.join("data");
        if !data_directory.is_dir() {
            return;
        }
        for artifact in self.artifacts_mut() {
            if Path::new(&artifact.path).exists() {
                continue;
            }
            let beside = data_directory.join(format!("{}.parquet", artifact.id));
            if beside.is_file() {
                artifact.path = beside.display().to_string();
            }
        }
    }
}

/// An artifact with its location forgotten, for fingerprinting.
///
/// Where a parquet sits is not part of what a frame computes. Hashing the
/// path means moving a document, copying it with Save As, or opening it on
/// a machine with a different home directory marks every snapshot in it
/// stale — announcing that the numbers changed when nothing did but the
/// address. Identity is the artifact id: a different file gets a different
/// one, and that is what a fingerprint should notice.
/// A list as a gutter shows it: the first few, then how many there are.
///
/// Truncated rather than scrolled, because the gutter is one line high and
/// the useful facts about a long list are its front and its length.
fn render_list(series: &polars::prelude::Series, data_type: DataType) -> String {
    const SHOWN: usize = 6;
    let head: Vec<String> = (0..series.len().min(SHOWN))
        .map(|index| {
            crate::polars_value_at(series, index)
                .map(|value| crate::engine::values::format_scalar_value(&value, data_type))
                .unwrap_or_else(|_| "—".into())
        })
        .collect();
    if series.len() > SHOWN {
        format!("[{}, …] · {} values", head.join(", "), series.len())
    } else {
        format!("[{}]", head.join(", "))
    }
}

/// Reads a frozen answer back off its parquet: one column, one row.
///
/// Written as a frame rather than a number in the document so that the same
/// mechanism can hold a column or a small frame later without the document
/// format changing — and so a frozen answer is a data file like every other
/// recorded thing here, swept and relocated by the machinery that already
/// exists for those.
pub(crate) fn read_frozen_series(
    path: &str,
) -> Result<(DataType, polars::prelude::Series), String> {
    use polars::prelude as pl;
    let frame = pl::LazyFrame::scan_parquet(pl::PlRefPath::new(path), Default::default())
        .and_then(|scan| scan.collect())
        .map_err(|error| format!("The frozen answer could not be read: {error}"))?;
    let series = frame
        .get_column_names()
        .first()
        .and_then(|name| frame.column(name).ok())
        .ok_or_else(|| "The frozen answer is empty".to_string())?
        .as_materialized_series();
    if series.is_empty() {
        return Err("The frozen answer is empty".into());
    }
    let data_type = framework_type_from_polars(series.dtype()).unwrap_or(DataType::String);
    Ok((data_type, series.clone()))
}

/// The same answer where only one value can be meant — a result card, and
/// anything reading a frozen answer as a single literal.
pub(crate) fn read_frozen_answer(path: &str) -> Result<(DataType, ScalarValue), String> {
    let (data_type, series) = read_frozen_series(path)?;
    Ok((data_type, crate::polars_value_at(&series, 0)?))
}

fn placeless(artifact: Option<&DataArtifact>) -> Option<DataArtifact> {
    artifact.map(|artifact| DataArtifact {
        path: String::new(),
        ..artifact.clone()
    })
}

fn frame_of(object: &DataObject) -> Option<&FrameObject> {
    match object {
        DataObject::Frame(frame) => Some(frame),
        _ => None,
    }
}

fn frame_of_mut(object: &mut DataObject) -> Option<&mut FrameObject> {
    match object {
        DataObject::Frame(frame) => Some(frame),
        _ => None,
    }
}

/// The directory a document's relative paths are measured from.
///
/// `None` for a path with no directory part at all — a bare `notes.fw`
/// handed in as a relative path — where "relative to the document" and
/// "relative to the working directory" are the same thing and there is
/// nothing to rewrite.
fn document_directory(document_path: &Path) -> Option<PathBuf> {
    document_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
}
