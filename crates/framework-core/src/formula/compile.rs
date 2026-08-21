use crate::formula::ast::{BinaryOperator, Expr};
use crate::model::document::{DataObject, Document};
use crate::model::frame::FrameObject;
use crate::model::value::{DataType, ScalarValue};
use crate::{keyword_argument, parse_scalar_value};
use polars::prelude as pl;
use polars::prelude::NamedFrom;

const MAX_SEQUENCE_VALUES: usize = 1_000_000;

impl Expr {
    pub(crate) fn to_polars(&self, document: &Document) -> Result<pl::Expr, String> {
        match self {
            Expr::Integer { value } => Ok(pl::lit(*value)),
            Expr::Number { value } => Ok(pl::lit(*value)),
            // The fraction, because that is what it is. Polars has no idea
            // this was written with a sign on it and does not need one:
            // the notation is carried by the type, and only reading it back
            // out — rendering the formula, formatting the answer — asks.
            Expr::Percentage { value } | Expr::Money { value } => Ok(pl::lit(*value)),
            Expr::String { value } => Ok(pl::lit(value.clone())),
            Expr::Boolean { value } => Ok(pl::lit(*value)),
            Expr::Date { value } => Ok(pl::lit(*value)),
            Expr::Duration { value } => Err(format!(
                "‘{value}’ is a length of time, not a value. Add it to a date \
                 or subtract it from one."
            )),
            Expr::Null => Ok(pl::lit(pl::NULL)),
            Expr::Column { column_id } => Ok(pl::col(column_id)),
            Expr::ForeignColumn {
                frame_id,
                column_id,
            } => foreign_column_literal(document, frame_id, column_id),
            Expr::Value { object_id } => {
                let object = document
                    .objects
                    .iter()
                    .find(|object| object.id() == object_id);
                match object {
                    Some(DataObject::Value(value)) => {
                        scalar_to_polars_literal(parse_scalar_value(&value.raw, value.data_type)?)
                    }
                    // A result is its formula. Inlined rather than evaluated
                    // here, so a chain of results compiles into one Polars
                    // expression; the document refuses a result that reaches
                    // itself, which is what makes this recursion finite.
                    Some(DataObject::Result(result)) => {
                        inline_value(document, object_id, &result.formula.expression)
                    }
                    Some(_) => Err("Value not found".to_string()),
                    // Not an object at all: a block line, which is a result
                    // held in a block. Same inlining, same finiteness — the
                    // document refuses a line that reaches itself, and lines
                    // of one block only read upward.
                    None => {
                        let line = document
                            .block_line(object_id)
                            .map(|(block, index)| &block.lines[index])
                            .ok_or_else(|| "Value not found".to_string())?;
                        // A line that does not compute is not a hole to fall
                        // into: the formula reading it says whose fault it
                        // is, in the gutter next to itself.
                        let expression = line.expression().ok_or_else(|| {
                            format!("‘{}’ does not compute, so this cannot either", line.name)
                        })?;
                        inline_value(document, object_id, expression)
                    }
                }
            }
            Expr::Series { object_id } => {
                let series = document
                    .objects
                    .iter()
                    .find_map(|object| match object {
                        DataObject::Series(series) if series.id == *object_id => Some(series),
                        _ => None,
                    })
                    .ok_or_else(|| "List not found".to_string())?;
                Ok(pl::lit(crate::series_to_polars(series)?))
            }
            Expr::List { items } => compile_written_list(items, document),
            Expr::Negate { expression } => Ok(-expression.to_polars(document)?),
            Expr::Not { expression } => Ok(expression.to_polars(document)?.not()),
            Expr::Binary {
                operator,
                left,
                right,
            } => {
                if let Some(offset) = compile_date_offset(*operator, left, right, document)? {
                    return Ok(offset);
                }
                if let Some(offset) = compile_date_day_arithmetic(*operator, left, right, document)?
                {
                    return Ok(offset);
                }
                if *operator == BinaryOperator::Add
                    && let Some(joined) = compile_text_join(left, right, document)?
                {
                    return Ok(joined);
                }
                // Equality against null asks whether a value is missing, so
                // it compiles to the question being asked. Left to Polars,
                // `x == null` is null for every row — which a filter reads
                // as false — and `x != null` matches nothing, silently: two
                // entries existed and the filter found zero. SQL made the
                // same distinction and requires IS NULL; a spreadsheet
                // formula should just mean the obvious thing.
                if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual) {
                    let value = match (left.as_ref(), right.as_ref()) {
                        (Expr::Null, Expr::Null) => {
                            return Ok(pl::lit(*operator == BinaryOperator::Equal));
                        }
                        (Expr::Null, value) | (value, Expr::Null) => Some(value),
                        _ => None,
                    };
                    if let Some(value) = value {
                        let compiled = value.to_polars(document)?;
                        return Ok(if *operator == BinaryOperator::Equal {
                            compiled.is_null()
                        } else {
                            compiled.is_not_null()
                        });
                    }
                }
                let left = left.to_polars(document)?;
                let right = right.to_polars(document)?;
                Ok(match operator {
                    BinaryOperator::Add => left + right,
                    BinaryOperator::Subtract => left - right,
                    BinaryOperator::Multiply => left * right,
                    // `/` is spreadsheet division, not integer division.
                    // Integer storage must not turn `1 / 2` into zero or
                    // make a variance percentage jump to a whole number.
                    BinaryOperator::Divide => {
                        left.cast(pl::DataType::Float64) / right.cast(pl::DataType::Float64)
                    }
                    BinaryOperator::FloorDivide => left.floor_div(right),
                    BinaryOperator::Modulo => left % right,
                    // Polars keeps integer and floating-point exponentiation
                    // deliberately strict. Formula literals are floats, so
                    // an imported integer base such as `Height ** 2` would
                    // otherwise become null row by row. Spreadsheet
                    // arithmetic promotes this operation to a float.
                    BinaryOperator::Power => left
                        .cast(pl::DataType::Float64)
                        .pow(right.cast(pl::DataType::Float64)),
                    BinaryOperator::Equal => left.eq(right),
                    BinaryOperator::NotEqual => left.neq(right),
                    BinaryOperator::Less => left.lt(right),
                    BinaryOperator::LessEqual => left.lt_eq(right),
                    BinaryOperator::Greater => left.gt(right),
                    BinaryOperator::GreaterEqual => left.gt_eq(right),
                    BinaryOperator::And => left.logical_and(right),
                    BinaryOperator::Or => left.logical_or(right),
                })
            }
            Expr::PolarsCall {
                name,
                arguments,
                keyword_arguments,
            } => compile_polars_root_call(name, arguments, keyword_arguments, document),
            Expr::Method {
                input,
                path,
                arguments,
                keyword_arguments,
            } => {
                if path.as_slice() == ["otherwise"] {
                    return compile_when_chain(input, arguments, keyword_arguments, document);
                }
                // Read before the input is compiled, because turning a value
                // into text is a question about what it *is* — and the answer
                // lives in the expression, not in the Polars tree it becomes.
                if path.as_slice() == ["cast"] {
                    return compile_cast(input, arguments, keyword_arguments, document);
                }
                if path.as_slice() == ["str", "to_date"] {
                    return compile_string_to_date(input, arguments, keyword_arguments, document);
                }
                if path.as_slice() == ["show"] {
                    return compile_show(input, arguments, keyword_arguments, document);
                }
                compile_polars_method(
                    input.to_polars(document)?,
                    path,
                    arguments,
                    keyword_arguments,
                    document,
                )
            }
        }
    }
}

/// Compiles `date + duration` and `date - duration`, or says this is not
/// one of those.
///
/// Routed through `offset_by` rather than through arithmetic, because the
/// calendar units are why a duration is a type at all. `today() - 1mo`
/// asks for the same day of the previous month, whatever that month's
/// length; subtracting a fixed span would land on a different day
/// depending on the time of year, which is not what anyone meant.
///
/// Adding is symmetric — `30d + today()` reads oddly but means the same
/// thing. Subtracting is not: a date does not sit inside a length of time,
/// so `30d - today()` is refused rather than quietly reinterpreted.
fn compile_date_offset(
    operator: BinaryOperator,
    left: &Expr,
    right: &Expr,
    document: &Document,
) -> Result<Option<pl::Expr>, String> {
    let offset_of = |expression: &Expr| match expression {
        Expr::Duration { value } => Some(value.clone()),
        _ => None,
    };
    let (base, offset) = match (operator, offset_of(left), offset_of(right)) {
        (BinaryOperator::Add, Some(offset), None) => (right, offset),
        (BinaryOperator::Add | BinaryOperator::Subtract, None, Some(offset)) => (left, offset),
        (BinaryOperator::Subtract, Some(offset), _) => {
            return Err(format!(
                "‘{offset}’ is a length of time, so nothing can be subtracted from it. \
                 Write the date first."
            ));
        }
        _ => return Ok(None),
    };
    let signed = if operator == BinaryOperator::Subtract {
        format!("-{offset}")
    } else {
        offset
    };
    Ok(Some(
        base.to_polars(document)?.dt().offset_by(pl::lit(signed)),
    ))
}

