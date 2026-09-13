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
//! the calls: an optional `calendar` names a document calendar and an
//! optional `fy_start` names the month a fiscal year starts on. With
//! neither written, every period-relative call reads the document default
//! calendar — the same calendar `fiscal_period` reads — so one document
//! cannot number its periods two ways.
//!
//! `PeriodBasis` is that numbering, resolved once per call and shared by
//! every period-relative function: month arithmetic under a calendar-month
//! pattern, and the week table under a retail pattern, where a period is a
//! block of weeks rather than a month.
use crate::*;
use polars::prelude as pl;

use super::financial_calendar::{broadcast_dates, fiscal_start, int_series, resolve_calendar};
use crate::model::calendar::{ResolvedCalendar, WeekPattern};

/// The numbering one period-relative call counts in. Every call that joins
/// on an index resolves one of these and hands it to both sides of the
/// join, so the two sides can never disagree about where a year turns.
#[derive(Clone)]
pub(super) struct PeriodBasis {
    calendar: ResolvedCalendar,
    fy_start: i64,
}

impl PeriodBasis {
    /// The basis a call counts in: the named calendar or the document
    /// default, with `fy_start` overriding its year start.
    pub(super) fn resolve(
        name: &str,
        fy_start: Option<&Expr>,
        calendar: Option<&Expr>,
        document: &Document,
    ) -> Result<PeriodBasis, String> {
        let calendar = resolve_calendar(document, calendar)?;
        if fy_start.is_some() && calendar.pattern != WeekPattern::Months {
            return Err(retail_fy_start_error(name));
        }
        let fy_start = fiscal_start(fy_start, &calendar, document)?;
        Ok(PeriodBasis { calendar, fy_start })
    }

    /// The period index of a date under this basis. Calendar months count
    /// by arithmetic; retail weeks count through the week table, so a date
    /// lands in the period `fiscal_period` reports for it rather than in
    /// the calendar month it happens to sit in.
    pub(super) fn index(&self, date: pl::Expr) -> pl::Expr {
        match self.calendar.pattern {
            WeekPattern::Months => period_index_expr(date, self.fy_start),
            _ => retail_period_index(date, self.calendar.clone()),
        }
    }
}

/// Why a retail calendar refuses `fy_start`. A week-pattern year starts the
/// day after the previous year's end rule fires — the Saturday nearest
/// January 31, say — so there is no month number to move it to: honouring
/// `fy_start` here would mean inventing a different calendar and quietly
/// answering about that one instead. Refusing says so once, in the same
/// words, for every call that takes both.
pub(super) fn retail_fy_start_error(name: &str) -> String {
    format!(
        "{name} cannot take fy_start together with a retail week calendar: the week table fixes where that year starts. Drop fy_start, or name a calendar-month calendar."
    )
}

/// A retail period index: twelve blocks a year, so `year * 12 + period - 1`
/// counts exactly the way month arithmetic does — one date at a time,
/// through the same `locate` the retail fiscal functions read.
fn retail_period_index(date: pl::Expr, calendar: ResolvedCalendar) -> pl::Expr {
    pl::apply_multiple(
        move |columns| {
            let length = columns.iter().map(|column| column.len()).max().unwrap_or(0);
            let raw = broadcast_dates("period_index", &columns[0], length)?;
            Ok(int_series(
                "period_index",
                raw.into_iter()
                    .map(|date| {
                        date.map(|date| {
                            let placed = calendar.locate(date);
                            placed.year * 12 + placed.period as i32 - 1
                        })
                    })
                    .collect(),
            ))
        },
        [date],
        |_, _| Ok(pl::Field::new("period_index".into(), pl::DataType::Int32)),
        false,
    )
}

/// Monthly index of a date expression: fiscal years of twelve months from
/// `fy_start`, times twelve plus the zero-based fiscal month.
fn period_index_expr(date: pl::Expr, fy_start: i64) -> pl::Expr {
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
    let (date, basis) = period_arguments(arguments, keywords, "period_index", document)?;
    Ok(basis.index(date.to_polars(document)?))
}

