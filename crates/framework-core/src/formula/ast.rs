use crate::Id;
use crate::engine::values::normalize_name;
use crate::error::CoreError;
use crate::format_number;
use crate::formula::compile::format_polars_call;
use crate::formula::lexer::ReferenceName;
use crate::model::document::{DataObject, Document};
use crate::model::frame::{Column, FrameObject};
use crate::model::value::DataType;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The private one-row column that `previous()` compiles to while a
/// recurrence is being evaluated. It is not document state and can never be
/// named by ordinary formula syntax.
pub(crate) const PREVIOUS_RESULT_COLUMN_ID: &str = "__framework_previous_result";

pub(crate) struct RecurrenceParts<'a> {
    pub(crate) seed: &'a Expr,
    pub(crate) next: &'a Expr,
    pub(crate) restart_by: Vec<&'a Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FormulaArgument {
    /// The stable argument name used by both positional and keyword calls.
    /// Keeping it separate from the signature lets the UI mark the argument
    /// under the cursor without trying to parse a Polars signature itself.
    pub name: String,
    /// Optional arguments are still shown in the signature, but nobody needs
    /// to supply them to make the call valid.
    pub required: bool,
    /// Concise, input-oriented guidance. This comes from the same catalog as
    /// completion, so an argument cannot quietly acquire a second UI-only
    /// meaning.
    pub description: String,
    /// A short expression in the formula language, where one is useful.
    pub example: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FormulaFunction {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub category: String,
    pub signature: String,
    pub description: String,
    pub minimum_arguments: usize,
    pub maximum_arguments: usize,
    pub return_type: String,
    pub null_behavior: String,
    /// Argument-level guidance for contextual formula help. The catalog owns
    /// this rather than the frontend so catalog consumers and completion
    /// teach the exact call shape the compiler accepts.
    pub arguments: Vec<FormulaArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Formula {
    /// Opaque on the TypeScript side, deliberately, and this is the one place
    /// that decision is written down.
    ///
    /// [`Expr`] is a wide recursive enum that exists to be evaluated, and the
    /// interface never evaluates one: it reads formulas as *text*, rendered
    /// back by the core (`ComputedFrame::formulas`, `RenderedFrameStep`), and
    /// writes them back as text for the core to parse. Nothing in `src/` has
    /// ever looked inside an expression tree, so generating a faithful mirror
    /// of it would publish several hundred lines of union that no caller can
    /// use — and would break the frontend build every time a variant is added
    /// to a parser the frontend does not care about.
    ///
    /// `unknown` is therefore the honest type, not a shortcut: it says the
    /// value round-trips through the frontend untouched, and it is what the
    /// hand-written mirror said before this file generated it.
    #[ts(type = "unknown")]
    pub expression: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Expr {
    Integer {
        value: i64,
    },
    Number {
        value: f64,
    },
    String {
        value: String,
    },
    Boolean {
        value: bool,
    },
    /// A rate written the way rates are written: `4.25%`.
    ///
    /// `value` is the number, already divided — `4.25%` is `0.0425` — so
    /// everything downstream does arithmetic on the number and never on the
    /// notation. What the variant adds over [`Expr::Number`] is the fact
    /// that it *was* written as a percentage, which is what lets the answer
    /// come back as `4.25%` instead of as `0.0425`.
    ///
    /// That fact is a type, [`DataType::Percentage`], not a formatting flag:
    /// money and percentages are numbers with a way of being written, and
    /// this document has said so since before there was a formula language.
    Percentage {
        value: f64,
    },
    /// An amount of money, written the way amounts are: `$250000`.
    ///
    /// The sibling of [`Expr::Percentage`], and the same idea: `value` is
    /// the plain number, and the variant carries the fact that it was
    /// written with a mark on it. Money differs in being a *dimension*
    /// rather than a way of writing a ratio, which is what decides where it
    /// survives arithmetic — see [`arithmetic_type`].
    Money {
        value: f64,
    },
    /// A calendar date, written the way people write dates: `2026-08-12`.
    ///
    /// A literal rather than a call because `YYYY-MM-DD` is what anyone
    /// filtering a date column types first, and it is unambiguous enough to
    /// read as one thing — see the lexer for why that is safe.
    Date {
        value: NaiveDate,
    },
    /// A span of time: `30d`, `6h`, `1mo`, `2y`.
    ///
    /// Held as the Polars duration string it will be handed to, because the
    /// calendar units are the whole point. `1mo` has to stay one *month* —
    /// a length that depends on which month — rather than being flattened
    /// into a fixed number of days on the way in.
    ///
    /// A duration is not a value on its own; it only means something added
    /// to or subtracted from a date. [`Expr::Binary`] is where that happens.
    Duration {
        value: String,
    },
    Null,
    Column {
        column_id: Id,
    },
    /// A column in another frame: `` `Monthly totals`.`Revenue` ``.
    ///
    /// A frame formula may only name a foreign frame holding a snapshot: the
    /// file ends lineage and cannot lead back into the plan doing the read.
    /// Scratchwork may name a live or derived frame because it evaluates as a
    /// top-level semantic query rather than being spliced into another frame.
    ///
    /// How many rows that frame has decides what the reference *is*. One row
    /// and it is a value, which broadcasts down the column it lands in. More
    /// than one and it is a list, and a list has to be handed to a function
    /// that takes one — see [`Expr::validate_list_placement`], which is the
    /// same rule written-out lists have always followed. The alternative is
    /// worse than it sounds: Polars will quietly zip two unrelated frames
    /// together whenever their row counts happen to agree, matching by
    /// position, on no key at all.
    ForeignColumn {
        frame_id: Id,
        column_id: Id,
    },
    Value {
        object_id: Id,
    },
    /// A named list on the canvas: `` `Allowed currencies` ``.
    ///
    /// Always a list, however many values are in it, because that is what it
    /// was declared as — unlike [`Expr::ForeignColumn`], whose shape is a
    /// fact about somebody else's frame rather than something written down
    /// here. A list of one is a list; a single value is a value, and the
    /// canvas has one of those already.
    Series {
        object_id: Id,
    },
    List {
        items: Vec<Expr>,
    },
    Negate {
        expression: Box<Expr>,
    },
    Not {
        expression: Box<Expr>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    PolarsCall {
        name: String,
        arguments: Vec<Expr>,
        #[serde(default)]
        keyword_arguments: Vec<(String, Expr)>,
    },
    Method {
        input: Box<Expr>,
        path: Vec<String>,
        arguments: Vec<Expr>,
        #[serde(default)]
        keyword_arguments: Vec<(String, Expr)>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    FloorDivide,
    Modulo,
    Power,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

/// What an expression evaluates to, roughly — see [`Expr::shape`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// One value, which broadcasts against anything.
    Scalar,
    /// A list written down or declared on the canvas.
    List,
    /// A column of a frame, whose rows belong to that frame's row identity
    /// and cannot be lined up against anything else's by position.
    Column,
}

/// The methods that take a whole list and answer with one value. The set a
/// list may be handed to directly, alongside the functions that take one.
const REDUCING_METHODS: &[&str] = &[
    "sum",
    "mean",
    "median",
    "quantile",
    "min",
    "max",
    "count",
    "len",
    "null_count",
    "n_unique",
    "std",
    "var",
    "product",
    "first",
    "last",
];

/// The type two values have to have in common to sit in one list, or
/// `None` when they have none worth having.
///
/// A promotion tree rather than a coercion: the join of two types is the
/// least-committed type that still says the truth about both. Numbers,
/// money and percentages are all one number underneath and differ only in
/// how they are written, so a list holding more than one of them is
/// numbers — the shared fact — and the writing is dropped rather than
/// guessed at. Text and a category are both text, and go the same way.
///
/// Across those families there is no join, and this answers `None` so the
/// list can be refused. Making text the top of the tree would be the
/// tidier lattice and the wrong answer: it would swallow `[1, $2]`, two
/// numbers, into `["1", "$2"]`, which is the quiet wrongness this model
/// exists to refuse rather than a convenience.
pub(crate) fn promote_types(left: DataType, right: DataType) -> Option<DataType> {
    use DataType::*;
    if left == right {
        return Some(left);
    }
    match (left, right) {
        (Integer | Number | Currency | Percentage, Integer | Number | Currency | Percentage) => {
            Some(Number)
        }
        (String | Categorical, String | Categorical) => Some(String),
        _ => None,
    }
}

/// How `.show("…")` is spelled, and what it means.
///
/// Presentation, not storage — which is exactly why it is not one of
/// `cast`'s targets. `cast` converts: it changes what the value *is*, and
/// asking it for money would suggest it could put a dollar sign on a piece
/// of text. This changes nothing about the value and everything about how
/// it is written down, and the number underneath is the same number.
///
/// `"plain"` rather than `"number"` for the same reason: it says *stop
/// writing this as anything*, which is the thing somebody reaches for when
/// a chain of arithmetic has carried a dollar sign somewhere it was not
/// wanted.
///
/// How a value is written down, given what it computed to and what this
/// document knows about where it came from.
///
/// The two disagree in exactly one direction. Polars answers with what it
/// stores — Float64 for a price and Float64 for a rate — and a dollar sign
/// or a percent sign is a fact about the document, carried by
/// [`Expr::declared_type`]. So a number is written the way the document says
/// it is written, and anything else keeps what it computed to: a sum that
/// comes back as text or a date is not money because its input was.
pub(crate) fn written_type(found: DataType, declared: Option<DataType>) -> DataType {
    match (found, declared) {
        (
            DataType::Number | DataType::Integer,
            Some(declared @ (DataType::Currency | DataType::Percentage)),
        ) => declared,
        _ => found,
    }
}

pub(crate) fn shown_as(name: &str) -> Option<DataType> {
    match name {
        "money" | "currency" | "dollars" => Some(DataType::Currency),
        "percent" | "percentage" => Some(DataType::Percentage),
        "plain" | "number" => Some(DataType::Number),
        _ => None,
    }
}

/// What arithmetic on two written numbers is written as.
///
/// A separate question from [`promote_types`], which asks what type could
/// hold *both* of two values — the right question for a list and the wrong
/// one here. `$100 * 5%` is not "the type that holds money and a rate". It
/// is five dollars.
///
/// The rule that settles every case: **money is a dimension and a
/// percentage is not.** A percentage is a way of writing a pure ratio, so
/// multiplying by one consumes it and leaves whatever it was applied to;
/// dividing money by money cancels the dimension and leaves a ratio, which
/// is a percentage and is the one line this whole idea exists for —
/// `margin / revenue` reads `38%` where a spreadsheet says `0.38` and makes
/// you go and find the button.
///
/// Two places the dimensions do not decide it, because both sides are
/// dimensionless and only the writing is in question:
///
/// - `rate * 12` is a **number**, not a rate. Dimensionally either would do,
///   so the tie goes to the reading that cannot embarrass itself: the
///   commonest percentage line anybody writes is applying a rate to an
///   amount, and `20000 * 4.25%` announcing `85000%` is far worse than
///   `4.25% * 12` answering `0.51`. `.show("percent")` is there for the
///   second one.
/// - `rate / 12` **is** a rate, because sharing a rate out over twelve
///   months leaves a rate — an annual figure over the months of the year is
///   the line being served, and it is the same tie broken the other way for
///   the same reason: nothing about it can come out looking absurd.
///
/// `None` means this says nothing, which is what everything involving a
/// date, a duration, or text answers. Guessing there would put this
/// document's writing on a value it does not own.
fn arithmetic_type(operator: BinaryOperator, left: DataType, right: DataType) -> Option<DataType> {
    use BinaryOperator::*;
    use DataType::*;
    // Only numbers carry a way of being written. Anything else here is a
    // date, a span, or text, and what those do under arithmetic is not a
    // question about notation.
    if !matches!(left, Integer | Number | Currency | Percentage)
        || !matches!(right, Integer | Number | Currency | Percentage)
    {
        return None;
    }
    Some(match (operator, left, right) {
        // Money and money is money; money and a plain number or a rate is
        // still money, because the dimension has nowhere to go.
        (Add | Subtract, Currency, _) | (Add | Subtract, _, Currency) => Currency,
        (Add | Subtract, Percentage, Percentage) => Percentage,
        (Add | Subtract, Integer, Integer) => Integer,
        (Add | Subtract, _, _) => Number,

        // Money squared is not money, so two of them cancel to a number.
        (Multiply, Currency, Currency) => Number,
        (Multiply, Currency, _) | (Multiply, _, Currency) => Currency,
        (Multiply, Percentage, Percentage) => Percentage,
        (Multiply, Integer, Integer) => Integer,
        (Multiply, _, _) => Number,

        // The one that earns the feature.
        (Divide, Currency, Currency) => Percentage,
        (Divide, Currency, _) => Currency,
        (Divide, Percentage, Integer | Number) => Percentage,
        (Divide, _, _) => Number,

        // A remainder and a floor division keep the left side's dimension —
        // `$100 // 3` is thirty-three dollars — and lose it against another
        // dimension, because `$100 // $30` is a count of times.
        (FloorDivide | Modulo, Currency, Currency) => Number,
        (FloorDivide | Modulo, Currency, _) => Currency,
        (FloorDivide | Modulo, Integer, Integer) => Integer,
        (FloorDivide | Modulo, _, _) => Number,

        // A power leaves every dimension behind: `$2 ** 3` is eight of
        // something that is not dollars.
        (Power, _, _) => Number,

        // The comparisons never reach here — see the caller.
        _ => return None,
    })
}

/// What people call a type when they are being told two of them disagree.
pub(crate) fn type_name(data_type: DataType) -> &'static str {
    match data_type {
        DataType::String => "text",
        DataType::Categorical => "a category",
        DataType::Integer => "an integer",
        DataType::Number => "a number",
        DataType::Currency => "money",
        DataType::Percentage => "a percentage",
        DataType::Boolean => "a true/false value",
        DataType::Date => "a date",
    }
}

impl Expr {
    pub(crate) fn is_explicit_null(&self) -> bool {
        matches!(self, Expr::Null)
    }

    /// What this expression's values are, as far as the expression itself
    /// says so.
    ///
    /// `None` means "cannot be told from here" rather than "has no type" —
    /// what a Polars method hands back is a fact about Polars, not about
    /// anything written down in this document. The only thing this decides
    /// is whether a written list holds values that belong together, so the
    /// answer that refuses nothing is the safe one to give when unsure, in
    /// the same way [`Expr::shape`] treats what it cannot work out.
    pub(crate) fn declared_type(&self, document: &Document) -> Option<DataType> {
        self.declared_type_among(document, &[])
    }

    /// The same, with a scratch schema consulted before the document's own.
    ///
    /// A chain of steps produces columns that exist nowhere yet — the third
    /// step reads what the second one made, and the document has never heard
    /// of it. Without this, money survives one step and is plain by the next,
    /// which is worse than losing it outright because it looks deliberate.
    pub(crate) fn declared_type_among(
        &self,
        document: &Document,
        scope: &[Column],
    ) -> Option<DataType> {
        match self {
            Expr::Integer { .. } => Some(DataType::Integer),
            Expr::Number { .. } => Some(DataType::Number),
            Expr::Percentage { .. } => Some(DataType::Percentage),
            Expr::Money { .. } => Some(DataType::Currency),
            Expr::String { .. } => Some(DataType::String),
            Expr::Boolean { .. } => Some(DataType::Boolean),
            Expr::Date { .. } => Some(DataType::Date),
            // A gap sits in a list of anything, so it brings no type of its
            // own for the others to disagree with. A duration is not a
            // value at all — see [`Expr::Duration`].
            Expr::Null | Expr::Duration { .. } => None,
            Expr::Column { column_id } | Expr::ForeignColumn { column_id, .. } => scope
                .iter()
                .find(|column| column.id == *column_id)
                .map(|column| column.data_type)
                .or_else(|| document.column_type(column_id)),
            Expr::Series { object_id } => document.objects.iter().find_map(|object| match object {
                DataObject::Series(series) if series.id == *object_id => Some(series.data_type),
                _ => None,
            }),
            // A value says what it is; a result and a block line are their
            // formulas, and say whatever those work out to.
            Expr::Value { object_id } => match document.object(object_id) {
                Ok(DataObject::Value(value)) => Some(value.data_type),
                Ok(DataObject::Result(result)) => result
                    .formula
                    .expression
                    .declared_type_among(document, scope),
                _ => document
                    .block_line(object_id)
                    .and_then(|(block, index)| block.lines[index].expression().cloned())
                    .and_then(|expression| expression.declared_type_among(document, scope)),
            },
            Expr::List { items } => items
                .iter()
                .filter_map(|item| item.declared_type_among(document, scope))
                .try_fold(None, |carried: Option<DataType>, next| match carried {
                    None => Some(Some(next)),
                    Some(carried) => promote_types(carried, next).map(Some),
                })
                .flatten(),
            Expr::Negate { expression } => expression.declared_type_among(document, scope),
            Expr::Not { .. } => Some(DataType::Boolean),
            Expr::Binary {
                operator,
                left,
                right,
            } => match operator {
                BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual
                | BinaryOperator::Greater
                | BinaryOperator::GreaterEqual
                | BinaryOperator::And
                | BinaryOperator::Or => Some(DataType::Boolean),
                _ => arithmetic_type(
                    *operator,
                    left.declared_type_among(document, scope)?,
                    right.declared_type_among(document, scope)?,
                ),
            },
            // A fold answers in the same kind its values were — the sum of
            // money is money, the earliest of some dates is a date — except
            // for the ones that answer with a tally, which are numbers
            // whatever they counted. Everything else stays unknown: what a
            // Polars method hands back is a fact about Polars, and guessing
            // at it would put this document's writing on a value it does
            // not own.
            Expr::Method {
                input,
                path,
                arguments,
                ..
            } => match path.as_slice() {
                // The override, and the only thing in the language whose
                // whole job is to answer this question. Said out loud, it
                // beats whatever the arithmetic worked out — and it is
                // where the chain starts again, because everything above
                // reads this node rather than the one under it.
                [name] if name == "show" => match arguments.as_slice() {
                    [Expr::String { value }] => shown_as(value),
                    _ => None,
                },
                [name] if matches!(name.as_str(), "count" | "len" | "null_count" | "n_unique") => {
                    Some(DataType::Integer)
                }
                [name] if matches!(name.as_str(), "mean" | "median" | "quantile") => input
                    .declared_type_among(document, scope)
                    .map(|data_type| match data_type {
                        DataType::Integer | DataType::Number => DataType::Number,
                        other => other,
                    }),
                [name] if name == "mode" => input.declared_type_among(document, scope),
                // A fraction of the way along a range, whatever went in.
                // Money normalized is not money -- said here so notation
                // stops at this node rather than riding out on a number
                // that no longer means dollars.
                [name] if name == "normalize" => Some(DataType::Number),
                [namespace, name] if namespace == "str" && name == "to_date" => {
                    Some(DataType::Date)
                }
                // Date in, date out — not a guess about Polars but the
                // meaning of the operation: moving or snapping a date
                // cannot answer with anything else, and a date has no
                // notation variants for this to get wrong. Known here so
                // `today().dt.month_start() + 1` can read the `1` as days.
                [namespace, name]
                    if namespace == "dt"
                        && matches!(
                            name.as_str(),
                            "date" | "month_start" | "month_end" | "offset_by"
                        ) =>
                {
                    Some(DataType::Date)
                }
                // The calendar parts are counts by the same definitional
                // argument as `count` and `len` above. Declaring them is
                // load-bearing, not cosmetic: `offset_by` reads an
                // integer-typed argument as a day count, and an expression
                // built from `.dt.day()` has to *say* it is an integer for
                // that reading to reach it — left unknown, the raw number
                // once flowed into a string slot and took a frame down
                // with it.
                [namespace, name]
                    if namespace == "dt"
                        && matches!(
                            name.as_str(),
                            "year"
                                | "iso_year"
                                | "quarter"
                                | "month"
                                | "week"
                                | "weekday"
                                | "day"
                                | "ordinal_day"
                                | "days_in_month"
                        ) =>
                {
                    Some(DataType::Integer)
                }
                [name] if REDUCING_METHODS.contains(&name.as_str()) => {
                    input.declared_type_among(document, scope)
                }
                _ => None,
            },
            Expr::PolarsCall {
                name, arguments, ..
            } if name == "recur" => arguments
                .first()
                .and_then(|seed| seed.declared_type_among(document, scope)),
            // The clock functions are dates by definition of this language,
            // not by inspection of Polars.
            Expr::PolarsCall { name, .. } if name == "today" => Some(DataType::Date),
            _ => None,
        }
    }

    /// Refuses comparisons whose two written types cannot describe the same
    /// value. Polars can build a lazy plan for `text == number`, so schema
    /// collection alone says the filter is Boolean and lets the bad step be
    /// saved; the mismatch appears only when rows are read, at which point a
    /// perfectly good frame looks empty. This check belongs to the formula
    /// boundary instead: it preserves the last valid chain and points at the
    /// expression that needs a cast.
    pub(crate) fn validate_comparison_types_among(
        &self,
        document: &Document,
        scope: &[Column],
    ) -> Result<(), CoreError> {
        let mut mismatch = None;
        self.walk(&mut |expression| {
            if mismatch.is_some() {
                return;
            }
            let Expr::Binary {
                operator,
                left,
                right,
            } = expression
            else {
                return;
            };
            if !matches!(
                operator,
                BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::Less
                    | BinaryOperator::LessEqual
                    | BinaryOperator::Greater
                    | BinaryOperator::GreaterEqual
            ) {
                return;
            }
            let (Some(left_type), Some(right_type)) = (
                left.declared_type_among(document, scope),
                right.declared_type_among(document, scope),
            ) else {
                return;
            };
            if promote_types(left_type, right_type).is_none() {
                mismatch = Some(format!(
                    "Cannot compare {} with {}. Convert one side with .cast(…) first.",
                    type_name(left_type),
                    type_name(right_type)
                ));
            }
        });
        mismatch.map_or(Ok(()), |message| Err(CoreError::Formula(message)))
    }

    /// Refuses a written list whose values do not belong in one list.
    ///
    /// A list is one type, the way a column is one type — that is what
    /// makes `.sum()` on it mean anything, and what lets the values be
    /// written out with one set of rules rather than each on its own terms.
    /// Polars would rather settle a disagreement than report one, and
    /// settles it by making everything text: `[1, "a"]` comes back as
    /// `["1.0", "a"]`, a number rewritten by a formatter this document does
    /// not own, from a line nobody would have written on purpose. Saying so
    /// where it was written is worth more than an answer.
    fn validate_list_types(&self, document: &Document) -> Result<(), CoreError> {
        let Expr::List { items } = self else {
            return Ok(());
        };
        let mut carried: Option<DataType> = None;
        for item in items {
            let Some(next) = item.declared_type(document) else {
                continue;
            };
            let Some(previous) = carried else {
                carried = Some(next);
                continue;
            };
            carried = Some(promote_types(previous, next).ok_or_else(|| {
                CoreError::Formula(format!(
                    "A list holds one kind of value, and this one holds both {} and {}. \
                     Write them as two lists, or make them all one kind.",
                    type_name(previous),
                    type_name(next)
                ))
            })?);
        }
        Ok(())
    }

    fn validate_call_list_placement(
        &self,
        document: &Document,
        list_allowed: bool,
        name: &str,
        arguments: &[Expr],
        keyword_arguments: &[(String, Expr)],
    ) -> Result<(), CoreError> {
        if name == "recur" {
            for argument in arguments {
                argument.validate_list_placement(document, false)?;
            }
            for (_, argument) in keyword_arguments {
                argument.validate_list_placement(document, true)?;
            }
            return Ok(());
        }
        if name == "sequence" && !list_allowed && !self.uses_frame_length() {
            return Err(CoreError::Formula(
                "sequence makes a list, so it belongs in Scratchwork; use an ordered \
                 transformation when filling a frame column"
                    .into(),
            ));
        }
        for argument in arguments {
            argument.validate_list_placement(document, true)?;
        }
        for (_, argument) in keyword_arguments {
            argument.validate_list_placement(document, false)?;
        }
        Ok(())
    }

    pub(crate) fn validate_list_placement(
        &self,
        document: &Document,
        list_allowed: bool,
    ) -> Result<(), CoreError> {
        match self {
            Expr::List { items } => {
                if !list_allowed {
                    return Err(CoreError::Formula(
                        "An expression list must be passed to a function".into(),
                    ));
                }
                self.validate_list_types(document)?;
                for item in items {
                    item.validate_list_placement(document, false)?;
                }
            }
            Expr::Series { object_id } if !list_allowed => {
                let name = document
                    .objects
                    .iter()
                    .find(|object| object.id() == object_id)
                    .map(|object| object.name())
                    .unwrap_or(object_id.as_str());
                return Err(CoreError::Formula(format!(
                    "‘{name}’ is a list, so it has to be passed to something that \
                     takes one — is_in, for instance."
                )));
            }
            // A column of another frame is a list of everything in it, and
            // falls under the same rule — with its own wording, because the
            // fix is a different one. Nothing about `Prices.Amount` looks
            // like a list at the point of writing it; what makes it one is
            // how many rows that frame happens to have, which is worth
            // saying out loud alongside the refusal.
            Expr::ForeignColumn {
                frame_id,
                column_id,
            } if !list_allowed => {
                let rows = document.snapshot_row_count(frame_id).unwrap_or(1);
                if rows != 1 {
                    let (frame, column) = document.foreign_names(frame_id, column_id);
                    return Err(CoreError::Formula(format!(
                        "‘{frame}’ has {rows} rows, so ‘{column}’ is a list of {rows} \
                         values rather than one value. Summarize it down to a single \
                         row first, or pass it to something that takes a list."
                    )));
                }
            }
            Expr::Negate { expression } | Expr::Not { expression } => {
                expression.validate_list_placement(document, list_allowed)?
            }
            Expr::Binary { left, right, .. } => {
                // One pairing is refused, and only one: a list beside a
                // column. That aligns two independent objects by ordinal,
                // and Polars will do it silently whenever their lengths
                // happen to agree — a wrong answer with no error attached,
                // which is the single failure the row-identity model exists
                // to prevent.
                //
                // Everything else is allowed to be attempted. A scalar
                // beside a list broadcasts; two lists of the same length
                // pair up; two of different lengths fail, loudly, and the
                // line says so. Refusing arithmetic in advance because it
                // *might* fail is a worse trade than letting it fail where
                // it can be read and fixed.
                let shapes = (left.shape(document), right.shape(document));
                if matches!(
                    shapes,
                    (Shape::List, Shape::Column) | (Shape::Column, Shape::List)
                ) {
                    return Err(CoreError::Formula(
                        "A list and a frame column cannot be paired up value by value: \
                         they have no key in common, so this would match them by position \
                         and quietly mean nothing. Aggregate one of them, join them, or \
                         pass the list to something that takes one — is_in, for instance."
                            .into(),
                    ));
                }
                left.validate_list_placement(document, list_allowed)?;
                right.validate_list_placement(document, list_allowed)?;
            }
            Expr::PolarsCall {
                name,
                arguments,
                keyword_arguments,
            } => {
                self.validate_call_list_placement(
                    document,
                    list_allowed,
                    name,
                    arguments,
                    keyword_arguments,
                )?;
            }
            Expr::Method {
                input,
                path,
                arguments,
                keyword_arguments,
            } => {
                // A method that folds a whole column of values down to one
                // is a place a list belongs wherever it is written —
                // `` `Prices`.`Amount`.sum() `` is the very reason to name a
                // list at all, and it belongs in a column formula as much as
                // on a scratchpad line.
                //
                // Anything else keeps whatever permission the surrounding
                // formula had. A list handed an element-wise method is one
                // object having something done to it, not two objects being
                // lined up, so there is no positional zip here to prevent —
                // which is why arithmetic on a list has always been allowed
                // and `[1, 2, 3].round(0)` was refused only by oversight. In
                // a column formula the ambient permission is still `false`,
                // so a foreign column of many rows cannot sneak into a
                // column of another frame by wearing a method.
                let reduces = matches!(
                    path.as_slice(),
                    [name] if REDUCING_METHODS.contains(&name.as_str())
                );
                let filters = matches!(path.as_slice(), [name] if name == "filter");
                if filters && !list_allowed {
                    return Err(CoreError::Formula(
                        ".filter selects a shorter set of rows, so finish it with an aggregate such as .sum() or write it in Scratchwork."
                            .into(),
                    ));
                }
                input.validate_list_placement(document, list_allowed || reduces)?;
                for argument in arguments {
                    // Filter predicates have to remain row-aligned with the
                    // receiver. A written/canvas/foreign list has no row key
                    // to align by, unlike the current frame's Boolean column.
                    argument.validate_list_placement(document, !filters)?;
                }
                for (_, argument) in keyword_arguments {
                    argument.validate_list_placement(document, false)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn column_dependencies<'a>(&'a self, output: &mut Vec<&'a str>) {
        match self {
            Expr::Column { column_id } => output.push(column_id),
            Expr::List { items } => {
                for item in items {
                    item.column_dependencies(output);
                }
            }
            Expr::Negate { expression } | Expr::Not { expression } => {
                expression.column_dependencies(output)
            }
            Expr::Binary { left, right, .. } => {
                left.column_dependencies(output);
                right.column_dependencies(output);
            }
            Expr::PolarsCall {
                arguments,
                keyword_arguments,
                ..
            } => {
                for argument in arguments {
                    argument.column_dependencies(output);
                }
                for (_, argument) in keyword_arguments {
                    argument.column_dependencies(output);
                }
            }
            Expr::Method {
                input,
                arguments,
                keyword_arguments,
                ..
            } => {
                input.column_dependencies(output);
                for argument in arguments {
                    argument.column_dependencies(output);
                }
                for (_, argument) in keyword_arguments {
                    argument.column_dependencies(output);
                }
            }
            _ => {}
        }
    }

    /// Every other frame this expression reads, by id.
    ///
    /// A lineage edge like any other, and the reason it needs collecting
    /// separately: it does not come from a derivation, so nothing that walks
    /// derivations would ever find it. Staleness, liveness, and the cache
    /// key all travel this edge.
    pub(crate) fn foreign_frames<'a>(&'a self, output: &mut Vec<&'a str>) {
        self.walk(&mut |expression| {
            if let Expr::ForeignColumn { frame_id, .. } = expression {
                output.push(frame_id);
            }
        });
    }

    /// Every canvas object this expression reads by id — a value, a result,
    /// or a block line, all of which arrive as [`Expr::Value`].
    pub(crate) fn walk_values<'a>(&'a self, visit: &mut impl FnMut(&'a str)) {
        self.walk(&mut |expression| {
            if let Expr::Value { object_id } = expression {
                visit(object_id);
            }
        });
    }

    /// Calls `visit` on this expression and everything inside it.
    pub(crate) fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a Expr)) {
        visit(self);
        match self {
            Expr::List { items } => {
                for item in items {
                    item.walk(visit);
                }
            }
            Expr::Negate { expression } | Expr::Not { expression } => expression.walk(visit),
            Expr::Binary { left, right, .. } => {
                left.walk(visit);
                right.walk(visit);
            }
            Expr::PolarsCall {
                arguments,
                keyword_arguments,
                ..
            } => {
                for argument in arguments {
                    argument.walk(visit);
                }
                for (_, argument) in keyword_arguments {
                    argument.walk(visit);
                }
            }
            Expr::Method {
                input,
                arguments,
                keyword_arguments,
                ..
            } => {
                input.walk(visit);
                for argument in arguments {
                    argument.walk(visit);
                }
                for (_, argument) in keyword_arguments {
                    argument.walk(visit);
                }
            }
            _ => {}
        }
    }

    /// Whether evaluating this expression makes row position meaningful.
    ///
    /// A shift is unlike an ordinary expression: its answer changes when
    /// the same rows arrive in a different order. Compilation cannot enforce
    /// that precondition because several compiler call sites deliberately
    /// have no frame. The operation-preparation boundary does, so it asks the
    /// expression this narrow structural question before accepting a write.
    pub(crate) fn uses_row_shift(&self) -> bool {
        let mut found = false;
        self.walk(&mut |expression| {
            if let Expr::Method { path, .. } = expression {
                found |= matches!(path.as_slice(), [name] if matches!(name.as_str(), "shift" | "lag" | "lead"));
            }
        });
        found
    }

    /// Whether this expression accumulates over the rows seen so far.
    ///
    /// Cumulative functions are recurrence in their useful, vectorized
    /// form. They need the same explicit order as shift, but naming them
    /// separately lets the refusal describe the gesture the author made
    /// instead of calling a running total a shift.
    pub(crate) fn uses_running_calculation(&self) -> bool {
        let mut found = false;
        self.walk(&mut |expression| {
            if let Expr::Method { path, .. } = expression {
                found |= matches!(
                    path.as_slice(),
                    [name]
                        if matches!(
                            name.as_str(),
                            "cum_count" | "cum_sum" | "cum_prod" | "cum_min" | "cum_max" | "cumulative_eval"
                        )
                );
            }
        });
        found
    }

    /// Whether this formula asks for an ordered, seed-based calculation down
    /// one column. The sequential execution cost is deliberate; the explicit
    /// Sort step is the semantic guard that makes "previous" mean one thing.
    pub(crate) fn uses_recurrence(&self) -> bool {
        self.any(
            |expression| matches!(expression, Expr::PolarsCall { name, .. } if name == "recur"),
        )
    }

    /// The two expressions and optional restart keys of a top-level
    /// recurrence, validated before a plan tries to execute it.
    pub(crate) fn recurrence_parts(&self) -> Result<Option<RecurrenceParts<'_>>, String> {
        let Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } = self
        else {
            return Ok(None);
        };
        if name != "recur" {
            return Ok(None);
        }
        let [seed, next] = arguments.as_slice() else {
            return Err("recur expects a first-row value and a next-row formula".into());
        };
        let is_previous = |expression: &Expr| matches!(expression, Expr::PolarsCall { name, .. } if name == "previous");
        if seed.any(is_previous) {
            return Err(
                "The first row has no previous result. Enter its starting value instead.".into(),
            );
        }
        if !next.any(is_previous) {
            return Err(
                "The next-row formula needs previous() so it can read the earlier result.".into(),
            );
        }
        let mut invalid_previous = false;
        next.walk(&mut |expression| {
            if let Expr::PolarsCall {
                name,
                arguments,
                keyword_arguments,
            } = expression
                && name == "previous"
                && (!arguments.is_empty() || !keyword_arguments.is_empty())
            {
                invalid_previous = true;
            }
        });
        if invalid_previous {
            return Err("previous() takes no arguments".into());
        }
        if next.any(
            |expression| matches!(expression, Expr::PolarsCall { name, .. } if name == "recur"),
        ) {
            return Err("Put one recur calculation in each calculated column".into());
        }
        let mut restart_by = Vec::new();
        for (keyword, value) in keyword_arguments {
            if keyword != "restart_by" {
                return Err(format!("recur does not accept the keyword ‘{keyword}’"));
            }
            let Expr::List { items } = value else {
                return Err("restart_by expects a list of columns".into());
            };
            for item in items {
                let Expr::Column { column_id } = item else {
                    return Err("restart_by expects only column names".into());
                };
                restart_by.push(column_id);
            }
        }
        Ok(Some(RecurrenceParts {
            seed,
            next,
            restart_by,
        }))
    }

    /// Whether this expression asks for the row count of the current frame.
    pub(crate) fn uses_frame_length(&self) -> bool {
        self.any(
            |expression| matches!(expression, Expr::PolarsCall { name, .. } if name == "frame_len"),
        )
    }

    /// A sequence tied to `frame.len()` fills rows by position and therefore
    /// needs the same declared-order guard as shift.
    pub(crate) fn uses_frame_sequence(&self) -> bool {
        self.any(|expression| {
            matches!(expression, Expr::PolarsCall { name, .. } if name == "sequence")
                && expression.uses_frame_length()
        })
    }

    /// Whether this expression names a given column anywhere inside it.
    ///
    /// Column ids are unique across the whole document, so a column read out
    /// of another frame is named by the same id its own frame knows it by —
    /// which is what lets this one check protect a column from being deleted
    /// out from under a formula anywhere.
    pub(crate) fn references_column(&self, target_column_id: &str) -> bool {
        self.any(|expression| match expression {
            Expr::Column { column_id } | Expr::ForeignColumn { column_id, .. } => {
                column_id == target_column_id
            }
            _ => false,
        })
    }

    /// Whether this expression names a given object on the canvas — a value
    /// or a list, both of which are read by id and both of which something
    /// would break by deleting.
    pub(crate) fn references_object(&self, target_object_id: &str) -> bool {
        self.any(|expression| match expression {
            Expr::Value { object_id } | Expr::Series { object_id } => object_id == target_object_id,
            _ => false,
        })
    }

    /// What this expression evaluates to, as far as it can be told without
    /// running it: one value, a list of its own, or a column of a frame.
    ///
    /// Only precise enough for the one question that needs asking — whether
    /// a binary expression is about to pair a list with a column. Anything
    /// it cannot work out is [`Shape::Scalar`], which is the answer that
    /// refuses nothing; being wrong that way costs an error at evaluation,
    /// and being wrong the other way costs arithmetic somebody meant.
    pub(crate) fn shape(&self, document: &Document) -> Shape {
        match self {
            Expr::Column { .. } => Shape::Column,
            Expr::ForeignColumn { frame_id, .. } => {
                match document.snapshot_row_count(frame_id).unwrap_or(1) {
                    1 => Shape::Scalar,
                    _ => Shape::Column,
                }
            }
            Expr::Series { .. } | Expr::List { .. } => Shape::List,
            Expr::PolarsCall { name, .. } if name == "sequence" => {
                if self.uses_frame_length() {
                    Shape::Column
                } else {
                    Shape::List
                }
            }
            // A block line may itself be a list now, so the shape of a
            // reference is the shape of what it refers to. Results and
            // written values are single by construction.
            Expr::Value { object_id } => document
                .block_line(object_id)
                .and_then(|(block, index)| block.lines[index].expression().cloned())
                .map(|expression| expression.shape(document))
                .unwrap_or(Shape::Scalar),
            Expr::Negate { expression } | Expr::Not { expression } => expression.shape(document),
            Expr::Binary { left, right, .. } => {
                match (left.shape(document), right.shape(document)) {
                    (Shape::Column, _) | (_, Shape::Column) => Shape::Column,
                    (Shape::List, _) | (_, Shape::List) => Shape::List,
                    _ => Shape::Scalar,
                }
            }
            // A method that folds a list down answers with one value; any
            // other keeps the shape of what it was called on.
            Expr::Method { input, path, .. } => match path.last() {
                Some(name) if REDUCING_METHODS.contains(&name.as_str()) => Shape::Scalar,
                // Like a written list, a filtered expression no longer has
                // one value for every input row. It is only valid on a
                // frame when some later aggregate folds it back down.
                Some(name) if name == "filter" => Shape::List,
                _ => input.shape(document),
            },
            _ => Shape::Scalar,
        }
    }

    /// Whether `predicate` holds of this expression or anything inside it.
    fn any(&self, predicate: impl Fn(&Expr) -> bool) -> bool {
        let mut found = false;
        self.walk(&mut |expression| found = found || predicate(expression));
        found
    }

    pub(crate) fn render(
        &self,
        frame: &FrameObject,
        document: &Document,
        parent_precedence: u8,
    ) -> String {
        self.render_in_scope(frame, document, None, parent_precedence)
    }

    /// [`Expr::render`] with the block the formula sits in, when it sits in
    /// one. The block plays the part the frame does for columns: a sibling
    /// line renders bare inside its own block, and qualified through the
    /// block's name from anywhere else — so the text that comes out is the
    /// text a person in that scope would type.
    pub(crate) fn render_in_scope(
        &self,
        frame: &FrameObject,
        document: &Document,
        block: Option<&crate::model::value::BlockObject>,
        parent_precedence: u8,
    ) -> String {
        match self {
            Expr::Integer { value } => value.to_string(),
            Expr::Number { value } => format_number(*value),
            // Back out as it went in: the value is the fraction, and the
            // sign is put back on the figure a person would have typed.
            Expr::Percentage { value } => format!("{}%", format_number(*value * 100.0)),
            Expr::Money { value } => format!("${}", format_number(*value)),
            Expr::String { value } => {
                serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
            }
            Expr::Boolean { value } => if *value { "True" } else { "False" }.into(),
            Expr::Date { value } => value.format("%Y-%m-%d").to_string(),
            Expr::Duration { value } => value.clone(),
            Expr::Null => "None".into(),
            Expr::Column { column_id } => frame
                .columns
                .iter()
                .find(|column| column.id == *column_id)
                .map(|column| formula_name(&column.name))
                .unwrap_or_else(|| "#REF".into()),
            Expr::ForeignColumn {
                frame_id,
                column_id,
            } => document
                .frame(frame_id)
                .ok()
                .and_then(|other| {
                    other
                        .columns
                        .iter()
                        .find(|column| column.id == *column_id)
                        .map(|column| {
                            format!(
                                "{}.{}",
                                formula_name(&other.name),
                                formula_name(&column.name)
                            )
                        })
                })
                .unwrap_or_else(|| "#REF".into()),
            // Written back with the containers it sits in, outermost first.
            // The name that comes out is the name that goes in: reading a
            // formula and typing one are the same act, and a member rendered
            // bare would be a name that only resolves by luck.
            Expr::Value { object_id } | Expr::Series { object_id } => {
                let Ok(object) = document.object(object_id) else {
                    // Not an object: a block line, addressed by the same id
                    // its formula travels under. Bare from a sibling in the
                    // same block, `` `Block`.`line` `` from anywhere else.
                    //
                    // The scope is asked first and believed: when a block is
                    // being rewritten it holds names the document has not
                    // been told about yet, and those are the names the text
                    // coming out has to use.
                    let scoped = block.and_then(|current| {
                        current
                            .lines
                            .iter()
                            .find(|line| line.id == *object_id)
                            .map(|line| (current, line))
                    });
                    let Some((owner, line)) = scoped.or_else(|| {
                        document
                            .block_line(object_id)
                            .map(|(owner, index)| (owner, &owner.lines[index]))
                    }) else {
                        return "#REF".into();
                    };
                    return if block.is_some_and(|current| current.id == owner.id) {
                        // Bare where bare is unambiguous: the text that
                        // comes out of a scratchpad has to be the text
                        // somebody would have typed into it.
                        if crate::formula::line::is_bare_name(&line.name) {
                            line.name.clone()
                        } else {
                            formula_name(&line.name)
                        }
                    } else {
                        format!("{}.{}", formula_name(&owner.name), formula_name(&line.name))
                    };
                };
                let mut path = vec![formula_name(object.name())];
                let mut current = object_id.as_str();
                while let Some(container) = document.container_of(current) {
                    path.push(formula_name(&container.name));
                    current = &container.id;
                }
                path.reverse();
                path.join(".")
            }
            Expr::List { items } => format!(
                "[{}]",
                items
                    .iter()
                    .map(|item| item.render_in_scope(frame, document, block, 0))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::Negate { expression } => {
                format!("-{}", expression.render_in_scope(frame, document, block, 6))
            }
            Expr::Not { expression } => {
                format!("~{}", expression.render_in_scope(frame, document, block, 6))
            }
            Expr::Binary {
                operator,
                left,
                right,
            } => {
                let (symbol, precedence) = match operator {
                    BinaryOperator::Or => ("|", 1),
                    BinaryOperator::And => ("&", 2),
                    BinaryOperator::Equal => ("==", 3),
                    BinaryOperator::NotEqual => ("!=", 3),
                    BinaryOperator::Less => ("<", 3),
                    BinaryOperator::LessEqual => ("<=", 3),
                    BinaryOperator::Greater => (">", 3),
                    BinaryOperator::GreaterEqual => (">=", 3),
                    BinaryOperator::Add => ("+", 4),
                    BinaryOperator::Subtract => ("-", 4),
                    BinaryOperator::Multiply => ("*", 5),
                    BinaryOperator::Divide => ("/", 5),
                    BinaryOperator::FloorDivide => ("//", 5),
                    BinaryOperator::Modulo => ("%", 5),
                    BinaryOperator::Power => ("**", 7),
                };
                let rendered = format!(
                    "{} {} {}",
                    left.render_in_scope(frame, document, block, precedence),
                    symbol,
                    right.render_in_scope(frame, document, block, precedence + 1)
                );
                if precedence < parent_precedence {
                    format!("({rendered})")
                } else {
                    rendered
                }
            }
            Expr::PolarsCall {
                name,
                arguments,
                keyword_arguments,
            } => format_polars_call(name, arguments, keyword_arguments, frame, document, block),
            Expr::Method {
                input,
                path,
                arguments,
                keyword_arguments,
            } => format!(
                "{}.{}",
                input.render_in_scope(frame, document, block, 8),
                format_polars_call(
                    &path.join("."),
                    arguments,
                    keyword_arguments,
                    frame,
                    document,
                    block,
                )
            ),
        }
    }
}

pub(crate) fn keyword_argument<'a>(
    arguments: &'a [(String, Expr)],
    name: &str,
) -> Option<&'a Expr> {
    arguments
        .iter()
        .find_map(|(candidate, value)| (candidate == name).then_some(value))
}