/// Compiles `date + n` and `date - n` where `n` is integer-typed, or says
/// this is not one of those.
///
/// This is the Excel habit — `A1 + 1` is tomorrow — and it is unambiguous:
/// the only length an unadorned integer can mean against a date is days,
/// because every other calendar unit already has a duration spelling. The
/// integer may be a column or a value, not just a literal, so the offset
/// string is assembled row by row rather than at compile time. Refusing
/// `n - date` matches the duration rule above: a date does not sit inside
/// a number.
fn compile_date_day_arithmetic(
    operator: BinaryOperator,
    left: &Expr,
    right: &Expr,
    document: &Document,
) -> Result<Option<pl::Expr>, String> {
    if !matches!(operator, BinaryOperator::Add | BinaryOperator::Subtract) {
        return Ok(None);
    }
    let is_date = |expression: &Expr| expression.declared_type(document) == Some(DataType::Date);
    let is_days = |expression: &Expr| expression.declared_type(document) == Some(DataType::Integer);
    let (base, days) = match (
        is_date(left) && is_days(right),
        operator == BinaryOperator::Add && is_days(left) && is_date(right),
    ) {
        (true, _) => (left, right),
        (_, true) => (right, left),
        _ => return Ok(None),
    };
    let days = days.to_polars(document)?;
    let days = if operator == BinaryOperator::Subtract {
        days * pl::lit(-1)
    } else {
        days
    };
    Ok(Some(
        base.to_polars(document)?
            .dt()
            .offset_by(day_count_string(days)),
    ))
}

/// An integer expression rendered as the `"{n}d"` offset text `offset_by`
/// reads. A null count stays null through the concatenation, so a missing
/// count makes a missing date rather than a corrupt offset.
fn day_count_string(days: pl::Expr) -> pl::Expr {
    days.cast(pl::DataType::String) + pl::lit("d")
}

/// The offset handed to `.dt.offset_by(...)`: an integer-typed argument is
/// read as a whole-day count, anything else is the duration text Polars
/// already accepts. Decided from the written expression's declared type, so
/// a string column of mixed offsets ("2mo", "-3d") keeps working untouched.
fn day_count_offset(argument: &Expr, compiled: pl::Expr, document: &Document) -> pl::Expr {
    if argument.declared_type(document) == Some(DataType::Integer) {
        day_count_string(compiled)
    } else {
        compiled
    }
}

pub(crate) fn format_polars_call(
    name: &str,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    frame: &FrameObject,
    document: &Document,
    block: Option<&crate::model::value::BlockObject>,
) -> String {
    if name == "frame_len" {
        return "frame.len()".into();
    }
    let mut rendered = arguments
        .iter()
        .map(|argument| argument.render_in_scope(frame, document, block, 0))
        .collect::<Vec<_>>();
    rendered.extend(keyword_arguments.iter().map(|(name, argument)| {
        format!(
            "{name}={}",
            argument.render_in_scope(frame, document, block, 0)
        )
    }));
    format!("{name}({})", rendered.join(", "))
}

/// Reads a column out of another frame as a literal.
///
/// A literal rather than a column reference, because it is not a column of
/// the plan this expression is being compiled into — the values come from
/// somewhere else and get carried in. One row arrives as a single value,
/// which broadcasts down whatever column it lands in; more than one arrives
/// as the whole series, which only the functions that take a list will
/// accept. Which of the two it is has already been settled at parse time,
/// under the same rule lists follow; this just honours it.
///
/// A frame formula reaches this with a snapshot; top-level Scratchwork may
/// instead evaluate the other frame's current lazy plan. Either way the
/// values arrive here by stable column id rather than by screen position.
///
/// A result or block line, as it appears inside a formula that reads it.
///
/// Frozen first: the whole point of writing an answer down is that everything
/// afterwards sees the written-down one. Failing that, a value that reads
/// live data is refused here rather than allowed to drag a frame's plan into
/// whatever is compiling — including, possibly, the plan of the very frame it
/// reads. This refusal is what keeps the graph finite now that a value may
/// name a frame with no snapshot.
fn inline_value(
    document: &Document,
    object_id: &str,
    expression: &Expr,
) -> Result<pl::Expr, String> {
    if document.frozen_values.contains_key(object_id) {
        let (_, series) = crate::engine::compute::read_frozen_series(
            &document.frozen_values[object_id].artifact.path,
        )?;
        // What was written down, at the length it was written at. One value
        // carries in as a literal that broadcasts; more than one carries in
        // as the whole list, and meets the same rules any other list does.
        return match series.len() {
            1 => scalar_to_polars_literal(crate::polars_value_at(&series, 0)?),
            _ => Ok(pl::lit(series)),
        };
    }
    if let Some(frame_id) = document.first_live_frame(expression) {
        return Err(document.freeze_required(frame_id));
    }
    expression.to_polars(document)
}

