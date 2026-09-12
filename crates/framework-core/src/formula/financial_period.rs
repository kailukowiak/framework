//! Monthly period indexes and the prior-period self-join.
//!
//! A finance model is a monthly series, and "the period before" is a fact
//! about time, not about row position. `period_index` turns a date into a
//! whole number — fiscal months since year zero — so every period offset is
//! exact integer arithmetic rather than date math. `prior` then reads the
//! value a column held at an earlier index by joining the frame to itself
//! on `index - n` within the declared partitions, the way `lookup` joins a
//! dictionary: a deleted or shuffled row changes which periods exist, never
//! which row is silently read instead.
//!
//! The declaration lives on the frame (see `FramePeriod`), the calendar on
//! the calls: `fy_start` names the month a fiscal year starts on, defaulting
//! to January. A fuller calendar object will supply that default later; the
//! index arithmetic here will not have to change.
use crate::*;
use polars::prelude as pl;

/// Monthly index of a date expression: fiscal years of twelve months from
/// `fy_start`, times twelve plus the zero-based fiscal month.
pub(super) fn period_index_expr(date: pl::Expr, fy_start: i64) -> pl::Expr {
    let year = date.clone().dt().year().cast(pl::DataType::Int32);
    let month = date.dt().month().cast(pl::DataType::Int32);
    let start = pl::lit(fy_start as i32);
    let fiscal_year = pl::when(month.clone().gt_eq(start.clone()))
        .then(year.clone())
        .otherwise(year - pl::lit(1));
    // month - start runs -11 through 11, so shifting by a full year keeps
    // the remainder exact: no negative dividends reach the modulo.
    let month_offset = (month - start + pl::lit(12)) % pl::lit(12);
    (fiscal_year * pl::lit(12) + month_offset).cast(pl::DataType::Int32)
}

pub(super) fn compile_period_index(
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    let (date, fy_start) = period_arguments(arguments, keywords, "period_index", document)?;
    Ok(period_index_expr(date.to_polars(document)?, fy_start))
}

fn period_arguments<'a>(
    arguments: &'a [Expr],
    keywords: &[(String, Expr)],
    name: &str,
    document: &Document,
) -> Result<(&'a Expr, i64), String> {
    if arguments.len() > 2 {
        return Err(format!("{name} expects at most 2 arguments"));
    }
    let date = arguments
        .first()
        .ok_or_else(|| format!("{name} expects the date to count from"))?;
    let mut fy_start = arguments.get(1);
    for (key, value) in keywords {
        if key != "fy_start" {
            return Err(format!("{name} has no argument ‘{key}’"));
        }
        if fy_start.replace(value).is_some() {
            return Err(format!("{name}: ‘fy_start’ was supplied twice"));
        }
    }
    Ok((date, fiscal_year_start(fy_start, document)?))
}

pub(super) fn fiscal_year_start(
    argument: Option<&Expr>,
    document: &Document,
) -> Result<i64, String> {
    let start = match argument {
        None => 1,
        Some(expression) => integer_literal(expression, document).map_err(|_| {
            "fy_start must be a whole month number from 1 to 12, with 1 meaning January".to_string()
        })?,
    };
    if !(1..=12).contains(&start) {
        return Err(
            "fy_start must be a whole month number from 1 to 12, with 1 meaning January".into(),
        );
    }
    Ok(start)
}

/// A whole number written in the formula or held by a named value.
/// Plan-time joins need the offset up front, so `n` cannot be a column.
pub(super) fn integer_literal(expression: &Expr, document: &Document) -> Result<i64, String> {
    match expression {
        Expr::Integer { value } => Ok(*value),
        Expr::Value { object_id } => match document.object(object_id) {
            Ok(DataObject::Value(value)) => document
                .effective_value_raw(value)
                .trim()
                .parse::<i64>()
                .map_err(|_| "expected a whole number".to_string()),
            _ => Err("expected a whole number".to_string()),
        },
        _ => Err("expected a whole number".to_string()),
    }
}

/// One `prior(...)` call: the value expression, the whole-period offset,
/// and the fiscal year start it counts in.
struct PriorCall<'a> {
    expression: &'a Expr,
    n: i64,
    fy_start: i64,
}

