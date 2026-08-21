use crate::Id;
use crate::formula::ast::Formula;
use crate::model::data_artifact::DataArtifact;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ValueObject {
    pub id: Id,
    pub name: String,
    pub raw: String,
    pub data_type: DataType,
}

/// A value the document works out rather than one somebody typed.
///
/// A value is static: a number, a date, a name, entered and then true until
/// somebody retypes it. A result is the other thing a single cell can be —
/// `= DownPayment / PurchasePrice` — and it holds the formula, not the
/// answer. The answer is computed when the document is read, from whatever
/// the formula's references hold *now*, which is what makes the card live
/// rather than a copy that quietly goes stale.
///
/// The formula is stored parsed, the way a column's is: references travel
/// by id, so renaming a value does not orphan the results that read it.
/// What the formula may name is the scalar scope — values, lists, other
/// results, and columns of materialized frames — and never the columns of
/// "the current frame", because a result does not sit in one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResultObject {
    pub id: Id,
    pub name: String,
    pub formula: Formula,
}

/// An answer computed from live data once and written down.
///
/// The third thing a formula can do about a frame that has no snapshot.
/// Materializing the frame records all 900,000 rows to get at one number,
/// which is the wrong trade when the number is all you wanted; refusing the
/// reference, which is what happened before this existed, makes the user
/// materialize anyway. Freezing records the answer instead — a few bytes of
/// parquet, refreshed on demand.
///
/// It also keeps the document acyclic for free, and by the same argument
/// snapshots do: a written-down answer is inert. A formula that reads one
/// reads a recorded fact, not a computation that might lead back round to
/// itself, so nothing here has to watch its own feet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrozenValue {
    pub artifact: DataArtifact,
    /// Everything that decided this answer — the formula, and the lineage of
    /// every frame it reads — as it stood when the answer was taken. When
    /// the two disagree the answer is stale, which is a thing to say on the
    /// card rather than a thing to fix behind someone's back.
    pub fingerprint: String,
    /// When it was taken, to say so on the card. Recorded rather than
    /// derived: a frozen answer's age is the whole reason to look at it.
    pub taken_at: String,
}

/// An ordered list of expression lines — the scratchpad.
///
/// The density mechanism for results: forty scratch calculations live on
/// one card instead of forty. A block is a namespace, not a new value kind
/// — each line is a result as specified above, and the block contributes
/// containment, ordering, and a qualified name. Lines within a block may
/// name each other bare; everything else reaches a line through the block,
/// `` `General calculations`.`account_balance` ``.
///
/// Order is meaning: a line may read only the lines above it, so the block
/// reads top to bottom as a worked calculation. Across blocks the ordinary
/// dependency graph and cycle check apply, the same as between results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlockObject {
    pub id: Id,
    pub name: String,
    pub lines: Vec<BlockLine>,
}

/// One line of a block: what was typed, and the parse of it if it parsed.
///
/// Every line gets a name at birth (`line_1`, `line_2`, …) so it is always
/// addressable; naming is just renaming, and renaming is free because
/// references travel by the line's id. The id is document-unique, which is
/// what lets a formula anywhere hold one the way it holds a result's.
///
/// The text is kept, and the parse is optional, which is the one place this
/// document model departs from parse-at-write. Everywhere else a formula
/// that does not parse is a refused edit, because everywhere else a formula
/// is committed deliberately. A scratchpad is half-typed by definition: it
/// is a draft surface, and refusing the keystroke that leaves `1 +` on the
/// screen would make it unusable. So a line that does not parse is stored
/// as it stands and reports its complaint in its own gutter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlockLine {
    pub id: Id,
    /// Empty on a blank or a comment line: those name nothing, and nothing
    /// names them.
    pub name: String,
    /// Whether the name was typed (`x = 10`) or handed to the line so that
    /// it could be referred to at all. Only a typed name is written back
    /// into the block's text — an automatic one would put words in the
    /// author's mouth.
    #[serde(default)]
    pub named: bool,
    /// Whether a typed name was written in formula-style backticks. The
    /// semantic name is stored without quoting; this bit only preserves the
    /// author's spelling when the editable block text is reconstructed.
    #[serde(default)]
    pub name_quoted: bool,
    /// The line as typed, with any `name =` taken off the front.
    #[serde(default)]
    pub source: String,
    /// The parse of `source`, when there is one.
    pub formula: Option<Formula>,
    /// Why there is not one, when there is not: the parser's complaint, or
    /// the reason a formula that parsed cannot be allowed to run — reading a
    /// line below itself, or closing a loop. Kept rather than worked out
    /// again at display time so the gutter shows the reason the edit found,
    /// not a second opinion about it.
    #[serde(default)]
    pub error: Option<String>,
}

impl BlockLine {
    /// The parsed expression, when the line has one.
    pub fn expression(&self) -> Option<&crate::formula::ast::Expr> {
        self.formula.as_ref().map(|formula| &formula.expression)
    }