fn foreign_column_literal(
    document: &Document,
    frame_id: &str,
    column_id: &str,
) -> Result<pl::Expr, String> {
    let (frame_name, column_name) = document.foreign_names(frame_id, column_id);
    let frame = document
        .frame(frame_id)
        .map_err(|_| format!("‘{frame_name}’ is gone, so ‘{column_name}’ cannot be read"))?;
    // A snapshot if there is one, and the frame's own plan if there is not.
    //
    // Reaching the second of those means Scratchwork asked for a live read
    // deliberately: a column formula cannot name a frame without a snapshot
    // at all. This is a semantic column query, not a captured row ordinal.
    let frame = match frame.materialization.as_ref() {
        Some(materialization) => pl::LazyFrame::scan_parquet(
            pl::PlRefPath::new(&materialization.artifact.path),
            pl::ScanArgsParquet::default(),
        )
        .and_then(|scan| scan.select([pl::col(column_id)]).collect())
        .map_err(|error| {
            format!("‘{frame_name}’.‘{column_name}’ could not be read from the snapshot: {error}")
        })?,
        None => document
            .materialize_frame_lazy(
                frame_id,
                crate::engine::Layer::Data,
                &mut Default::default(),
            )
            .and_then(|plan| {
                plan.select([pl::col(column_id)])
                    .collect()
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| {
                format!("‘{frame_name}’.‘{column_name}’ could not be read: {error}")
            })?,
    };
    let series = frame
        .column(column_id)
        .map_err(|error| error.to_string())?
        .as_materialized_series();
    if series.len() == 1 {
        return scalar_to_polars_literal(crate::polars_value_at(series, 0)?);
    }
    Ok(pl::lit(series.clone()))
}

fn scalar_to_polars_literal(value: ScalarValue) -> Result<pl::Expr, String> {
    Ok(match value {
        ScalarValue::Null => pl::lit(pl::NULL),
        ScalarValue::Number(value) => pl::lit(value),
        ScalarValue::String(value) => pl::lit(value),
        ScalarValue::Boolean(value) => pl::lit(value),
        ScalarValue::Date(value) => pl::lit(value),
    })
}

/// Compiles `"Q" + 1` — text joined to something that is not text — or
/// says this is not one of those.
///
/// `+` already joins two pieces of text, and there is nothing else `+`
/// could mean with text on one side of it, so it means this. It is the one
/// gesture every spreadsheet user already owns: `="Q"&A1`.
///
/// What makes it safe to do quietly is that the writing is *this document's*
/// writing. Polars would happily render the number itself, but it would
/// render it as `1.0`, a date as a nanosecond timestamp, and a third as
/// `0.3333333333333333` — a formatter this document does not own and does
/// not agree with. Handing a number to [`as_text`] instead gets back what
/// the gutter would have shown, so a number reads the same inside a
/// sentence as it does beside one.
///
/// Only where the type is known. What a Polars method hands back is not
/// written down anywhere, so those keep the arithmetic they always had and
/// fail the way they always did.
fn compile_text_join(
    left: &Expr,
    right: &Expr,
    document: &Document,
) -> Result<Option<pl::Expr>, String> {
    let is_text = |data_type| matches!(data_type, DataType::String | DataType::Categorical);
    let (Some(left_type), Some(right_type)) =
        (left.declared_type(document), right.declared_type(document))
    else {
        return Ok(None);
    };
    // One side text and the other not. Two pieces of text already join, and
    // two numbers already add.
    if is_text(left_type) == is_text(right_type) {
        return Ok(None);
    }
    Ok(Some(
        as_text(left, left_type, document)? + as_text(right, right_type, document)?,
    ))
}

/// One operand of a text join, written the way this document writes it.
///
/// The rules are [`crate::engine::values::format_scalar_value`]'s, in
/// Polars: a whole number loses its `.0`, a fraction keeps four decimal
/// places, and a date is the `YYYY-MM-DD` it was typed as rather than the
/// midnight timestamp it is stored as.
///
/// Money and percentages arrive as their plain numbers. That is the same
/// call the promotion tree makes for a list holding both — a format is
/// dropped rather than guessed at — and `format` is where a spelling gets
/// asked for on purpose.
fn as_text(
    expression: &Expr,
    data_type: DataType,
    document: &Document,
) -> Result<pl::Expr, String> {
    let compiled = expression.to_polars(document)?;
    Ok(match data_type {
        DataType::String | DataType::Categorical => compiled,
        DataType::Boolean => compiled.cast(pl::DataType::String),
        DataType::Date => compiled.dt().strftime("%Y-%m-%d"),
        DataType::Integer | DataType::Number => plain_number_text(compiled),
        // Money and a percentage are a number and a way of writing it, and
        // this is the place that writing gets done: `"rate: " + 4.25%` says
        // `rate: 4.25%`, because that is what the person who typed `4.25%`
        // meant by it. Anything else would be this function handing back a
        // number the document itself would never print.
        DataType::Currency => pl::lit("$") + plain_number_text(compiled),
        DataType::Percentage => plain_number_text(compiled * pl::lit(100.0)) + pl::lit("%"),
    })
}

/// A number written out for reading: four decimal places at most, and no
/// trailing `.0` on a whole one.
///
/// The same shape as [`crate::engine::values::format_number`], which is what
/// formats an answer for the gutter — this is that rule expressed as Polars
/// so it can run over a column, rather than a second opinion about how a
/// number looks.
fn plain_number_text(compiled: pl::Expr) -> pl::Expr {
    compiled
        .round(4, pl::RoundMode::HalfToEven)
        .cast(pl::DataType::String)
        // Only ever a whole number's own `.0`: a literal suffix rather than
        // a pattern, so text that happens to end that way — a version
        // number written down as `v1.0` — is never reached by this at all.
        .str()
        .strip_suffix(pl::lit(".0"))
}

/// The types `cast` can be asked for, by the word somebody writes.
///
/// Storage rather than presentation: money and a percentage are a number
/// with a way of being written, not a fourth and fifth kind of thing to
/// convert to, and offering them here would suggest `cast` can put a `$` on
/// something. Formatting a number for reading is [`as_text`]'s job.
fn cast_target(name: &str) -> Option<DataType> {
    match name {
        "string" | "text" => Some(DataType::String),
        "integer" | "int" => Some(DataType::Integer),
        "number" => Some(DataType::Number),
        "boolean" => Some(DataType::Boolean),
        "date" => Some(DataType::Date),
        _ => None,
    }
}

/// `.cast("string")` and the rest: the type written in quotes.
///
/// Quoted rather than bare, following `is_between(closed="both")` a few
/// lines down — an argument whose meaning is not in its position is written
/// out in this language, and a type name is exactly that. It also keeps the
/// lexer out of it: `String` as a bare word would be a seventh keyword
/// competing with every column somebody might have called `String`.
fn compile_cast(
    input: &Expr,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if !keyword_arguments.is_empty() {
        return Err(".cast does not accept keyword arguments".into());
    }
    let named = match arguments {
        [Expr::String { value }] => value.as_str(),
        _ => {
            return Err(
                ".cast expects one type in quotes — \"string\", \"integer\", \
                        \"number\", \"date\" or \"boolean\""
                    .into(),
            );
        }
    };
    let Some(target) = cast_target(named) else {
        // Category is the one refusal worth explaining rather than listing
        // past. It is a real type here, so being told it does not exist
        // would be a lie -- but it is a type that carries its allowed values
        // with it, and a formula has nowhere to write those down. It is
        // declared on the column, not computed by an expression. The rules
        // that motivate the ask mostly do not need it either: text already
        // sorts rows into named cases, and the case list is filled from the
        // data whether or not the column calls itself a category.
        if matches!(named, "categorical" | "category" | "enum") {
            return Err(
                "A column becomes a category by setting its type, which is where its allowed \
                 values are declared — a formula has nowhere to write those down. For \
                 conditional formatting, ordinary text already sorts rows into named values."
                    .into(),
            );
        }
        return Err(format!(
            "‘{named}’ is not a type this can convert to. Write \"string\", \
             \"integer\", \"number\", \"date\" or \"boolean\"."
        ));
    };
    let compiled = input.to_polars(document)?;
    Ok(match target {
        // Text is a rendering rather than a reinterpretation, so it goes
        // through the same writing `+` uses -- a number cast to text reads
        // the way the gutter would have shown it, not the way Polars would.
        DataType::String => match input.declared_type(document) {
            Some(known) => as_text(input, known, document)?,
            None => compiled.cast(pl::DataType::String),
        },
        DataType::Integer => compiled.cast(pl::DataType::Int64),
        DataType::Number => compiled.cast(pl::DataType::Float64),
        DataType::Boolean => compiled.cast(pl::DataType::Boolean),
        DataType::Date => match input.declared_type(document) {
            // Reading a date out of text is parsing, not reinterpreting the
            // bits, and Polars keeps the two apart.
            Some(DataType::String | DataType::Categorical) => {
                compiled.str().to_date(pl::StrptimeOptions {
                    format: None,
                    strict: true,
                    exact: true,
                    cache: false,
                })
            }
            _ => compiled.cast(pl::DataType::Date),
        },
        other => return Err(format!("{other:?} is not a type this can convert to")),
    })
}

/// `.str.to_date()` is deliberately the narrow, discoverable spelling of the
/// string branch of `.cast("date")`. The shared implementation keeps their
/// strict ISO parser identical; accepting Polars' broader `format`/`strict`
/// options here before FrameWork exposes them would teach calls we cannot
/// faithfully preserve or explain.
fn compile_string_to_date(
    input: &Expr,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if !arguments.is_empty() || !keyword_arguments.is_empty() {
        return Err(".str.to_date() takes no arguments and parses strict YYYY-MM-DD text".into());
    }
    if !matches!(
        input.declared_type(document),
        Some(DataType::String | DataType::Categorical)
    ) {
        return Err(".str.to_date() only accepts a string or categorical column".into());
    }
    let date = [Expr::String {
        value: "date".into(),
    }];
    compile_cast(input, &date, &[], document)
}

/// `.show("money")` — the same number, written differently.
///
/// Compiles to the input and nothing else, because there is nothing here
/// for Polars to do: a dollar sign is not a value, and this changes no
/// value. What it changes lives in the expression, where
/// [`Expr::declared_type`] reads it and everything above it inherits the
/// answer instead of working one out from the arithmetic.
///
/// That is the whole of "overridable, and it breaks the chain": the chain
/// is a walk up the expression, and this node stops the walk.
fn compile_show(
    input: &Expr,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if !keyword_arguments.is_empty() {
        return Err(".show does not accept keyword arguments".into());
    }
    let named = match arguments {
        [Expr::String { value }] => value.as_str(),
        _ => {
            return Err(".show expects one word in quotes — \"money\", \
                        \"percent\" or \"plain\""
                .into());
        }
    };
    if crate::formula::ast::shown_as(named).is_none() {
        return Err(format!(
            "‘{named}’ is not a way of writing a number. Write \"money\", \
             \"percent\" or \"plain\"."
        ));
    }
    match input.declared_type(document) {
        // Only a number is written as money or a rate. Saying it of a date
        // or a piece of text would be a promise the gutter cannot keep.
        Some(DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage)
        | None => input.to_polars(document),
        Some(other) => Err(format!(
            "{} cannot be written as {named}. Only a number can.",
            crate::formula::ast::type_name(other)
        )),
    }
}

/// `format("Q{}", quarters)` — a sentence with holes in it.
///
/// Polars' own `format`, with Polars' own `{}`, and the values written the
/// way this document writes them. It earns its place over `+` where there
/// is more than one hole: `format("{} of {}", part, whole)` stays a sentence
/// you can read, where the same thing spelled with `+` is four operands and
/// a squint.
fn compile_format(
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if !keyword_arguments.is_empty() {
        return Err("format does not accept keyword arguments".into());
    }
    let [Expr::String { value: pattern }, values @ ..] = arguments else {
        return Err("format expects a pattern in quotes first — format(\"Q{}\", ...)".into());
    };
    let chunks: Vec<&str> = pattern.split("{}").collect();
    let holes = chunks.len() - 1;
    if holes != values.len() {
        return Err(format!(
            "This pattern has {holes} ‘{{}}’ and {} value{} after it. They have to match.",
            values.len(),
            if values.len() == 1 { "" } else { "s" }
        ));
    }
    // Built by joining rather than handed to Polars' `format`, so every hole
    // is filled by the same `as_text` a `+` would have used and there is one
    // set of rules for how a value reads inside a sentence.
    let mut built = pl::lit(chunks[0].to_string());
    for (value, tail) in values.iter().zip(chunks.iter().skip(1)) {
        let rendered = match value.declared_type(document) {
            Some(known) => as_text(value, known, document)?,
            None => value.to_polars(document)?.cast(pl::DataType::String),
        };
        built = built + rendered + pl::lit(tail.to_string());
    }
    Ok(built)
}

/// A list somebody wrote down, as the values it holds.
///
/// `[1, 2, 3]` on a scratchpad line is three numbers, and the line holds
/// them — that is what the spec means by a hand-typed list being nothing
/// more than a block line, and why there is no separate way to make one.
///
/// Built by gathering the items into a single list-valued row and then
/// opening it back out into three, because that is the one route that works
/// whatever the items are: a written number, a reference to a line above,
/// arithmetic on both. Polars settles the type across them the way it does
/// everywhere else, so `[1, 2.5]` is numbers and `[1, "a"]` is refused by
/// the same rules that would refuse it in a column.
///
/// Only reachable from a scratchpad. A bare list in a column formula is
/// refused at parse time — see `Expr::validate_list_placement` — and a list
/// handed to a function is spread across its arguments before it ever gets
/// here.
fn compile_written_list(items: &[Expr], document: &Document) -> Result<pl::Expr, String> {
    let compiled = items
        .iter()
        .map(|item| item.to_polars(document))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(pl::concat_list(compiled)
        .map_err(|error| error.to_string())?
        // A written list holds exactly what was written in it. `[1, None, 3]`
        // is three values, the middle one missing, rather than two values and
        // a gap that closed up behind it.
        .explode(pl::ExplodeOptions {
            empty_as_null: true,
            keep_nulls: true,
        }))
}

pub(crate) fn flatten_polars_arguments(
    arguments: &[Expr],
    document: &Document,
) -> Result<Vec<pl::Expr>, String> {
    let mut output = Vec::new();
    for argument in arguments {
        match argument {
            Expr::List { items } => {
                for item in items {
                    output.push(item.to_polars(document)?);
                }
            }
            argument => output.push(argument.to_polars(document)?),
        }
    }
    Ok(output)
}

fn literal_integer(expression: &Expr, label: &str) -> Result<i64, String> {
    match expression {
        Expr::Integer { value } => Ok(*value),
        // Older documents serialized whole literals as Number. Continue to
        // accept those where an integer argument is required; new source is
        // preserved as Integer by the lexer above.
        Expr::Number { value } if value.fract() == 0.0 => Ok(*value as i64),
        _ => Err(format!("{label} must be an integer literal")),
    }
}

/// Compile a finite, literal range into one list-shaped expression.
///
/// A sequence is deliberately not a row-number function. It makes a value in
/// Scratchwork, with the same independent identity as a written list, and is
/// refused in a frame formula by `validate_list_placement`. Letting it expand
/// against an arbitrary frame would align two things merely because their
/// lengths happened to agree — exactly the positional join the document model
/// is built not to perform.
///
/// The stop is excluded, following Python's `range`: adjacent sequences can be
/// joined without repeating their boundary. Dates use a duration directly so
/// `1mo` remains a calendar month; turning integers into offsets would make
/// month ends and leap years somebody's problem again.
fn compile_sequence(
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if keyword_arguments
        .iter()
        .any(|(keyword, _)| keyword != "step" && keyword != "periods")
    {
        return Err("sequence only accepts the keyword arguments step and periods".into());
    }
    if keyword_arguments
        .iter()
        .filter(|(keyword, _)| keyword == "step")
        .count()
        > 1
    {
        return Err("sequence received step more than once".into());
    }
    if keyword_arguments
        .iter()
        .filter(|(keyword, _)| keyword == "periods")
        .count()
        > 1
    {
        return Err("sequence received periods more than once".into());
    }
    if !(1..=3).contains(&arguments.len()) {
        return Err("sequence expects stop, or start, stop, and an optional step".into());
    }
    if arguments.len() == 3 && keyword_argument(keyword_arguments, "step").is_some() {
        return Err("sequence received step both positionally and by name".into());
    }

    let (start, stop) = if arguments.len() == 1 {
        (None, &arguments[0])
    } else {
        (Some(&arguments[0]), &arguments[1])
    };
    let step = arguments
        .get(2)
        .or_else(|| keyword_argument(keyword_arguments, "step"));

    if let Some(periods) = keyword_argument(keyword_arguments, "periods") {
        if arguments.len() != 1 {
            return Err(
                "sequence with periods expects one start date, for example sequence(2026-01-01, periods=frame.len(), step=1d)"
                    .into(),
            );
        }
        let start = &arguments[0];
        if start.declared_type(document) != Some(DataType::Date) {
            return Err("sequence periods currently fills dates from one start date".into());
        }
        let samples = compile_frame_sequence_bound(periods, document)?;
        return compile_date_sequence_periods(start, samples, step, document);
    }

    if arguments.iter().any(Expr::uses_frame_length)
        || keyword_arguments
            .iter()
            .any(|(_, expression)| expression.uses_frame_length())
    {
        let (start, stop) = if arguments.len() == 1 {
            (pl::lit(0i64), &arguments[0])
        } else {
            (
                compile_frame_sequence_bound(&arguments[0], document)?,
                &arguments[1],
            )
        };
        let stop = compile_frame_sequence_bound(stop, document)?;
        let step = match step {
            None => 1,
            Some(expression) => literal_integer(expression, "sequence step")?,
        };
        if step == 0 {
            return Err("sequence step cannot be zero".into());
        }
        return Ok(polars::lazy::dsl::int_range(
            start,
            stop,
            step,
            pl::DataType::Int64,
        ));
    }

    if let Some((start, stop)) = numeric_sequence_bounds(start, stop) {
        return compile_numeric_sequence(start, stop, step);
    }
    if let (Some(Expr::Date { value: start }), Expr::Date { value: stop }) = (start, stop) {
        return compile_dated_sequence(*start, *stop, step);
    }
    if let (None, Expr::Date { .. }) = (start, stop) {
        return Err("a date sequence needs both a start and a stop date".into());
    }

    // Bounds written as expressions rather than literals: a value on the
    // canvas, `Anchor.dt.month_start()`, arithmetic over either. They are
    // folded to the one value they mean right now, the same eager reading a
    // foreign column gets, so `sequence(Anchor.dt.month_start(), Anchor + 1)`
    // regrows whenever the anchor is edited. Only the bounds fold — the step
    // stays literal, because a step is a property of the sequence someone
    // wrote, not a fact about the document.
    let folded_stop = fold_sequence_bound(stop, document)?;
    let folded_start = match start {
        Some(start) => Some(fold_sequence_bound(start, document)?),
        None => None,
    };
    match (folded_start, folded_stop) {
        (Some(ScalarValue::Date(start)), ScalarValue::Date(stop)) => {
            compile_dated_sequence(start, stop, step)
        }
        (None, ScalarValue::Date(_)) => {
            Err("a date sequence needs both a start and a stop date".into())
        }
        (Some(ScalarValue::Number(start)), ScalarValue::Number(stop)) => {
            compile_numeric_sequence(start, stop, step)
        }
        (None, ScalarValue::Number(stop)) => compile_numeric_sequence(0.0, stop, step),
        _ => Err("sequence start and stop must both be numbers or both be dates".into()),
    }
}

/// One sequence bound, worked out to the single value it stands for.
///
/// Runs the bound against a one-row probe exactly the way a scratchpad line
/// evaluates, so anything a line could hold — a value reference, a date
/// method, arithmetic — is a legal bound. An answer that is not exactly one
/// value is refused rather than truncated: a list-shaped bound means someone
/// reached for a column, and truncating would quietly pick a row for them.
fn fold_sequence_bound(expression: &Expr, document: &Document) -> Result<ScalarValue, String> {
    use polars::prelude::IntoLazy;
    let compiled = expression.to_polars(document)?;
    let frame = polars::df!("__bound_probe" => [true])
        .map_err(|error| error.to_string())?
        .lazy()
        .select([compiled.alias("__bound")])
        .collect()
        .map_err(|error| format!("This sequence bound could not be worked out: {error}"))?;
    let series = frame
        .column("__bound")
        .map_err(|error| error.to_string())?
        .as_materialized_series()
        .clone();
    if series.len() != 1 {
        return Err("a sequence bound must be one value, not a list".into());
    }
    crate::polars_value_at(&series, 0)
}

/// The date-sequence checks and dispatch, shared by literal bounds and
/// bounds folded from expressions.
fn compile_dated_sequence(
    start: chrono::NaiveDate,
    stop: chrono::NaiveDate,
    step: Option<&Expr>,
) -> Result<pl::Expr, String> {
    let step_text = sequence_date_step_text(step)?;
    let duration = sequence_date_duration(step)?;
    let descending = start > stop;
    if start != stop && descending != duration.negative() {
        return Err(if descending {
            "a descending date sequence needs a negative step"
        } else {
            "an ascending date sequence needs a positive step"
        }
        .into());
    }
    compile_date_sequence(start, stop, &step_text)
}

fn sequence_date_step_text(step: Option<&Expr>) -> Result<String, String> {
    Ok(match step {
        None => "1d".to_string(),
        Some(Expr::Duration { value }) => value.clone(),
        Some(Expr::Negate { expression }) => match expression.as_ref() {
            Expr::Duration { value } => format!("-{value}"),
            _ => {
                return Err("a date sequence step must be a duration such as 1d or 1mo".into());
            }
        },
        Some(_) => {
            return Err("a date sequence step must be a duration such as 1d or 1mo".into());
        }
    })
}

fn sequence_date_duration(step: Option<&Expr>) -> Result<pl::Duration, String> {
    let step_text = sequence_date_step_text(step)?;
    let duration = pl::Duration::try_parse(&step_text).map_err(|error| error.to_string())?;
    if duration.duration_ns() == 0 {
        return Err("sequence step cannot be zero".into());
    }
    if !duration.is_full_days() {
        return Err("a Date sequence needs a whole-day step such as 1d, 1w, or 1mo".into());
    }
    Ok(duration)
}

/// Make exactly `periods` dates from one start, keeping calendar units.
///
/// Polars' start+interval+sample-count date range is not implemented in the
/// execution engine yet. Building the ordinal range first is equivalent and
/// preserves the important part: every offset is measured from the anchor,
/// so Jan 31 + 1mo, +2mo lands on Feb 28, Mar 31 rather than drifting from
/// the shortened February result.
fn compile_date_sequence_periods(
    start: &Expr,
    periods: pl::Expr,
    step: Option<&Expr>,
    document: &Document,
) -> Result<pl::Expr, String> {
    sequence_date_duration(step)?;
    let step_text = sequence_date_step_text(step)?;
    let unsigned = step_text.trim_start_matches('-');
    let unit_at = unsigned
        .find(|character: char| character.is_ascii_alphabetic())
        .ok_or("a date sequence step needs a time unit")?;
    let mut magnitude = unsigned[..unit_at]
        .parse::<i64>()
        .map_err(|_| "a date sequence step is too large")?;
    if step_text.starts_with('-') {
        magnitude = -magnitude;
    }
    let unit = unsigned[unit_at..].to_string();
    let ordinals = polars::lazy::dsl::int_range(pl::lit(0i64), periods, 1, pl::DataType::Int64);
    let offsets = (ordinals * pl::lit(magnitude)).cast(pl::DataType::String) + pl::lit(unit);
    Ok(start.to_polars(document)?.dt().offset_by(offsets))
}

/// A frame sequence is intentionally narrower than a general Polars range.
/// Its bounds may be integer arithmetic over `frame.len()`, never a column:
/// accepting a column here would make one list per row rather than one column
/// for the frame, and would turn a readable fill declaration into a nested
/// dtype the grid does not claim to edit.
fn compile_frame_sequence_bound(
    expression: &Expr,
    document: &Document,
) -> Result<pl::Expr, String> {
    match expression {
        Expr::Integer { .. } => expression.to_polars(document),
        Expr::Number { value } if value.fract() == 0.0 => expression.to_polars(document),
        Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } if name == "frame_len" && arguments.is_empty() && keyword_arguments.is_empty() => {
            Ok(pl::len())
        }
        Expr::Binary {
            operator,
            left,
            right,
        } if matches!(
            operator,
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply
        ) =>
        {
            let left = compile_frame_sequence_bound(left, document)?;
            let right = compile_frame_sequence_bound(right, document)?;
            Ok(match operator {
                BinaryOperator::Add => left + right,
                BinaryOperator::Subtract => left - right,
                _ => left * right,
            })
        }
        _ => Err(
            "a frame sequence bound must be an integer expression, optionally using frame.len()"
                .into(),
        ),
    }
}