fn period_arguments<'a>(
    arguments: &'a [Expr],
    keywords: &'a [(String, Expr)],
    name: &str,
    document: &Document,
) -> Result<(&'a Expr, PeriodBasis), String> {
    let parameters = ["date", "fy_start", "calendar"];
    if arguments.len() > parameters.len() {
        return Err(format!("{name} expects at most 3 arguments"));
    }
    let mut slots: [Option<&Expr>; 3] = [None, None, None];
    for (slot, argument) in slots.iter_mut().zip(arguments) {
        *slot = Some(argument);
    }
    for (key, value) in keywords {
        let index = parameters
            .iter()
            .position(|parameter| parameter == key)
            .ok_or_else(|| format!("{name} has no argument ‘{key}’"))?;
        if slots[index].replace(value).is_some() {
            return Err(format!("{name}: ‘{key}’ was supplied twice"));
        }
    }
    let date = slots[0].ok_or_else(|| format!("{name} expects the date to count from"))?;
    Ok((
        date,
        PeriodBasis::resolve(name, slots[1], slots[2], document)?,
    ))
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
/// and the numbering it counts in.
pub(super) struct PriorCall<'a> {
    pub(super) expression: &'a Expr,
    pub(super) n: i64,
    pub(super) basis: PeriodBasis,
}

