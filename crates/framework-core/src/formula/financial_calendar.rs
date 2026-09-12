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

/// The calendar a call reads: the named one, the document default, or
/// calendar months when the document sets none.
pub(super) fn resolve_calendar(
    document: &Document,
    argument: Option<&Expr>,
) -> Result<ResolvedCalendar, String> {
    match argument {
        None => Ok(default_calendar(document)),
        Some(expression) => {
            let reference = string_literal(expression, document).map_err(|_| {
                "calendar must be a calendar name written in the formula or held by a named value"
                    .to_string()
            })?;
            document
                .calendars
                .iter()
                .find(|calendar| {
                    calendar.id == reference || calendar.name.eq_ignore_ascii_case(&reference)
                })
                .ok_or_else(|| no_such_calendar(document, &reference))?
                .resolve()
        }
    }
}

fn default_calendar(document: &Document) -> ResolvedCalendar {
    document
        .default_calendar_id
        .as_deref()
        .and_then(|id| {
            document
                .calendars
                .iter()
                .find(|calendar| calendar.id == *id)
        })
        .and_then(|calendar| calendar.resolve().ok())
        .unwrap_or_else(ResolvedCalendar::builtin_months)
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

/// A whole string written in the formula or held by a named value.
fn string_literal(expression: &Expr, document: &Document) -> Result<String, String> {
    match expression {
        Expr::String { value } => Ok(value.clone()),
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
    let parameters: &[&str] = match name {
        "fiscal_week" => &["date", "calendar"],
        "workday" => &["date", "n", "calendar"],
        "networkdays" => &["start_date", "end_date", "calendar"],
        _ => unreachable!(),
    };
    if arguments.len() > parameters.len() {
        return Err(format!(
            "{name} expects at most {} arguments",
            parameters.len()
        ));
    }
    let mut slots: Vec<Option<&Expr>> = vec![None; parameters.len()];
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

fn int_series(name: &str, values: Vec<Option<i32>>) -> pl::Column {
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
                            Some(date_to_days(calendar.add_workdays(date, count)))
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

fn broadcast_dates(
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
