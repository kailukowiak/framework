use crate::Id;
use crate::formula::ast::{Expr, Formula};
use crate::model::data_artifact::{ConnectorRecipe, DataArtifact};
use crate::model::derivation::{
    DerivedSort, FrameDerivation, FrameStep, Materialization, UniqueKeyConstraint,
};
use crate::model::value::{ColumnFormat, DataType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameObject {
    pub id: Id,
    pub name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// The wrangle chain: what this frame *is*. A frame derived from this
    /// one reads these steps, because they are part of its lineage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<FrameStep>,
    /// The display layer: how this frame is *shown*. Trailing filter and
    /// sort applied after the wrangle chain, on this frame's own reads
    /// only — a frame derived from this one never sees them.
    ///
    /// That one difference is the entire distinction between the Wrangle
    /// tab and the View tab, and it is enforced in exactly one place:
    /// recursive calls in [`Document::materialize_frame_lazy`] always ask
    /// for [`Layer::Data`].
    ///
    /// [`Layer::Data`]: crate::Layer::Data
    /// [`Document::materialize_frame_lazy`]: crate::Document::materialize_frame_lazy
    #[serde(default, skip_serializing_if = "FrameDisplay::is_empty")]
    pub display: FrameDisplay,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_columns: Vec<Column>,
    /// A remark pinned to the frame: what it is, for the next reader.
    /// Markdown, shown behind an icon rather than spending card space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub comment: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub source_file: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub artifact: Option<DataArtifact>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub connector: Option<ConnectorRecipe>,
    #[serde(default)]
    pub derivation: Option<FrameDerivation>,
    /// Rows spelled as a rule instead of written down or read from a file:
    /// `sequence(0, 16)`, or a date range whose bounds name a value on the
    /// canvas. The document evaluates the rule on every read, so editing the
    /// value regrows the frame — the whole point of writing rows this way.
    ///
    /// This is its own source kind, not a derivation, because a generator
    /// has no upstream frame: every lineage walk that follows
    /// `derivation.source_frame_id` correctly treats a generated frame as a
    /// root. The one column the rule fills is `columns[0]`; a wrangle chain
    /// in `steps` may build more on top of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub generator: Option<FrameGenerator>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub materialization: Option<Materialization>,
    #[serde(default)]
    pub unique_keys: Vec<UniqueKeyConstraint>,
    /// Hand-entered values living on a computed frame, keyed by row identity
    /// rather than row position.
    ///
    /// A derived frame's rows are outputs and cannot be typed into — but the
    /// *inputs a person owns* about those rows (hours against a generated
    /// timesheet line, a note against a scenario) need somewhere to live
    /// that survives the frame being regrown. Position dies on every
    /// refresh; the key columns' values do not. So each entry column stores
    /// its values against the row's key, and every read joins them back on.
    /// An entry whose key no longer matches any row simply waits, unjoined,
    /// for the row to come back.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entry_columns: Vec<EntryColumn>,
    #[serde(default)]
    pub summaries: Vec<Summary>,
}

impl FrameObject {
    /// Whether the values in this frame's cells are the frame's own.
    ///
    /// The one question behind every "can I type here": a frame that was
    /// typed in owns its rows and can be edited like a spreadsheet, while an
    /// imported or derived frame is a view of values that live somewhere
    /// else — the file, or the transformation above it. Editing those in
    /// place would either be overwritten on the next read or silently
    /// diverge from the source, which is why the edit is refused rather than
    /// accepted and quietly dropped.
    ///
    /// A cell *override* is the exception, and deliberately a different
    /// thing: it is a formula recorded against one cell, visible as an
    /// override, not a value pretending to have come from the source.
    pub fn owns_its_rows(&self) -> bool {
        self.derivation.is_none()
            && self.artifact.is_none()
            && self.source_file.is_none()
            && self.generator.is_none()
    }

    /// What this frame reads from, named the way a person would name it.
    ///
    /// The connected file wins over the imported one: when a frame has both,
    /// the connector is where the next refresh will read from, and that is
    /// the honest answer to "where does this come from". `None` for a frame
    /// that owns its rows or is derived — neither reads a file.
    pub fn source_name(&self) -> Option<String> {
        if let Some(connector) = &self.connector {
            return Some(connector.source_name());
        }
        if let Some(path) = &self.source_file {
            return Some(file_name_of(path));
        }
        self.artifact
            .as_ref()
            .map(|artifact| artifact.source_name.clone())
    }
}

/// The last component of a path, or the whole thing when it has no
/// components worth trimming.
fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// How a crosstab view spreads a long frame wide: which column's values
/// become column headers, and which column fills the cells. Everything
/// else visible groups the rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CrosstabDisplay {
    pub names_column_id: Id,
    pub values_column_id: Id,
}

