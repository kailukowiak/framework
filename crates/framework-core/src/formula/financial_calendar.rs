//! Calendar-aware fiscal functions: retail weeks, business days, and the
//! calendar-keyword retrofit on the month-arithmetic functions.
//!
//! A `calendar` argument names a document calendar — by id or by name —
//! either written in the formula or held by a named value, the same
//! plan-time rule as `fy_start`. Omitting it reads the document default,
//! and a document with no default reads calendar months starting in
//! January, which is exactly what these functions did before calendars
//! existed. An explicit `fy_start` still wins over the calendar's; the
//! calendar otherwise supplies both the year start and, for `fiscal_year`,
//! which year number the year is labelled with.
//!
//! Retail weeks and business days evaluate as native scalars over the
//! collected series — the same mechanism as the return solvers — because
//! 52/53-week rules and holiday skipping are scalar iteration, not
//! expression shapes. A Polars expression beats a scalar native for these
//! only when the engine offers business-day primitives, which 0.55.2 does
//! not: the `business` feature is off.
use crate::*;
use chrono::NaiveDate;
use polars::prelude as pl;
use polars::prelude::NamedFrom;

use super::financial_period::integer_literal;
use crate::model::calendar::ResolvedCalendar;

/// Calendar months starting in January, for calls that read no calendar
/// at all — not even the document default.
pub(super) fn builtin_months_calendar() -> ResolvedCalendar {
    ResolvedCalendar::builtin_months()
}

/// What the calendar `calendar_id` is called now — which is the only
/// place a rename has to reach, because the id is what formulas hold.
/// `#REF` for an id no calendar answers to, the way a lost column renders.
pub(super) fn calendar_name<'a>(document: &'a Document, calendar_id: &str) -> &'a str {
    document
        .calendars
        .iter()
        .find(|calendar| calendar.id == calendar_id)
        .map(|calendar| calendar.name.as_str())
        .unwrap_or("#REF")
}

/// The calendar a call reads: the one it names, the document default, or
/// calendar months when the document sets none.
///
/// A calendar written in the formula arrives already bound to an id — the
/// parser did that, the way it binds every other name (see
/// [`bind_references`]) — so nothing here has to guess which calendar
/// `"NRF"` meant when it was typed. A calendar handed over by a named
/// value is still a name at this point and can only be one: what that
/// value says is the user's to change between one plan and the next.
pub(super) fn resolve_calendar(
    document: &Document,
    argument: Option<&Expr>,
) -> Result<ResolvedCalendar, String> {
    match argument {
        None => default_calendar(document),
        Some(Expr::Calendar { calendar_id }) => document
            .calendars
            .iter()
            .find(|calendar| calendar.id == *calendar_id)
            .ok_or_else(|| {
                format!("The calendar this formula reads is no longer on the document (‘{calendar_id}’).")
            })?
            .resolve(),
        Some(expression) => {
            let reference = value_name(expression, document).map_err(|_| {
                "calendar must be a calendar name written in the formula or held by a named value"
                    .to_string()
            })?;
            document
                .find_calendar(&reference)
                .ok_or_else(|| no_such_calendar(document, &reference))?
                .resolve()
        }
    }
}

/// Rewrites every calendar name written in `expression` into the id of the
/// calendar it names, refusing a name no calendar answers to.
///
/// This is the calendar half of what the parser already does for a value,
/// a list and a foreign column: a name is resolved once, where it was
/// typed, and never again. Doing it here rather than at plan time is what
/// makes a rename free — the formula holds the calendar, not the spelling
/// — and what makes an unknown calendar an error where the person can see
/// the formula they wrote, instead of a broken column later.
///
/// Which argument is the calendar is not guessed: the parameter tables in
/// `financial.rs` say, and the receiver binding in
/// `financial_namespace.rs` says where the same parameter sits when the
/// call is written as a method. A calendar-taking function added there
/// needs nothing added here.
pub(crate) fn bind_references(
    expression: &mut Expr,
    document: &Document,
) -> Result<(), crate::error::CoreError> {
    match expression {
        Expr::PolarsCall {
            name,
            arguments,
            keyword_arguments,
        } => {
            let slot = calendar_slot(name, false);
            bind_call(slot, arguments, keyword_arguments, document)?;
        }
        Expr::Method {
            input,
            path,
            arguments,
            keyword_arguments,
        } => {
            // Only `.finance.<name>(…)` is a financial call written as a
            // method; a bare `.fiscal_week(…)` is an ordinary Polars
            // method name that happens to read alike, and binding a
            // calendar inside it would answer a nonexistent method with a
            // confusing complaint about its argument.
            let slot = match path.as_slice() {
                [namespace, function] if namespace.eq_ignore_ascii_case("finance") => {
                    calendar_slot(function, true)
                }
                _ => None,
            };
            bind_call(slot, arguments, keyword_arguments, document)?;
            bind_references(input, document)?;
        }
        Expr::List { items } => {
            for item in items {
                bind_references(item, document)?;
            }
        }
        Expr::Negate { expression } | Expr::Not { expression } => {
            bind_references(expression, document)?
        }
        Expr::Binary { left, right, .. } => {
            bind_references(left, document)?;
            bind_references(right, document)?;
        }
        _ => {}
    }
    Ok(())
}