fn compile_date_sequence(
    start: chrono::NaiveDate,
    stop: chrono::NaiveDate,
    step_text: &str,
) -> Result<pl::Expr, String> {
    let unsigned = step_text.trim_start_matches('-');
    let unit_at = unsigned
        .find(|character: char| character.is_ascii_alphabetic())
        .ok_or("a date sequence step needs a time unit")?;
    let magnitude = unsigned[..unit_at]
        .parse::<i64>()
        .map_err(|_| "a date sequence step is too large")?;
    let unit = &unsigned[unit_at..];
    let negative = step_text.starts_with('-');
    let start_ms = start
        .and_hms_opt(0, 0, 0)
        .expect("midnight is always a valid naive time")
        .and_utc()
        .timestamp_millis();
    let mut values = Vec::new();

    for index in 0..=MAX_SEQUENCE_VALUES {
        let current = if index == 0 {
            start
        } else {
            let multiple = magnitude
                .checked_mul(index as i64)
                .ok_or("sequence step is too large")?;
            let offset = pl::Duration::try_parse(&format!(
                "{}{multiple}{unit}",
                if negative { "-" } else { "" }
            ))
            .map_err(|error| error.to_string())?;
            let timestamp = offset
                .add_ms(start_ms, None)
                .map_err(|error| error.to_string())?;
            chrono::DateTime::from_timestamp_millis(timestamp)
                .ok_or("sequence produced a date outside the supported range")?
                .date_naive()
        };
        let before_stop = if negative {
            current > stop
        } else {
            current < stop
        };
        if !before_stop {
            break;
        }
        if values.len() == MAX_SEQUENCE_VALUES {
            return Err(format!(
                "sequence would make more than {MAX_SEQUENCE_VALUES} values; use a larger step"
            ));
        }
        values.push(current);
    }
    Ok(pl::lit(pl::Series::new("sequence".into(), values)))
}