/// One hand-entered column on a computed frame: which column it fills,
/// which columns identify a row, and the values entered so far.
///
/// `key_column_ids` is written down here rather than borrowed from
/// `unique_keys` so an edit to the frame's keys later cannot silently
/// re-address every entry; the column keeps meaning what it meant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EntryColumn {
    pub column_id: Id,
    pub key_column_ids: Vec<Id>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<EntryValue>,
}

/// One entered value: the key column raws that name its row, and the raw
/// text entered, both in the document's own writing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EntryValue {
    pub key: Vec<String>,
    pub raw: String,
}

/// The rule a generated frame's rows come from.
///
/// The expression is scalar-scoped — it may name values, results, and block
/// lines, never a column of any frame — and it must come back list-shaped or
/// as one value. `sequence(...)` is the expected spelling, but a written
/// list works the same way. Bounds that reference a value are what make a
/// generated calendar follow its anchor date around.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameGenerator {
    pub formula: Formula,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Column {
    pub id: Id,
    pub name: String,
    /// The physical field read from an imported artifact.
    ///
    /// This is deliberately separate from `name`: renaming a column in the
    /// model must not make the parquet or SQL result retroactively rename its
    /// field. `None` means the column is literal or produced by a formula.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub source_name: Option<String>,
    pub data_type: DataType,
    #[serde(default)]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub categories: Vec<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub format: Option<ColumnFormat>,
    pub formula: Option<Formula>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Row {
    pub id: Id,
    pub cells: BTreeMap<Id, Cell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Cell {
    pub raw: String,
    pub override_formula: Option<Formula>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CellUpdate {
    pub row_id: Id,
    pub column_id: Id,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Summary {
    pub id: Id,
    pub column_id: Id,
    pub operation: SummaryOperation,
    pub label: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SummaryOperation {
    Sum,
    Mean,
    Quartile25,
    Median,
    Quartile75,
    Min,
    Max,
    Count,
    Missing,
    CountDistinct,
    Mode,
}

impl SummaryOperation {
    /// The short word that fits in a frame's row-header gutter.
    ///
    /// A summary footer can hold several of these at once, so this is not a
    /// sentence or a field label. It is the row's identity, at the same
    /// density as `1`, `2`, `3` above it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sum => "Sum",
            Self::Mean => "Mean",
            Self::Quartile25 => "25%",
            Self::Median => "50%",
            Self::Quartile75 => "75%",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Count => "Count",
            Self::Missing => "Nulls",
            Self::CountDistinct => "Distinct",
            Self::Mode => "Mode",
        }
    }

    /// Whether a statistic has an honest meaning for a column type.
    ///
    /// The footer still draws the cell when this is false, as `n/a`. That is
    /// materially different from a blank: the row was asked for and the
    /// column was considered; the statistic simply does not apply.
    pub fn supports(self, data_type: DataType) -> bool {
        let numeric = matches!(
            data_type,
            DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage
        );
        match self {
            Self::Sum | Self::Mean | Self::Quartile25 | Self::Median | Self::Quartile75 => numeric,
            Self::Min | Self::Max => numeric || data_type == DataType::Date,
            Self::Count | Self::Missing | Self::CountDistinct => true,
            Self::Mode => matches!(data_type, DataType::String | DataType::Categorical),
        }
    }

    /// The type used to format the aggregate rather than the source cell.
    pub fn output_type(self, data_type: DataType) -> DataType {
        match self {
            Self::Count | Self::Missing | Self::CountDistinct => DataType::Integer,
            Self::Mean | Self::Quartile25 | Self::Median | Self::Quartile75
                if matches!(data_type, DataType::Integer | DataType::Number) =>
            {
                DataType::Number
            }
            _ => data_type,
        }
    }
}

/// Everything about a frame that is presentation rather than lineage.
///
/// `steps` is the same [`FrameStep`] the wrangle chain uses and runs through
/// the same evaluator, so a display filter and a wrangle filter are the same
/// code path with the same null ordering. Only [`FrameStep::Filter`] and
/// [`FrameStep::Sort`] may appear here, and only in that order — a display
/// layer that could add or drop columns would change the frame's schema
/// contract, which is a lineage change by definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameDisplay {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<FrameStep>,
    /// Whole-frame footer rows, in display order.
    ///
    /// These live with filters, sorting and orientation because they describe
    /// how this frame is being looked at, not another transformation in its
    /// lineage. The results are queried on demand: keeping a footer on a
    /// million-row import must not rescan it whenever an unrelated canvas
    /// object changes.
    /// `None` is an older document whose per-column summaries should be
    /// projected into footer rows. `Some([])` is a deliberate choice to show
    /// no rows. Keeping those states distinct means Clear stays cleared
    /// without making old workbooks lose their summaries on open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub summary_rows: Option<Vec<SummaryOperation>>,
    /// Whether the profile drawer below the records grid is expanded.
    #[serde(default, skip_serializing_if = "is_false")]
    pub summary_drawer_open: bool,
    /// The drawer's own height, independent of its card's outer height.
    /// Absence means the compact interface default rather than a second
    /// hard-coded value written into every document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub summary_drawer_height: Option<f64>,
    #[serde(default)]
    pub orientation: FrameViewOrientation,
    /// A long frame shown wide: one row per key, one column per value of
    /// the names column, cells from the values column. Display only — the
    /// data stays long, which is what makes this safe where a pivot step is
    /// not: a *view* has no schema for other frames to depend on, so it may
    /// follow the data freely. The editable crosstab over an entry column
    /// is the spreadsheet-shaped face of a keyed long frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub crosstab: Option<CrosstabDisplay>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<FrameStyle>,
    /// Ordered, row-wise presentation rules. Their predicates are formulas
    /// over this frame, but their answers never become data columns: they
    /// only contribute sparse style properties to the rendered view.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub style_rules: Vec<FrameStyleRule>,
}

impl FrameDisplay {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
            && self.summary_rows.is_none()
            && !self.summary_drawer_open
            && self.summary_drawer_height.is_none()
            && self.styles.is_empty()
            && self.style_rules.is_empty()
            && self.orientation == FrameViewOrientation::default()
            && self.crosstab.is_none()
    }

    /// The display filter, as `(predicates, match_all)`.
    pub fn filter(&self) -> Option<(&[Expr], bool)> {
        self.steps.iter().find_map(|step| match step {
            FrameStep::Filter {
                predicates,
                match_all,
            } => Some((predicates.as_slice(), *match_all)),
            _ => None,
        })
    }

    /// The display sort keys, empty when the frame is shown in plan order.
    pub fn sort(&self) -> &[DerivedSort] {
        self.steps
            .iter()
            .find_map(|step| match step {
                FrameStep::Sort { keys } => Some(keys.as_slice()),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Replaces the display filter, keeping the `[Filter, Sort]` order an
    /// empty predicate list drops the step entirely, so "no filter" is the
    /// absence of a step rather than a step that matches everything.
    pub fn set_filter(&mut self, predicates: Vec<Expr>, match_all: bool) {
        self.steps
            .retain(|step| !matches!(step, FrameStep::Filter { .. }));
        if !predicates.is_empty() {
            self.steps.insert(
                0,
                FrameStep::Filter {
                    predicates,
                    match_all,
                },
            );
        }
    }

    /// Whether the display filter reads `column_id`. Deleting a column out
    /// from under a filter that names it would leave the frame unreadable.
    pub fn references_column(&self, column_id: &str) -> bool {
        self.filter().is_some_and(|(predicates, _)| {
            predicates
                .iter()
                .any(|predicate| predicate.references_column(column_id))
        }) || self.style_rules.iter().any(|rule| {
            rule.column_id.as_deref() == Some(column_id)
                || rule.formula.expression.references_column(column_id)
        }) || self.crosstab.as_ref().is_some_and(|crosstab| {
            crosstab.names_column_id == column_id || crosstab.values_column_id == column_id
        })
    }

    /// Whether the display filter reads the value object `object_id`.
    pub fn references_object(&self, object_id: &str) -> bool {
        self.filter().is_some_and(|(predicates, _)| {
            predicates
                .iter()
                .any(|predicate| predicate.references_object(object_id))
        }) || self
            .style_rules
            .iter()
            .any(|rule| rule.formula.expression.references_object(object_id))
    }

    /// Replaces the display sort, which always runs after the filter.
    pub fn set_sort(&mut self, keys: Vec<DerivedSort>) {
        self.steps
            .retain(|step| !matches!(step, FrameStep::Sort { .. }));
        if !keys.is_empty() {
            self.steps.push(FrameStep::Sort { keys });
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameCellStyle {
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub underline: Option<bool>,
    #[serde(default)]
    pub text_color: Option<String>,
    #[serde(default)]
    pub fill_color: Option<String>,
    #[serde(default)]
    pub alignment: Option<FrameCellAlignment>,
    #[serde(default)]
    pub line_style: Option<FrameLineStyle>,
}

impl FrameCellStyle {
    pub fn is_empty(&self) -> bool {
        self.bold.is_none()
            && self.italic.is_none()
            && self.underline.is_none()
            && self.text_color.is_none()
            && self.fill_color.is_none()
            && self.alignment.is_none()
            && self.line_style.is_none()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FrameCellAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FrameLineStyle {
    Solid,
    Dashed,
    Dotted,
    Double,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum FrameStyleTarget {
    Frame,
    Column { column_id: Id },
    Row { row_id: Id },
    Cell { row_id: Id, column_id: Id },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameStyle {
    pub target: FrameStyleTarget,
    pub style: FrameCellStyle,
}

/// A conditional-formatting rule: one hidden column, and how its answer is
/// read as style.
///
/// The formula is an ordinary row-wise expression over the frame — the same
/// thing a calculated column holds — and what it *returns* decides how it is
/// read. A yes-or-no answer picks rows; a label sorts them into cases; a
/// number places them along a ramp. Nothing about the frame's data changes
/// either way: the column is computed, read as style, and dropped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameStyleRule {
    pub id: Id,
    pub formula: Formula,
    /// `None` styles every cell in a matching row. A column id confines the
    /// rule to that field while still allowing the formula to read any
    /// field in the row.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub column_id: Option<Id>,
    pub output: FrameStyleOutput,
}

/// How a rule's answer becomes style. The variant has to agree with what the
/// formula returns, which is checked when the rule is set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum FrameStyleOutput {
    /// True or false: the rows that answer true take one style.
    Condition { style: FrameCellStyle },
    /// A label per row: each listed value takes its own style, and anything
    /// unlisted takes `other` when there is one.
    Category {
        cases: Vec<FrameStyleCase>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        other: Option<FrameCellStyle>,
    },
    /// A number per row, read as a position between two colors.
    Scale { scale: FrameStyleScale },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameStyleCase {
    /// The value as it is written in the data. Compared as text, because
    /// that is what a category is.
    pub value: String,
    pub style: FrameCellStyle,
}

/// A color ramp over a number that is already a position on it.
///
/// The positions are fixed: low is 0, an optional middle is 0.5, high is 1,
/// and the formula says where each row sits between them. Text and fill each
/// choose independently whether they have that middle. That is the whole reason
/// the ramp carries no numbers of its own — a rule is a formula and a
/// reading of what it returns, and a scale that also held a domain was the
/// one reading that broke the rule by keeping data-dependent numbers beside
/// the formula instead of in it.
///
/// `.normalize()` is what a formula usually says here, and everything a
/// domain used to express is an edit to it: pinned ends are
/// `.normalize(0, 100)`, a diverging scale is `.normalize(center=0)`,
/// outliers are `.clip(...)` first, and a value substituted from another
/// column is a `when(...)` in front. None of those needed a control.
#[derive(Debug, Clone, Serialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameStyleScale {
    /// A text-colour ramp, when the rule paints text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub text: Option<FrameStyleColorScale>,
    /// A fill-colour ramp, when the rule paints the cell background.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub fill: Option<FrameStyleColorScale>,
}

/// The colors one property takes at a scale's fixed stops.
///
/// Text and fill keep separate ramps because the readable ink over a fill is
/// not the fill itself. A scale may carry either or both; making one enum say
/// which property owned a shared set of colors was what made choosing a fill
/// silently discard the text color somebody had already chosen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameStyleColorScale {
    /// The color at 0.
    pub low: String,
    /// The color at 1.
    pub high: String,
    /// A color at 0.5, for a ramp that turns rather than climbs.
    ///
    /// Optional because two colors is the common case and a third nobody
    /// asked for is a color they did not choose appearing in their data. It
    /// exists for the *diverging* ramp, where the interesting number is in
    /// the middle and the two directions away from it mean opposite things
    /// — which the formula places, with `.normalize(center=0)` or whatever
    /// else puts that number at a half.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub mid: Option<String>,
}

/// The shape saved before text and fill could coexist on one scale.
///
/// Kept public because old operations and documents are durable inputs. New
/// documents serialize [`FrameStyleScale::text`] and `fill`; deserialization
/// below promotes this property plus its colors into the corresponding ramp.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FrameStyleScaleProperty {
    Fill,
    Text,
}

impl<'de> Deserialize<'de> for FrameStyleScale {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", untagged)]
        enum StoredScale {
            Legacy {
                property: FrameStyleScaleProperty,
                low: String,
                high: String,
                #[serde(default)]
                mid: Option<String>,
            },
            Current {
                #[serde(default)]
                text: Option<FrameStyleColorScale>,
                #[serde(default)]
                fill: Option<FrameStyleColorScale>,
            },
        }

        Ok(match StoredScale::deserialize(deserializer)? {
            StoredScale::Legacy {
                property,
                low,
                high,
                mid,
            } => {
                let colors = Some(FrameStyleColorScale { low, high, mid });
                match property {
                    FrameStyleScaleProperty::Fill => Self {
                        text: None,
                        fill: colors,
                    },
                    FrameStyleScaleProperty::Text => Self {
                        text: colors,
                        fill: None,
                    },
                }
            }
            StoredScale::Current { text, fill } => Self { text, fill },
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FrameViewOrientation {
    #[default]
    RecordsAsRows,
    FieldsAsRows,
}