    /// A line holding nothing but space. Kept rather than dropped: blank
    /// lines are how a worked calculation is grouped, and dropping them
    /// would fight the person typing.
    pub fn is_blank(&self) -> bool {
        self.source.trim().is_empty()
    }

    /// Prose, not arithmetic — where the provenance of a hand-entered
    /// number goes. Carries no name and no answer.
    pub fn is_comment(&self) -> bool {
        self.source.trim_start().starts_with('#')
    }

    /// A line that computes something, and so has a name and a gutter.
    pub fn is_expression(&self) -> bool {
        !self.is_blank() && !self.is_comment()
    }

    /// The line as it belongs in the block's text.
    pub fn text(&self) -> String {
        if self.named {
            let name = if self.name_quoted {
                crate::formula::ast::formula_name(&self.name)
            } else {
                self.name.clone()
            };
            format!("{name} = {}", self.source)
        } else {
            self.source.clone()
        }
    }
}

/// A named list, written down rather than computed.
///
/// The third thing a document can hold, after a value and a frame, and it
/// exists because of the shape of question a spreadsheet answers by
/// selecting a range: the allowed currencies, the regions that count as
/// domestic, the thresholds. Those are lists, they are not frames, and
/// making a frame of one is a way of saying it that nobody means.
///
/// Values are kept as raw text for the same reason a cell is — it is what
/// was typed, it diffs a line at a time, and one parser turns text into a
/// value everywhere in the document rather than each place inventing its
/// own.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SeriesObject {
    pub id: Id,
    pub name: String,
    pub data_type: DataType,
    pub values: Vec<String>,
}

/// A heading and the things kept under it.
///
/// The spreadsheet version is a merged cell saying "Important lists" with
/// the lists sitting beneath it: a grouping that is real to the person
/// reading it and invisible to everything else. This one is real to the
/// formulas too — `` `Finance`.`Interest rate` `` — so the arrangement
/// that makes a canvas legible is the same arrangement you can write down.
///
/// Values, lists, and containers may go in one; frames and plots may not.
/// A frame already has a card, a lineage, and somewhere it belongs, and
/// putting it inside something else would give "where does this live" two
/// answers. Nesting is allowed, and is checked for loops.
///
/// Membership is a list on the container rather than a parent on each
/// member, which is what makes the order the one you arranged and keeps
/// "what is in here" a single place to read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ContainerObject {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub member_ids: Vec<Id>,
}

/// A card of prose: markdown, with `{{…}}` holes where live values render.
///
/// `segments` is the stored truth. The literal pieces keep the author's
/// markdown byte for byte; each hole keeps a parsed scalar formula, which
/// references the things it reads by id — so a rename elsewhere in the
/// document changes what the hole *prints* without anyone editing this
/// card. The editable source is reconstructed from the segments on every
/// view, the same way a chain renders its formulas back to text.
///
/// `text` is the shape this object had before segments existed: one plain
/// string. It is read once, as a single literal, when `segments` is empty,
/// and cleared by the first real edit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextObject {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<TextSegment>,
}

impl TextObject {
    /// The card's content, whichever shape holds it: stored segments, or
    /// the legacy string read as one literal.
    pub fn effective_segments(&self) -> Vec<TextSegment> {
        if self.segments.is_empty() && !self.text.is_empty() {
            return vec![TextSegment::Literal {
                text: self.text.clone(),
            }];
        }
        self.segments.clone()
    }
}

/// One piece of a text card, in order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum TextSegment {
    /// Markdown, exactly as typed.
    Literal { text: String },
    /// A `{{…}}` hole: a scalar formula whose answer renders in its place.
    Formula { formula: Formula },
    /// A hole that could not be read — the text kept, with its complaint,
    /// the way a formula block keeps a line it could not parse.
    Broken { source: String, error: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DataType {
    String,
    Categorical,
    Integer,
    Number,
    Currency,
    Percentage,
    Boolean,
    Date,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
#[ts(export)]
pub enum ScalarValue {
    Null,
    Number(f64),
    String(String),
    Boolean(bool),
    Date(#[ts(type = "string")] NaiveDate),
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ColumnFormatStyle {
    #[default]
    Plain,
    Number,
    Currency,
    Accounting,
    Percent,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ColumnFormatScale {
    #[default]
    Units,
    Thousands,
    Millions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ColumnFormat {
    pub style: ColumnFormatStyle,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub decimals: Option<u8>,
    #[serde(default)]
    #[ts(optional, as = "Option<ColumnFormatScale>")]
    pub scale: ColumnFormatScale,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub negative_parens: Option<bool>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub zero_dash: Option<bool>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub currency_code: Option<String>,
}

pub fn normalized_column_format(mut format: ColumnFormat) -> ColumnFormat {
    format.currency_code = format
        .currency_code
        .map(|code| code.trim().to_uppercase())
        .filter(|code| !code.is_empty());
    format
}