fn compile_numeric_sequence(
    start: f64,
    stop: f64,
    step: Option<&Expr>,
) -> Result<pl::Expr, String> {
    let step = match step {
        None => 1.0,
        Some(Expr::Integer { value }) => *value as f64,
        Some(Expr::Number { value }) => *value,
        Some(_) => return Err("a numeric sequence step must be a number".into()),
    };
    if !start.is_finite() || !stop.is_finite() || !step.is_finite() {
        return Err("sequence values must be finite numbers".into());
    }
    if step == 0.0 {
        return Err("sequence step cannot be zero".into());
    }
    if start != stop && (start < stop) != (step > 0.0) {
        return Err(if start > stop {
            "a descending sequence needs a negative step"
        } else {
            "an ascending sequence needs a positive step"
        }
        .into());
    }

    let possible_values = ((stop - start) / step).ceil().max(0.0);
    if possible_values > MAX_SEQUENCE_VALUES as f64 {
        return Err(format!(
            "sequence would make more than {MAX_SEQUENCE_VALUES} values; use a larger step"
        ));
    }
    // Calculate every value from the start instead of repeatedly adding the
    // step. Besides being quicker to bound, this keeps decimal rounding from
    // drifting far enough to invent an extra endpoint.
    let values = (0..possible_values as usize)
        .map(|index| start + index as f64 * step)
        .filter(|value| {
            if step > 0.0 {
                *value < stop
            } else {
                *value > stop
            }
        })
        .collect::<Vec<_>>();
    let integral = start.fract() == 0.0
        && stop.fract() == 0.0
        && step.fract() == 0.0
        && start >= i64::MIN as f64
        && start <= i64::MAX as f64
        && stop >= i64::MIN as f64
        && stop <= i64::MAX as f64;
    if integral {
        let integers = values
            .into_iter()
            .map(|value| value as i64)
            .collect::<Vec<_>>();
        Ok(pl::lit(pl::Series::new("sequence".into(), integers)))
    } else {
        Ok(pl::lit(pl::Series::new("sequence".into(), values)))
    }
}

fn numeric_literal(expression: &Expr) -> Option<f64> {
    match expression {
        Expr::Integer { value } => Some(*value as f64),
        Expr::Number { value } => Some(*value),
        _ => None,
    }
}

fn numeric_sequence_bounds(start: Option<&Expr>, stop: &Expr) -> Option<(f64, f64)> {
    let stop = numeric_literal(stop)?;
    let start = match start {
        Some(start) => numeric_literal(start)?,
        None => 0.0,
    };
    Some((start, stop))
}

fn compile_polars_root_call(
    name: &str,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if name == "previous" {
        if !arguments.is_empty() || !keyword_arguments.is_empty() {
            return Err("previous() takes no arguments".into());
        }
        return Ok(pl::col(crate::formula::ast::PREVIOUS_RESULT_COLUMN_ID));
    }
    if name == "recur" {
        return Err(
            "recur is a Calculate down rows transformation, not an ordinary row formula".into(),
        );
    }
    if !keyword_arguments.is_empty()
        && !matches!(name, "sum_horizontal" | "mean_horizontal" | "sequence")
    {
        return Err(format!("{name} does not accept these keyword arguments"));
    }
    if matches!(name, "sum_horizontal" | "mean_horizontal")
        && keyword_arguments
            .iter()
            .any(|(keyword, _)| keyword != "ignore_nulls")
    {
        return Err(format!(
            "{name} only accepts the keyword argument ignore_nulls"
        ));
    }
    // Before the arguments are flattened: a pattern and its values are a
    // run of operands, and spreading a list across them would fill one hole
    // with three things.
    if name == "format" {
        return compile_format(arguments, keyword_arguments, document);
    }
    if name == "sequence" {
        return compile_sequence(arguments, keyword_arguments, document);
    }
    let args = flatten_polars_arguments(arguments, document)?;
    match name {
        "sum_horizontal" => {
            let ignore_nulls = horizontal_ignore_nulls(keyword_arguments)?;
            polars::lazy::dsl::sum_horizontal(&args, ignore_nulls)
                .map_err(|error| error.to_string())
        }
        "min_horizontal" => {
            polars::lazy::dsl::min_horizontal(&args).map_err(|error| error.to_string())
        }
        "max_horizontal" => {
            polars::lazy::dsl::max_horizontal(&args).map_err(|error| error.to_string())
        }
        "mean_horizontal" => {
            polars::lazy::dsl::mean_horizontal(&args, horizontal_ignore_nulls(keyword_arguments)?)
                .map_err(|error| error.to_string())
        }
        "coalesce" => Ok(pl::coalesce(&args)),
        "date" => {
            if args.len() != 3 {
                return Err("date(...) expects year, month, and day".into());
            }
            Ok(pl::datetime(pl::DatetimeArgs::new(
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
            ))
            .dt()
            .date())
        }
        // Read here rather than baked into the stored formula, so a saved
        // filter keeps meaning what it says. `to_polars` runs each time the
        // plan is built, so "the last 30 days" is the last 30 days from
        // whenever the frame is read — not from the afternoon it was
        // written. A frozen frame is the exception, and is meant to be:
        // holding still is what freezing one is for.
        "today" => {
            if !args.is_empty() {
                return Err("today() takes no arguments".into());
            }
            Ok(pl::lit(chrono::Local::now().date_naive()))
        }
        "now" => {
            if !args.is_empty() {
                return Err("now() takes no arguments".into());
            }
            Ok(pl::lit(chrono::Local::now().naive_local()))
        }
        "frame_len" => {
            if !args.is_empty() {
                return Err("frame.len() takes no arguments".into());
            }
            Ok(pl::len())
        }
        "when" => Err("when(...) must be followed by .then(...).otherwise(...)".into()),
        _ => crate::formula::generated_bindings::compile_generated_root_call(
            name,
            arguments,
            keyword_arguments,
            document,
        )
        .unwrap_or_else(|| Err(format!("Unsupported Polars function ‘{name}’"))),
    }
}