fn is_prior_call(expression: &Expr) -> bool {
    match expression {
        Expr::PolarsCall { name, .. } => name.eq_ignore_ascii_case("prior"),
        Expr::Method { path, .. } => {
            matches!(path.as_slice(), [namespace, name] if namespace.eq_ignore_ascii_case("finance") && name.eq_ignore_ascii_case("prior"))
        }
        _ => false,
    }
}

fn prior_call<'a>(expression: &'a Expr, document: &Document) -> Result<PriorCall<'a>, String> {
    let (inner, arguments, keywords) = match expression {
        Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } if name.eq_ignore_ascii_case("prior") => {
            (None, arguments.as_slice(), keyword_arguments.as_slice())
        }
        Expr::Method {
            input,
            path,
            arguments,
            keyword_arguments,
        } if matches!(path.as_slice(), [namespace, name] if namespace.eq_ignore_ascii_case("finance") && name.eq_ignore_ascii_case("prior")) => {
            (
                Some(input.as_ref()),
                arguments.as_slice(),
                keyword_arguments.as_slice(),
            )
        }
        _ => unreachable!("is_prior_call only collects prior calls"),
    };
    // The receiver stands in for `expr`, so the remaining positionals are
    // `n` then `fy_start` either way they were written.
    let mut slots: [Option<&Expr>; 3] = [None, None, None];
    let mut positional = Vec::with_capacity(3);
    if let Some(input) = inner {
        positional.push(input);
    }
    positional.extend(arguments.iter());
    if positional.len() > 3 {
        return Err("prior expects at most 3 arguments".into());
    }
    for (slot, argument) in slots.iter_mut().zip(positional) {
        *slot = Some(argument);
    }
    for (key, value) in keywords {
        let index = match key.as_str() {
            "expr" => 0,
            "n" => 1,
            "fy_start" => 2,
            _ => return Err(format!("prior has no argument ‘{key}’")),
        };
        if slots[index].replace(value).is_some() {
            return Err(format!("prior: ‘{key}’ was supplied twice"));
        }
    }
    let expression = slots[0].ok_or_else(|| "prior expects the expression to read".to_string())?;
    let n = match slots[1] {
        None => 1,
        Some(argument) => integer_literal(argument, document)
            .map_err(|_| "prior's n must be a whole number of periods".to_string())?,
    };
    if n < 1 {
        return Err("prior's n must be at least 1".into());
    }
    Ok(PriorCall {
        expression,
        n,
        fy_start: fiscal_year_start(slots[2], document)?,
    })
}

/// Every distinct `prior` call inside `columns`, in first-seen order. Two
/// identical calls share one join.
fn prior_calls(columns: &[DerivedExpression]) -> Vec<&Expr> {
    let mut calls: Vec<&Expr> = Vec::new();
    for column in columns {
        column.expression.walk(&mut |expression| {
            if is_prior_call(expression) && !calls.contains(&expression) {
                calls.push(expression);
            }
        });
    }
    calls
}

/// `expression` with each prior call replaced by the column its join
/// produced. Only the calls are lifted out; everything around them still
/// compiles through `to_polars`.
fn substitute(expression: &Expr, calls: &[&Expr], outputs: &[String]) -> Expr {
    if let Some(index) = calls.iter().position(|call| *call == expression) {
        return Expr::Column {
            column_id: outputs[index].clone(),
        };
    }
    let boxed = |inner: &Expr| Box::new(substitute(inner, calls, outputs));
    let each = |items: &[Expr]| {
        items
            .iter()
            .map(|item| substitute(item, calls, outputs))
            .collect()
    };
    let keyed = |items: &[(String, Expr)]| {
        items
            .iter()
            .map(|(keyword, item)| (keyword.clone(), substitute(item, calls, outputs)))
            .collect()
    };
    match expression {
        Expr::List { items } => Expr::List { items: each(items) },
        Expr::Negate { expression } => Expr::Negate {
            expression: boxed(expression),
        },
        Expr::Not { expression } => Expr::Not {
            expression: boxed(expression),
        },
        Expr::Binary {
            operator,
            left,
            right,
        } => Expr::Binary {
            operator: *operator,
            left: boxed(left),
            right: boxed(right),
        },
        Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } => Expr::PolarsCall {
            name: name.clone(),
            arguments: each(arguments),
            keyword_arguments: keyed(keyword_arguments),
        },
        Expr::Method {
            input,
            path,
            arguments,
            keyword_arguments,
        } => Expr::Method {
            input: boxed(input),
            path: path.clone(),
            arguments: each(arguments),
            keyword_arguments: keyed(keyword_arguments),
        },
        other => other.clone(),
    }
}