/// Where the `calendar` parameter sits among the arguments as written:
/// counted from the front for a plain call, and with the receiver taken
/// out for a method, because the receiver is one of the parameters.
fn calendar_slot(function: &str, as_method: bool) -> Option<usize> {
    let lower = function.to_ascii_lowercase();
    let name = lower.strip_prefix("finance.").unwrap_or(&lower);
    if !super::financial::is_financial(name) {
        return None;
    }
    let parameters = super::financial::parameter_names(name);
    if !as_method {
        return parameters
            .iter()
            .position(|parameter| *parameter == "calendar");
    }
    let receiver = super::financial_namespace::receiver_parameter(name);
    parameters
        .iter()
        .filter(|parameter| **parameter != receiver)
        .position(|parameter| *parameter == "calendar")
}

fn bind_call(
    slot: Option<usize>,
    arguments: &mut [Expr],
    keyword_arguments: &mut [(String, Expr)],
    document: &Document,
) -> Result<(), crate::error::CoreError> {
    if let Some(index) = slot
        && let Some(argument) = arguments.get_mut(index)
    {
        bind_one(argument, document)?;
    }
    for (key, value) in keyword_arguments.iter_mut() {
        if slot.is_some() && key == "calendar" {
            bind_one(value, document)?;
        } else {
            bind_references(value, document)?;
        }
    }
    for (index, argument) in arguments.iter_mut().enumerate() {
        if Some(index) != slot {
            bind_references(argument, document)?;
        }
    }
    Ok(())
}

/// A name in the calendar slot becomes the id it names; anything else —
/// a named value, or an already-bound reference — is left alone.
fn bind_one(argument: &mut Expr, document: &Document) -> Result<(), crate::error::CoreError> {
    let Expr::String { value } = argument else {
        return bind_references(argument, document);
    };
    let calendar = document
        .find_calendar(value)
        .ok_or_else(|| crate::error::CoreError::Formula(no_such_calendar(document, value)))?;
    *argument = Expr::Calendar {
        calendar_id: calendar.id.clone(),
    };
    Ok(())
}

/// The document default, or calendar months when it sets none.
///
/// A default that will not resolve is an error, not a shrug. Swallowing it
/// — which this did — meant a document carrying a broken calendar answered
/// every bare `fiscal_year(...)` with January arithmetic while the same
/// call written `calendar="Retail"` failed loudly: two different answers
/// from one broken calendar, and the silent one wrong. The error now
/// travels, so the calendar gets fixed instead of quietly ignored.
fn default_calendar(document: &Document) -> Result<ResolvedCalendar, String> {
    let Some(id) = document.default_calendar_id.as_deref() else {
        return Ok(ResolvedCalendar::builtin_months());
    };
    let Some(calendar) = document
        .calendars
        .iter()
        .find(|calendar| calendar.id == *id)
    else {
        return Ok(ResolvedCalendar::builtin_months());
    };
    calendar.resolve().map_err(|error| {
        format!(
            "The default calendar ‘{}’ cannot be read: {error}",
            calendar.name
        )
    })
}

fn no_such_calendar(document: &Document, reference: &str) -> String {
    if document.calendars.is_empty() {
        return format!(
            "There is no calendar named ‘{reference}’. This document declares none yet."
        );
    }
    let names: Vec<&str> = document
        .calendars
        .iter()
        .map(|calendar| calendar.name.as_str())
        .collect();
    format!(
        "There is no calendar named ‘{reference}’. Calendars on this document: {}.",
        names.join(", ")
    )
}

/// The name a named value holds.
fn value_name(expression: &Expr, document: &Document) -> Result<String, String> {
    match expression {
        Expr::Value { object_id } => match document.object(object_id) {
            Ok(DataObject::Value(value)) => {
                Ok(document.effective_value_raw(value).trim().to_string())
            }
            _ => Err("expected a value".to_string()),
        },
        _ => Err("expected a string".to_string()),
    }
}