fn horizontal_ignore_nulls(keyword_arguments: &[(String, Expr)]) -> Result<bool, String> {
    keyword_argument(keyword_arguments, "ignore_nulls")
        .map(|value| match value {
            Expr::Boolean { value } => Ok(*value),
            _ => Err("ignore_nulls must be True or False".to_string()),
        })
        .transpose()
        .map(|value| value.unwrap_or(true))
}

/// `when(a).then(x).when(b).then(y).otherwise(z)` — as many branches as
/// somebody writes, in the order they wrote them.
///
/// More than one branch is the difference between a formula that can name a
/// category and one that cannot. Three columns saying whether a day is a
/// holiday, a weekend, or neither collapse into one label — "Stat", "Reg
/// Holiday", "Work" — and a conditional-formatting rule paints that. Nesting
/// each branch inside the last one's `otherwise` says the same thing and
/// says it inside out, which is why it was worth teaching the chain.
///
/// Compiled by walking the expression rather than by compiling each link,
/// because a `.when(...)` hanging off a `.then(...)` is not an expression on
/// its own: it is half of a branch, and only the whole chain has a value.
fn compile_when_chain(
    input: &Expr,
    otherwise_arguments: &[Expr],
    otherwise_keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    if !otherwise_keywords.is_empty() || otherwise_arguments.len() != 1 {
        return Err("otherwise(...) expects one positional expression".into());
    }
    // Walked from the outside in, so the branches come out backwards and are
    // turned around once at the end. Order is load-bearing: the first branch
    // that answers true is the one that decides the row.
    let mut branches: Vec<(&Expr, &Expr)> = Vec::new();
    let mut node = input;
    loop {
        let Expr::Method {
            input: condition,
            path,
            arguments: then_arguments,
            keyword_arguments: then_keywords,
        } = node
        else {
            return Err("otherwise(...) must follow when(...).then(...)".into());
        };
        if path.as_slice() != ["then"] || !then_keywords.is_empty() || then_arguments.len() != 1 {
            return Err("otherwise(...) must follow when(...).then(...)".into());
        }
        match condition.as_ref() {
            // The first `when` is a function; every later one is a method on
            // the branch before it. Reaching the function is the bottom.
            Expr::PolarsCall {
                name,
                arguments,
                keyword_arguments,
            } if name == "when" => {
                if !keyword_arguments.is_empty() || arguments.len() != 1 {
                    return Err("when(...) expects one positional condition".into());
                }
                branches.push((&arguments[0], &then_arguments[0]));
                break;
            }
            Expr::Method {
                input: earlier,
                path,
                arguments,
                keyword_arguments,
            } if path.as_slice() == ["when"] => {
                if !keyword_arguments.is_empty() || arguments.len() != 1 {
                    return Err("when(...) expects one positional condition".into());
                }
                branches.push((&arguments[0], &then_arguments[0]));
                node = earlier;
            }
            _ => return Err("then(...) must follow when(...)".into()),
        }
    }
    branches.reverse();
    let otherwise = otherwise_arguments[0].to_polars(document)?;
    let compiled = |branch: &(&Expr, &Expr)| -> Result<(pl::Expr, pl::Expr), String> {
        Ok((branch.0.to_polars(document)?, branch.1.to_polars(document)?))
    };
    let (first, rest) = branches
        .split_first()
        .expect("the loop pushes before it breaks");
    let (condition, value) = compiled(first)?;
    let started = pl::when(condition).then(value);
    let Some((second, rest)) = rest.split_first() else {
        return Ok(started.otherwise(otherwise));
    };
    // Polars types the first branch and the ones after it differently, so
    // the second is taken by hand and the rest fold onto what it returns.
    let (condition, value) = compiled(second)?;
    let mut chain = started.when(condition).then(value);
    for branch in rest {
        let (condition, value) = compiled(branch)?;
        chain = chain.when(condition).then(value);
    }
    Ok(chain.otherwise(otherwise))
}

/// Reductions are one formula family even though Polars implements them as
/// separate methods. Keeping them together also gives the profile drawer one
/// place to define the exact formula behind each visible aggregate.
fn compile_reducing_method(
    input: &pl::Expr,
    path: &[String],
    args: &[pl::Expr],
    keyword_arguments: &[(String, Expr)],
) -> Option<Result<pl::Expr, String>> {
    let [name] = path else { return None };
    if !matches!(
        name.as_str(),
        "sum" | "quantile" | "mean" | "min" | "max" | "count" | "len" | "null_count"
    ) {
        return None;
    }
    if !keyword_arguments.is_empty() {
        return Some(Err(format!(
            ".{name} does not accept these keyword arguments"
        )));
    }
    Some(match name.as_str() {
        "sum" => Ok(input.clone().sum()),
        // Quantile carries a strategy enum the generic binding generator
        // cannot infer. Linear is also what the visible profile computes.
        "quantile" => args
            .first()
            .cloned()
            .map(|fraction| input.clone().quantile(fraction, pl::QuantileMethod::Linear))
            .ok_or_else(|| ".quantile expects an argument".into()),
        "mean" => Ok(input.clone().mean()),
        "min" => Ok(input.clone().min()),
        "max" => Ok(input.clone().max()),
        "count" => Ok(input.clone().count()),
        "len" => Ok(input.clone().len()),
        "null_count" => Ok(input.clone().null_count()),
        _ => unreachable!(),
    })
}

// Both defaults are the ones a person means by the word. "Between" includes
// its ends unless told otherwise, and a null is not a member of a set.
fn compile_is_between(
    input: pl::Expr,
    args: &[pl::Expr],
    keyword_arguments: &[(String, Expr)],
) -> Result<pl::Expr, String> {
    let closed = match keyword_argument(keyword_arguments, "closed") {
        None => pl::ClosedInterval::Both,
        Some(Expr::String { value }) => match value.as_str() {
            "both" => pl::ClosedInterval::Both,
            "left" => pl::ClosedInterval::Left,
            "right" => pl::ClosedInterval::Right,
            "none" => pl::ClosedInterval::None,
            other => {
                return Err(format!(
                    "closed={other} is not one of both, left, right, or none"
                ));
            }
        },
        Some(_) => return Err("closed= expects one of both, left, right, or none".into()),
    };
    if keyword_arguments
        .iter()
        .any(|(keyword, _)| keyword != "closed")
    {
        return Err(".is_between only accepts the keyword argument closed".into());
    }
    Ok(input.is_between(args[0].clone(), args[1].clone(), closed))
}

fn compile_is_in(
    input: pl::Expr,
    arguments: &[Expr],
    document: &Document,
) -> Result<pl::Expr, String> {
    if arguments.len() != 1 {
        return Err(".is_in expects one list of values".into());
    }
    // A bare list is the point of this call, so it is passed whole rather
    // than flattened into the argument run the way a horizontal function's
    // would be.
    Ok(input.is_in(
        match &arguments[0] {
            Expr::List { items } => pl::concat_list(
                items
                    .iter()
                    .map(|item| item.to_polars(document))
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|error| error.to_string())?,
            // A canvas list and a column of another frame both arrive as a
            // whole series. Polars calls a bare series here ambiguous — it
            // cannot tell "is this row in that list" from "match these up
            // one by one" — and imploding says the first, which is the only
            // reading that makes sense of the word `in`.
            other @ (Expr::Series { .. } | Expr::ForeignColumn { .. }) => {
                other.to_polars(document)?.implode(true)
            }
            other => other.to_polars(document)?,
        },
        false,
    ))
}