pub(super) fn is_prior_call(expression: &Expr) -> bool {
    match expression {
        // The `finance.` namespace spells the same call; only that
        // namespace lifts, so a dictionary method that happens to share
        // the name keeps compiling where it always did.
        Expr::PolarsCall { name, .. } => name
            .strip_prefix("finance.")
            .unwrap_or(name)
            .eq_ignore_ascii_case("prior"),
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
        } if name
            .strip_prefix("finance.")
            .unwrap_or(name)
            .eq_ignore_ascii_case("prior") =>
        {
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
    // `n`, `fy_start` then `calendar` either way they were written.
    let mut slots: [Option<&Expr>; 4] = [None, None, None, None];
    let mut positional = Vec::with_capacity(4);
    if let Some(input) = inner {
        positional.push(input);
    }
    positional.extend(arguments.iter());
    if positional.len() > 4 {
        return Err("prior expects at most 4 arguments".into());
    }
    for (slot, argument) in slots.iter_mut().zip(positional) {
        *slot = Some(argument);
    }
    for (key, value) in keywords {
        let index = match key.as_str() {
            "expr" => 0,
            "n" => 1,
            "fy_start" => 2,
            "calendar" => 3,
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
        basis: PeriodBasis::resolve("prior", slots[2], slots[3], document)?,
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
/// compiles through `to_polars`. Window aggregates reuse this with their
/// own call list, so a `prior` inside a `ytd` resolves first.
pub(super) fn substitute(expression: &Expr, calls: &[&Expr], outputs: &[String]) -> Expr {
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
/// rewritten to read the joined answers.
///
/// Every call derives its two sides from the pre-pass input snapshot and
/// joins its answer back by the declaration's natural keys, so ten prior
/// reads cost ten constant-size joins rather than ten doublings of the
/// accumulated plan. The answers are the only columns that survive each
/// join; the want/have helpers die inside it, so there is nothing left for
/// the caller to drop.
///
/// Each side of the join asks the same question: the left wants `index -
/// n`, the right offers each row's own index, and a left join keeps every
/// row — a first period with nothing before it reads blank, which is what
/// "that month is missing" means rather than an error.
pub(crate) fn join_prior_periods(
    document: &Document,
    frame_id: &str,
    mut plan: pl::LazyFrame,
    columns: &[DerivedExpression],
) -> Result<(pl::LazyFrame, Vec<DerivedExpression>, Vec<String>), String> {
    let calls = prior_calls(columns);
    if calls.is_empty() {
        return Ok((plan, columns.to_vec(), Vec::new()));
    }
    let frame = document
        .frame(frame_id)
        .map_err(|error| error.to_string())?;
    let schema = plan.collect_schema().map_err(|error| error.to_string())?;
    let base = plan;
    let mut acc = base.clone();
    let mut outputs = Vec::with_capacity(calls.len());
    for (index, call) in calls.iter().enumerate() {
        let prior = prior_call(call, document)?;
        let period = require_period(frame, "prior")?;
        check_period_columns(frame, period, &schema)?;
        let output = format!("__framework_prior_{index}");
        acc = join_prior(document, period, &base, acc, &prior, &output)?;
        outputs.push(output);
    }
    let rewritten = columns
        .iter()
        .map(|column| DerivedExpression {
            output_column_id: column.output_column_id.clone(),
            expression: substitute(&column.expression, &calls, &outputs),
        })
        .collect();
    Ok((acc, rewritten, outputs))
}

/// The frame's declared period, or an error naming the frame and the fix.
/// Every period-relative function refuses the same way: the declaration is
/// what makes "the previous period" mean anything, so without one there is
/// nothing to compute rather than something to guess.
pub(crate) fn require_period<'a>(
    frame: &'a FrameObject,
    name: &str,
) -> Result<&'a FramePeriod, String> {
    frame.period.as_ref().ok_or_else(|| {
        format!(
            "‘{}’ has no declared period column, so {name} cannot tell which row is the previous period. Declare a Date column as the period first.",
            frame.name
        )
    })
}

/// The declaration names stored columns; the chain may have dropped one
/// upstream. A bare Polars "column not found" would not say which frame
/// to fix, so check the schema while the frame is still in hand. The schema
/// is collected once per pre-pass over the input snapshot every join below
/// derives from.
pub(super) fn check_period_columns(
    frame: &FrameObject,
    period: &FramePeriod,
    schema: &pl::Schema,
) -> Result<(), String> {
    let column_name = |column_id: &str| {
        frame
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| column_id.to_string())
    };
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
    Ok(())
}

/// One prior read as a self-join on `index - n` within the declared
/// partitions. Both sides derive from the pre-pass snapshot, never from
/// the accumulation, and the answer joins back onto the accumulation by
/// the period and partition columns the declaration guarantees unique.
///
/// Many-to-one is the backstop behind the declaration's uniqueness check
/// on both joins: duplicate periods cannot silently multiply rows here.
/// Both keys are kept under their own names — unlike an equi-join on one
/// name, nothing here coalesces — so only the answer is selected out.
pub(super) fn join_prior(
    document: &Document,
    period: &FramePeriod,
    base: &pl::LazyFrame,
    acc: pl::LazyFrame,
    prior: &PriorCall<'_>,
    output: &str,
) -> Result<pl::LazyFrame, String> {
    let want = format!("{output}_want");
    let have = format!("{output}_have");
    let value = format!("{output}_value");
    let index = prior.basis.index(pl::col(&period.column_id));
    let mut keys: Vec<pl::Expr> = period.partition_column_ids.iter().map(pl::col).collect();
    keys.push(pl::col(&want));
    let mut have_keys: Vec<pl::Expr> = period.partition_column_ids.iter().map(pl::col).collect();
    have_keys.push(pl::col(&have));
    let left = base
        .clone()
        .with_columns([(index.clone() - pl::lit(prior.n as i32)).alias(want.as_str())]);
    let right = base.clone().select(
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
    let mut arguments = pl::JoinArgs::new(pl::JoinType::Left);
    arguments.validation = pl::JoinValidation::ManyToOne;
    arguments.maintain_order = pl::MaintainOrderJoin::Left;
    arguments.coalesce = pl::JoinCoalesce::KeepColumns;
    let paired = left.join(right, keys, have_keys, arguments);
    let mut natural: Vec<pl::Expr> = period.partition_column_ids.iter().map(pl::col).collect();
    natural.push(pl::col(&period.column_id));
    let answer = paired.select(
        natural
            .iter()
            .cloned()
            .chain([pl::col(value.as_str()).alias(output)])
            .collect::<Vec<_>>(),
    );
    let mut back = pl::JoinArgs::new(pl::JoinType::Left);
    back.validation = pl::JoinValidation::ManyToOne;
    back.maintain_order = pl::MaintainOrderJoin::Left;
    Ok(acc.join(answer, natural.clone(), natural, back))
}