/// The year start a fiscal call counts from: written outright wins, then
/// the calendar's, then January.
pub(super) fn fiscal_start(
    explicit: Option<&Expr>,
    calendar: &ResolvedCalendar,
    document: &Document,
) -> Result<i64, String> {
    match explicit {
        Some(expression) => {
            let start = integer_literal(expression, document).map_err(|_| {
                "fy_start must be a whole month number from 1 to 12, with 1 meaning January"
                    .to_string()
            })?;
            if !(1..=12).contains(&start) {
                return Err(
                    "fy_start must be a whole month number from 1 to 12, with 1 meaning January"
                        .into(),
                );
            }
            Ok(start)
        }
        None => Ok(calendar.fy_start as i64),
    }
}

pub(super) fn compile(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    // Receiver calls arrive with everything as keywords (the receiver
    // already bound), so each parameter resolves positionally first, then
    // by name — the same shape as the fiscal binder.
    let parameters = super::financial::parameter_names(name);
    let slots = super::financial::bind_slots(name, parameters, None, arguments, keywords)?;
    let calendar = resolve_calendar(document, slots[parameters.len() - 1])?;
    let dates = slots[0]
        .ok_or_else(|| format!("{name} expects the date to read"))?
        .to_polars(document)?;
    match name {
        "fiscal_week" => Ok(weeks_of(dates, calendar)),
        "workday" => {
            let count = slots[1]
                .ok_or_else(|| format!("{name} expects a date and a day count"))?
                .to_polars(document)?;
            Ok(shift_workdays(dates, count, calendar))
        }
        "networkdays" => {
            let end = slots[1]
                .ok_or_else(|| format!("{name} expects a start and an end date"))?
                .to_polars(document)?;
            Ok(count_workdays(dates, end, calendar))
        }
        _ => unreachable!(),
    }
}

/// Polars stores dates as days since the epoch; chrono counts from the
/// first day of year 1, which is day 719163.
const EPOCH_OFFSET: i32 = 719_163;

fn days_to_date(days: i32) -> Option<NaiveDate> {
    NaiveDate::from_num_days_from_ce_opt(days + EPOCH_OFFSET)
}

fn date_to_days(date: NaiveDate) -> i32 {
    date.signed_duration_since(EPOCH)
        .num_days()
        .try_into()
        .expect("forecast horizons stay within ±5 million years")
}

/// 1970-01-01, the day Polars calls zero.
const EPOCH: NaiveDate = match NaiveDate::from_ymd_opt(1970, 1, 1) {
    Some(date) => date,
    None => panic!("1970-01-01 is a date"),
};

/// Each input column normalized to one optional value per output row: a
/// scalar broadcasts, a full column reads through, anything else is a
/// length mismatch rather than silent recycling.
fn date_column(name: &str, column: &pl::Column) -> Result<Vec<Option<NaiveDate>>, pl::PolarsError> {
    let fail = |message: &str| pl::PolarsError::ComputeError(format!("{name}: {message}").into());
    if column.dtype() != &pl::DataType::Date {
        return Err(fail("dates must be a Date column"));
    }
    let days = column.cast(&pl::DataType::Int32)?;
    let days = days.i32().map_err(|_| fail("dates must be readable"))?;
    Ok((0..days.len())
        .map(|index| days.get(index).and_then(days_to_date))
        .collect())
}

fn int_column(name: &str, column: &pl::Column) -> Result<Vec<Option<i64>>, pl::PolarsError> {
    let numbers = column.cast(&pl::DataType::Int64).map_err(|_| {
        pl::PolarsError::ComputeError(format!("{name}: counts must be whole numbers").into())
    })?;
    let numbers = numbers.i64().map_err(|_| {
        pl::PolarsError::ComputeError(format!("{name}: counts must be readable").into())
    })?;
    Ok((0..numbers.len()).map(|index| numbers.get(index)).collect())
}

pub(super) fn int_series(name: &str, values: Vec<Option<i32>>) -> pl::Column {
    pl::Column::from(pl::Series::new(name.into(), values))
}

fn date_series(name: &str, values: Vec<Option<i32>>) -> pl::PolarsResult<pl::Column> {
    Ok(pl::Column::from(
        pl::Series::new(name.into(), values).cast(&pl::DataType::Date)?,
    ))
}