// Where a number lands between two others, as a fraction.
//
// Written here rather than left to the reader because the long form names
// its column three times -- `(x - x.min()) / (x.max() - x.min())` -- and
// because it is what a colour scale reads. A scale paints position, not
// value, so the mapping from a column onto that position belongs in the
// formula where it can be seen and edited: pinning the ends, clipping
// outliers with `.clip`, taking a log first, or substituting a value from
// another column are all then ordinary edits rather than settings that would
// each need a box.
//
// Ends left out are the column's own smallest and largest, computed over
// every row the way any aggregate here is.
fn compile_normalize(
    input: pl::Expr,
    args: &[pl::Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    let centre = keyword_argument(keyword_arguments, "center")
        .or_else(|| keyword_argument(keyword_arguments, "centre"));
    if centre.is_some() && !args.is_empty() {
        return Err(".normalize takes either two ends or a center, not both".into());
    }
    if centre.is_none() && !keyword_arguments.is_empty() {
        return Err(".normalize only accepts a center= keyword".into());
    }
    let value = input.clone().cast(pl::DataType::Float64);
    if let Some(centre) = centre {
        // A diverging scale: the centre lands halfway and the two
        // directions away from it get equal room, so the colour at the
        // middle means that number rather than the middle of whatever
        // range the rows happen to cover.
        let centre = centre.to_polars(document)?.cast(pl::DataType::Float64);
        let reach = (value.clone() - centre.clone()).abs().max();
        return Ok(pl::when(reach.clone().eq(pl::lit(0.0)))
            .then(pl::lit(0.5))
            .otherwise(pl::lit(0.5) + (value - centre) / (reach * pl::lit(2.0))));
    }
    let (low, high) = match args.len() {
        0 => (value.clone().min(), value.clone().max()),
        2 => (
            args[0].clone().cast(pl::DataType::Float64),
            args[1].clone().cast(pl::DataType::Float64),
        ),
        _ => return Err(".normalize expects no arguments, or a low and a high".into()),
    };
    // Every row the same number: there is no range to place them on, so
    // they sit in the middle rather than at an end.
    Ok(pl::when(high.clone().eq(low.clone()))
        .then(pl::lit(0.5))
        .otherwise((value - low.clone()) / (high - low)))
}

fn compile_rolling_aggregate(
    input: pl::Expr,
    kind: &str,
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
) -> Result<pl::Expr, String> {
    let window = arguments
        .first()
        .or_else(|| keyword_argument(keyword_arguments, "window_size"))
        .ok_or_else(|| format!(".{kind} requires window_size"))?;
    let window_size: usize = literal_integer(window, "window_size")?
        .try_into()
        .map_err(|_| "window_size must be positive")?;
    let min_periods = keyword_argument(keyword_arguments, "min_periods")
        .map(|value| literal_integer(value, "min_periods"))
        .transpose()?
        .map(usize::try_from)
        .transpose()
        .map_err(|_| "min_periods must be non-negative")?
        .unwrap_or(1);
    let center = match keyword_argument(keyword_arguments, "center") {
        Some(Expr::Boolean { value }) => *value,
        Some(_) => return Err("center must be True or False".into()),
        None => false,
    };
    let options = pl::RollingOptionsFixedWindow {
        window_size,
        min_periods,
        center,
        ..Default::default()
    };
    Ok(match kind {
        "rolling_mean" => input.rolling_mean(options),
        "rolling_sum" => input.rolling_sum(options),
        "rolling_min" => input.rolling_min(options),
        _ => input.rolling_max(options),
    })
}

fn compile_polars_method(
    input: pl::Expr,
    path: &[String],
    arguments: &[Expr],
    keyword_arguments: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    // The categorical string methods in Polars currently dispatch against
    // the physical column at execution time and unwrap its category mapping.
    // A categorical column read from Parquet can legitimately arrive there
    // as String even though the scan schema still describes it as Enum; the
    // unwrap then aborts the process instead of returning a query error.
    //
    // These four methods ask questions about the labels, not their declared
    // order or physical codes. Reading the labels as text is therefore the
    // same operation, and it also makes the plan safe across the artifact
    // boundary. Keep get_categories and physical on the categorical
    // namespace because those methods really do inspect the mapping.
    if matches!(
        path.iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["cat", "len_bytes"] | ["cat", "len_chars"] | ["cat", "starts_with"] | ["cat", "ends_with"]
    ) {
        let text_path = ["str".to_string(), path[1].clone()];
        return compile_polars_method(
            input.cast(pl::DataType::String),
            &text_path,
            arguments,
            keyword_arguments,
            document,
        );
    }
    let path_string = path.join(".");
    let args = flatten_polars_arguments(arguments, document)?;
    let expected_arguments = match path_string.as_str() {
        "abs" | "sign" | "sqrt" | "cbrt" | "exp" | "log1p" | "floor" | "ceil" | "sin" | "cos"
        | "tan" | "cot" | "arcsin" | "arccos" | "arctan" | "sinh" | "cosh" | "tanh" | "arcsinh"
        | "arccosh" | "arctanh" | "degrees" | "radians" | "is_null" | "is_not_null" | "sum"
        | "mean" | "min" | "max" | "count" | "len" | "null_count" | "dt.year" | "dt.iso_year"
        | "dt.quarter" | "dt.month" | "dt.week" | "dt.weekday" | "dt.ordinal_day"
        | "dt.is_leap_year" | "dt.days_in_month" | "dt.day" | "dt.date" | "dt.month_start"
        | "dt.month_end" | "str.to_uppercase" | "str.to_lowercase" => Some(0),
        "pow" | "round_sig_figs" | "clip_min" | "clip_max" | "floor_div" | "arctan2"
        | "fill_null" | "shift" | "filter" | "quantile" | "dt.offset_by" | "str.contains" => {
            Some(1)
        }
        "clip" | "is_between" => Some(2),
        _ => None,
    };
    if let Some(expected) = expected_arguments
        && arguments.len() != expected
    {
        return Err(format!(
            ".{path_string} expects {expected} argument{}",
            if expected == 1 { "" } else { "s" }
        ));
    }
    let no_keywords = || {
        if keyword_arguments.is_empty() {
            Ok(())
        } else {
            Err(format!(
                ".{path_string} does not accept these keyword arguments"
            ))
        }
    };
    let one = || {
        args.first()
            .cloned()
            .ok_or_else(|| format!(".{path_string} expects an argument"))
    };
    if let Some(reduction) = compile_reducing_method(&input, path, &args, keyword_arguments) {
        return reduction;
    }
    match path
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        // Hand-written for the same reason `date(...)` is: the generator
        // binds arguments it can map from the AST with certainty, and both
        // of these carry one it cannot. `is_between` takes a
        // `ClosedInterval`, and `is_in` a bare `bool` whose meaning is not
        // in its position. Deferring them was right; leaving the two
        // commonest filters in a spreadsheet unwritable was not.
        //
        // Both defaults are the ones a person means by the word. "Between"
        // includes its ends unless told otherwise, and a null is not a
        // member of a set.
        ["is_between"] => compile_is_between(input, &args, keyword_arguments),
        ["is_in"] => {
            no_keywords()?;
            compile_is_in(input, arguments, document)
        }
        ["filter"] => {
            no_keywords()?;
            let predicate = arguments
                .first()
                .ok_or(".filter expects a true/false predicate")?;
            if let Some(data_type) = predicate.declared_type(document)
                && data_type != DataType::Boolean
            {
                return Err(".filter expects a true/false predicate".into());
            }
            // `filter` changes a vertical expression's length. Taking the
            // predicate from the parsed argument rather than `args` matters:
            // `args` intentionally flattens written lists for horizontal
            // methods, whereas a predicate is one expression and a list is
            // a positional alignment error, not several predicates.
            Ok(input.filter(predicate.to_polars(document)?))
        }
        ["abs"] => {
            no_keywords()?;
            Ok(input.abs())
        }
        ["sign"] => {
            no_keywords()?;
            Ok(input.sign())
        }
        ["sqrt"] => {
            no_keywords()?;
            Ok(input.sqrt())
        }
        ["cbrt"] => {
            no_keywords()?;
            Ok(input.cbrt())
        }
        ["pow"] => {
            no_keywords()?;
            Ok(input.pow(one()?))
        }
        ["exp"] => {
            no_keywords()?;
            Ok(input.exp())
        }
        ["log1p"] => {
            no_keywords()?;
            Ok(input.log1p())
        }
        ["log"] => {
            no_keywords()?;
            Ok(input.log(
                args.first()
                    .cloned()
                    .unwrap_or_else(|| pl::lit(std::f64::consts::E)),
            ))
        }
        ["floor"] => {
            no_keywords()?;
            Ok(input.floor())
        }
        ["ceil"] => {
            no_keywords()?;
            Ok(input.ceil())
        }
        ["round"] => {
            let decimals = arguments
                .first()
                .or_else(|| keyword_argument(keyword_arguments, "decimals"))
                .map(|value| literal_integer(value, "round decimals"))
                .transpose()?
                .unwrap_or(0);
            Ok(input.round(
                decimals
                    .try_into()
                    .map_err(|_| "round decimals must be non-negative")?,
                pl::RoundMode::HalfToEven,
            ))
        }
        ["round_sig_figs"] => {
            no_keywords()?;
            Ok(input.round_sig_figs(literal_integer(
                arguments.first().ok_or("round_sig_figs expects digits")?,
                "significant digits",
            )? as i32))
        }
        ["truncate"] => {
            no_keywords()?;
            let decimals = arguments
                .first()
                .map(|value| literal_integer(value, "truncate decimals"))
                .transpose()?
                .unwrap_or(0);
            Ok(input.truncate(
                decimals
                    .try_into()
                    .map_err(|_| "truncate decimals must be non-negative")?,
            ))
        }
        // Where a number lands between two others, as a fraction.
        //
        // Written here rather than left to the reader because the long form
        // names its column three times -- `(x - x.min()) / (x.max() -
        // x.min())` -- and because it is what a colour scale reads. A scale
        // paints position, not value, so the mapping from a column onto that
        // position belongs in the formula where it can be seen and edited:
        // pinning the ends, clipping outliers with `.clip`, taking a log
        // first, or substituting a value from another column are all then
        // ordinary edits rather than settings that would each need a box.
        //
        // Ends left out are the column's own smallest and largest, computed
        // over every row the way any aggregate here is.
        ["normalize"] => compile_normalize(input, &args, keyword_arguments, document),
        ["clip"] => {
            no_keywords()?;
            if args.len() != 2 {
                return Err(".clip expects minimum and maximum".into());
            }
            Ok(input.clip(args[0].clone(), args[1].clone()))
        }
        ["clip_min"] => {
            no_keywords()?;
            Ok(input.clip_min(one()?))
        }
        ["clip_max"] => {
            no_keywords()?;
            Ok(input.clip_max(one()?))
        }
        ["floor_div"] => {
            no_keywords()?;
            Ok(input.floor_div(one()?))
        }
        ["sin"] => {
            no_keywords()?;
            Ok(input.sin())
        }
        ["cos"] => {
            no_keywords()?;
            Ok(input.cos())
        }
        ["tan"] => {
            no_keywords()?;
            Ok(input.tan())
        }
        ["cot"] => {
            no_keywords()?;
            Ok(input.cot())
        }
        ["arcsin"] => {
            no_keywords()?;
            Ok(input.arcsin())
        }
        ["arccos"] => {
            no_keywords()?;
            Ok(input.arccos())
        }
        ["arctan"] => {
            no_keywords()?;
            Ok(input.arctan())
        }
        ["arctan2"] => {
            no_keywords()?;
            Ok(input.arctan2(one()?))
        }
        ["sinh"] => {
            no_keywords()?;
            Ok(input.sinh())
        }
        ["cosh"] => {
            no_keywords()?;
            Ok(input.cosh())
        }
        ["tanh"] => {
            no_keywords()?;
            Ok(input.tanh())
        }
        ["arcsinh"] => {
            no_keywords()?;
            Ok(input.arcsinh())
        }
        ["arccosh"] => {
            no_keywords()?;
            Ok(input.arccosh())
        }
        ["arctanh"] => {
            no_keywords()?;
            Ok(input.arctanh())
        }
        ["degrees"] => {
            no_keywords()?;
            Ok(input.degrees())
        }
        ["radians"] => {
            no_keywords()?;
            Ok(input.radians())
        }
        ["is_null"] => {
            no_keywords()?;
            Ok(input.is_null())
        }
        ["is_not_null"] => {
            no_keywords()?;
            Ok(input.is_not_null())
        }
        ["fill_null"] => {
            no_keywords()?;
            Ok(input.fill_null(one()?))
        }
        ["shift"] => {
            no_keywords()?;
            Ok(input.shift(one()?))
        }
        ["over"] => {
            no_keywords()?;
            input.over(&args).map_err(|error| error.to_string())
        }
        ["rolling_mean"] | ["rolling_sum"] | ["rolling_min"] | ["rolling_max"] => {
            compile_rolling_aggregate(input, path[0].as_str(), arguments, keyword_arguments)
        }
        // Every calendar part is widened to the formula language's one
        // integer type. Polars answers these in the narrowest dtype that
        // fits a calendar — day is eight bits — and narrow arithmetic
        // *wraps*: `38 * day` quietly answered 116 in a real document,
        // which is not an error anyone can see, just a wrong number. A
        // spreadsheet number does not have a width, so none of these get
        // to keep one.
        ["dt", "year"] => {
            no_keywords()?;
            Ok(input.dt().year().cast(pl::DataType::Int64))
        }
        ["dt", "iso_year"] => {
            no_keywords()?;
            Ok(input.dt().iso_year().cast(pl::DataType::Int64))
        }
        ["dt", "quarter"] => {
            no_keywords()?;
            Ok(input.dt().quarter().cast(pl::DataType::Int64))
        }
        ["dt", "month"] => {
            no_keywords()?;
            Ok(input.dt().month().cast(pl::DataType::Int64))
        }
        ["dt", "week"] => {
            no_keywords()?;
            Ok(input.dt().week().cast(pl::DataType::Int64))
        }
        ["dt", "weekday"] => {
            no_keywords()?;
            Ok(input.dt().weekday().cast(pl::DataType::Int64))
        }
        ["dt", "day"] => {
            no_keywords()?;
            Ok(input.dt().day().cast(pl::DataType::Int64))
        }
        ["dt", "ordinal_day"] => {
            no_keywords()?;
            Ok(input.dt().ordinal_day().cast(pl::DataType::Int64))
        }
        ["dt", "is_leap_year"] => {
            no_keywords()?;
            Ok(input.dt().is_leap_year())
        }
        ["dt", "days_in_month"] => {
            no_keywords()?;
            Ok(input.dt().days_in_month().cast(pl::DataType::Int64))
        }
        ["dt", "date"] => {
            no_keywords()?;
            Ok(input.dt().date())
        }
        ["dt", "month_start"] => {
            no_keywords()?;
            Ok(input.dt().month_start())
        }
        ["dt", "month_end"] => {
            no_keywords()?;
            Ok(input.dt().month_end())
        }
        ["dt", "offset_by"] => {
            no_keywords()?;
            let argument = arguments.first().ok_or(".dt.offset_by expects offset")?;
            Ok(input
                .dt()
                .offset_by(day_count_offset(argument, one()?, document)))
        }
        ["str", "to_uppercase"] => {
            no_keywords()?;
            Ok(input.str().to_uppercase())
        }
        ["str", "to_lowercase"] => {
            no_keywords()?;
            Ok(input.str().to_lowercase())
        }
        ["str", "contains"] => {
            let strict = match keyword_argument(keyword_arguments, "strict") {
                Some(Expr::Boolean { value }) => *value,
                Some(_) => return Err("strict must be True or False".into()),
                None => true,
            };
            Ok(input.str().contains(one()?, strict))
        }
        ["alias"] => {
            Err("The calculated-column name supplies the Polars alias automatically".into())
        }
        ["when"] | ["then"] | ["otherwise"] => {
            Err("when/then/otherwise must be used as one complete chain".into())
        }
        _ => crate::formula::generated_bindings::compile_generated_method(
            input,
            path,
            arguments,
            keyword_arguments,
            document,
        )
        .unwrap_or_else(|| Err(format!("Unsupported Polars method ‘.{path_string}’"))),
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
    pub(crate) fn polars_methods_render_canonically_and_evaluate() {
        let mut store = demo_store();
        let frame_id = store
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame.id.clone()),
                _ => None,
            })
            .unwrap();
        let view = store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: "Quantity squared".into(),
                formula: "`Quantity`.pow(2).round(0)".into(),
                after_column_id: None,
            })
            .unwrap();
        let frame = view
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
                _ => None,
            })
            .unwrap();
        let column_id = frame
            .columns
            .iter()
            .find(|column| column.name == "Quantity squared")
            .unwrap()
            .id
            .clone();
        // Read back through `formulas`, which is where a rendered column
        // formula lives on a frame that owns its rows: the formula is on the
        // column itself rather than in a wrangle step, the same way the
        // demo's own Total column holds one. Rendering is the assertion --
        // an expression that parses and then writes itself back differently
        // is one somebody's document quietly rewrites every time it is read.
        let rendered = &view.computed_frames[&frame_id].formulas[&column_id];
        assert_eq!(rendered, "`Quantity`.pow(2).round(0)");
        let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
        let output = page
            .columns
            .iter()
            .position(|column| column.id == column_id)
            .unwrap();
        let values = page
            .rows
            .iter()
            .map(|row| row[output].parse::<f64>().ok())
            .collect::<Vec<_>>();
        assert_eq!(values, vec![Some(9.0), Some(25.0), Some(4.0)]);

        let wrong_arity = store.apply(Operation::AddComputedColumn {
            frame_id,
            name: "Broken".into(),
            formula: "`Quantity`.sqrt(1, 2)".into(),
            after_column_id: None,
        });
        assert!(
            matches!(wrong_arity, Err(CoreError::Formula(message)) if message.contains("0 arguments"))
        );

        let catalog = formula_function_catalog();
        assert_eq!(
            catalog.len(),
            crate::formula::catalog::POLARS_FORMULA_FUNCTIONS.len()
                + formula::generated_bindings::GENERATED_FORMULA_FUNCTIONS.len()
        );
        assert!(catalog.iter().any(|function| {
            function.id == "expr.pow" && function.aliases.contains(&"power".to_string())
        }));
        assert!(
            catalog
                .iter()
                .any(|function| function.id == "str.strip_chars")
        );
    }
}