/// Joins each prior read in `columns` into `plan` and returns the columns
/// rewritten to read the joined answers, plus the answer columns' names so
/// the caller can drop them once the step has used them.
///
/// Each side of the join is the same incoming plan: the left asks for
/// `index - n`, the right offers each row's own index, and a left join
/// keeps every row — a first period with nothing before it reads blank,
/// which is what "that month is missing" means rather than an error.
pub(crate) fn join_prior_periods(
    document: &Document,
    frame_id: &str,
    plan: pl::LazyFrame,
    columns: &[DerivedExpression],
) -> Result<(pl::LazyFrame, Vec<DerivedExpression>, Vec<String>), String> {
    let calls = prior_calls(columns);
    if calls.is_empty() {
        return Ok((plan, columns.to_vec(), Vec::new()));
    }
    let frame = document
        .frame(frame_id)
        .map_err(|error| error.to_string())?;
    let mut plan = plan;
    let mut outputs = Vec::with_capacity(calls.len());
    for (index, call) in calls.iter().enumerate() {
        let prior = prior_call(call, document)?;
        let output = format!("__framework_prior_{index}");
        plan = join_prior(document, frame, plan, &prior, &output)?;
        outputs.push(output);
    }
    let rewritten = columns
        .iter()
        .map(|column| DerivedExpression {
            output_column_id: column.output_column_id.clone(),
            expression: substitute(&column.expression, &calls, &outputs),
        })
        .collect();
    Ok((plan, rewritten, outputs))
}

fn join_prior(
    document: &Document,
    frame: &FrameObject,
    mut plan: pl::LazyFrame,
    prior: &PriorCall<'_>,
    output: &str,
) -> Result<pl::LazyFrame, String> {
    let period = frame.period.as_ref().ok_or_else(|| {
        format!(
            "‘{}’ has no declared period column, so prior cannot tell which row is the previous period. Declare a Date column as the period first.",
            frame.name
        )
    })?;
    let column_name = |column_id: &str| {
        frame
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| column_id.to_string())
    };
    // The declaration names stored columns; the chain may have dropped one
    // upstream. A bare Polars "column not found" would not say which frame
    // to fix, so check the schema while the frame is still in hand.
    let schema = plan.collect_schema().map_err(|error| error.to_string())?;
    for column_id in period
        .partition_column_ids
        .iter()
        .chain(std::iter::once(&period.column_id))
    {
        if !schema.contains(column_id) {
            return Err(format!(
                "‘{}’ declares ‘{}’ as its period, but the chain no longer carries that column above this step.",
                frame.name,
                column_name(column_id)
            ));
        }
    }
    let want = format!("{output}_want");
    let have = format!("{output}_have");
    let value = format!("{output}_value");
    let index = period_index_expr(pl::col(&period.column_id), prior.fy_start);
    let mut keys: Vec<pl::Expr> = period.partition_column_ids.iter().map(pl::col).collect();
    keys.push(pl::col(&want));
    let mut have_keys: Vec<pl::Expr> = period.partition_column_ids.iter().map(pl::col).collect();
    have_keys.push(pl::col(&have));
    let left = plan
        .clone()
        .with_columns([(index.clone() - pl::lit(prior.n as i32)).alias(want.as_str())]);
    let right = plan.select(
        period
            .partition_column_ids
            .iter()
            .map(pl::col)
            .chain([
                index.alias(have.as_str()),
                prior.expression.to_polars(document)?.alias(value.as_str()),
            ])
            .collect::<Vec<_>>(),
    );
    // Many-to-one is the backstop behind the declaration's uniqueness
    // check: duplicate periods cannot silently multiply rows here. Both
    // keys are kept under their own names — unlike an equi-join on one
    // name, nothing here coalesces — so the helpers below can drop them.
    let mut arguments = pl::JoinArgs::new(pl::JoinType::Left);
    arguments.validation = pl::JoinValidation::ManyToOne;
    arguments.maintain_order = pl::MaintainOrderJoin::Left;
    arguments.coalesce = pl::JoinCoalesce::KeepColumns;
    let joined = left.join(right, keys, have_keys, arguments);
    Ok(joined
        .with_columns([pl::col(value.as_str()).alias(output)])
        .drop(pl::cols([want.as_str(), have.as_str(), value.as_str()])))
}