pub(crate) fn formula_name(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

pub(crate) fn reference_matches(candidate: &str, reference: &ReferenceName) -> bool {
    if reference.exact {
        candidate == reference.value
    } else {
        normalize_name(candidate) == normalize_name(&reference.value)
    }
}

#[cfg(test)]
mod tests {

    #[allow(unused_imports)]
    use crate::test_support::*;
    #[allow(unused_imports)]
    use crate::*;
    #[allow(unused_imports)]
    use std::{fs, path::PathBuf};
    #[allow(unused_imports)]
    use uuid::Uuid;

    #[test]
    pub(crate) fn formula_references_use_exact_backticks_and_support_frame_qualification() {
        let mut document = Document::demo();
        // Pushed rather than added, because a value has no home on the bare
        // canvas any more and this test is about how a name is written, not
        // about where the thing it names is allowed to sit.
        document.objects.push(DataObject::Value(ValueObject {
            id: id(),
            name: "Safety Factor".into(),
            raw: "1.7".into(),
            data_type: DataType::Number,
        }));
        let frame = document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame),
                _ => None,
            })
            .unwrap();
        let frame_id = frame.id.clone();

        for source in [
            "`Quantity` * `Safety Factor`",
            "`Orders`.`Quantity` * `Safety Factor`",
        ] {
            let expression = document.parse_formula_for_frame(&frame_id, source).unwrap();
            let mut dependencies = Vec::new();
            expression.column_dependencies(&mut dependencies);
            assert_eq!(dependencies.len(), 1);
        }

        let expression = document
            .parse_formula_for_frame(&frame_id, "`Quantity` * `Safety Factor`")
            .unwrap();
        assert_eq!(
            expression.render(document.frame(&frame_id).unwrap(), &document, 0),
            "`Quantity` * `Safety Factor`"
        );
    }

    #[test]
    pub(crate) fn exact_backtick_references_disambiguate_names_and_escape_backticks() {
        let mut document = Document::demo();
        document.objects.push(DataObject::Value(ValueObject {
            id: id(),
            name: "Safety Factor".into(),
            raw: "1.7".into(),
            data_type: DataType::Number,
        }));
        document.objects.push(DataObject::Value(ValueObject {
            id: id(),
            name: "SafetyFactor".into(),
            raw: "2".into(),
            data_type: DataType::Number,
        }));
        document.objects.push(DataObject::Value(ValueObject {
            id: id(),
            name: "Cost ` cap".into(),
            raw: "3".into(),
            data_type: DataType::Number,
        }));
        let frame_id = document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame.id.clone()),
                _ => None,
            })
            .unwrap();

        let unquoted = document.parse_formula_for_frame(&frame_id, "SafetyFactor");
        assert!(matches!(
            unquoted,
            Err(CoreError::Formula(message)) if message.contains("backticks")
        ));

        let exact = document
            .parse_formula_for_frame(&frame_id, "`Safety Factor` + `Cost `` cap`")
            .unwrap();
        assert_eq!(
            exact.render(document.frame(&frame_id).unwrap(), &document, 0),
            "`Safety Factor` + `Cost `` cap`"
        );

        let wrong_case = document.parse_formula_for_frame(&frame_id, "`safety factor`");
        assert!(
            matches!(wrong_case, Err(CoreError::Formula(message)) if message.contains("Unknown"))
        );
    }
}