/// Year, quarter, period and period bounds from the retail week table:
/// the week number places the date, the pattern places the blocks. Only
/// reached under a week pattern; calendar months keep the expression path.
pub(super) fn compile_retail(name: &str, dates: pl::Expr, calendar: ResolvedCalendar) -> pl::Expr {
    let name = name.to_string();
    let field = if name == "period_start" || name == "period_end" {
        pl::DataType::Date
    } else {
        pl::DataType::Int32
    };
    pl::apply_multiple(
        move |columns| {
            let length = columns.iter().map(|column| column.len()).max().unwrap_or(0);
            let raw = broadcast_dates(&name, &columns[0], length)?;
            if name == "period_start" || name == "period_end" {
                return date_series(
                    &name,
                    raw.into_iter()
                        .map(|date| {
                            date.map(|date| {
                                let placed = calendar.locate(date);
                                let bound = if name == "period_start" {
                                    placed.period_start
                                } else {
                                    placed.period_end
                                };
                                date_to_days(bound)
                            })
                        })
                        .collect(),
                );
            }
            Ok(int_series(
                &name,
                raw.into_iter()
                    .map(|date| {
                        date.map(|date| {
                            let placed = calendar.locate(date);
                            match name.as_str() {
                                "fiscal_year" => placed.year,
                                "fiscal_quarter" => placed.quarter as i32,
                                _ => placed.period as i32,
                            }
                        })
                    })
                    .collect(),
            ))
        },
        [dates],
        move |_, _| Ok(pl::Field::new("retail".into(), field.clone())),
        true,
    )
}

/// The 1-based week of its fiscal year for each date.
fn weeks_of(dates: pl::Expr, calendar: ResolvedCalendar) -> pl::Expr {
    let name = "fiscal_week".to_string();
    pl::apply_multiple(
        move |columns| {
            let length = columns.iter().map(|column| column.len()).max().unwrap_or(0);
            let raw = broadcast_dates(&name, &columns[0], length)?;
            Ok(int_series(
                "week",
                raw.into_iter()
                    .map(|date| date.map(|date| calendar.locate(date).week as i32))
                    .collect(),
            ))
        },
        [dates],
        |_, _| Ok(pl::Field::new("week".into(), pl::DataType::Int32)),
        true,
    )
}

/// The date `n` business days from each date, skipping the calendar's
/// weekend and holidays.
fn shift_workdays(dates: pl::Expr, count: pl::Expr, calendar: ResolvedCalendar) -> pl::Expr {
    let name = "workday".to_string();
    pl::apply_multiple(
        move |columns| {
            let length = columns.iter().map(|column| column.len()).max().unwrap_or(0);
            let dates = broadcast_dates(&name, &columns[0], length)?;
            let counts = broadcast_ints(&name, &columns[1], length)?;
            date_series(
                "workday",
                dates
                    .into_iter()
                    .zip(counts)
                    .map(|(date, count)| match (date, count) {
                        (Some(date), Some(count)) => {
                            calendar.add_workdays(date, count).map(date_to_days)
                        }
                        _ => None,
                    })
                    .collect(),
            )
        },
        [dates, count],
        |_, _| Ok(pl::Field::new("workday".into(), pl::DataType::Date)),
        true,
    )
}

/// Business days from each start through each end inclusive, negated when
/// the end precedes the start.
fn count_workdays(start: pl::Expr, end: pl::Expr, calendar: ResolvedCalendar) -> pl::Expr {
    let name = "networkdays".to_string();
    pl::apply_multiple(
        move |columns| {
            let length = columns.iter().map(|column| column.len()).max().unwrap_or(0);
            let starts = broadcast_dates(&name, &columns[0], length)?;
            let ends = broadcast_dates(&name, &columns[1], length)?;
            Ok(int_series(
                "networkdays",
                starts
                    .into_iter()
                    .zip(ends)
                    .map(|(start, end)| match (start, end) {
                        (Some(start), Some(end)) => Some(calendar.networkdays(start, end) as i32),
                        _ => None,
                    })
                    .collect(),
            ))
        },
        [start, end],
        |_, _| Ok(pl::Field::new("networkdays".into(), pl::DataType::Int32)),
        true,
    )
}

pub(super) fn broadcast_dates(
    name: &str,
    column: &pl::Column,
    length: usize,
) -> Result<Vec<Option<NaiveDate>>, pl::PolarsError> {
    let raw = date_column(name, column)?;
    broadcast(name, raw, column.len(), length)
}

fn broadcast_ints(
    name: &str,
    column: &pl::Column,
    length: usize,
) -> Result<Vec<Option<i64>>, pl::PolarsError> {
    let raw = int_column(name, column)?;
    broadcast(name, raw, column.len(), length)
}

fn broadcast<T: Clone>(
    name: &str,
    raw: Vec<Option<T>>,
    column_len: usize,
    length: usize,
) -> Result<Vec<Option<T>>, pl::PolarsError> {
    if column_len == length {
        return Ok(raw);
    }
    if column_len == 1 {
        let only = raw.into_iter().next().flatten();
        return Ok(std::iter::repeat_n(only, length).collect());
    }
    Err(pl::PolarsError::ComputeError(
        format!("{name}: columns read together must match in length").into(),
    ))
}
