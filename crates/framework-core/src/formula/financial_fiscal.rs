//! Fiscal-calendar date functions: which year, quarter and month a date
//! falls in under an offset year start, and calendar-month shifting with
//! end-of-month clamping.
//!
//! These take dates directly and need no period declaration — they are
//! row-local arithmetic, so shuffled rows and gaps cannot change an answer.
//! The declaration-owning functions (`period_index`, `prior`) live in
//! `financial_period`; the shared `fy_start` rule lives there too, so the
//! calendar on these calls can never drift from the calendar on the index.
//!
//! A `calendar` argument names a document calendar for the year start —
//! and, for `fiscal_year`, for which calendar year numbers the year. An
//! explicit `fy_start` wins over the calendar's; without either, the
//! document default calendar's year start applies, and with no default
//! calendar January starts the year. Under a retail week pattern the year,
//! quarter and period come from the week table instead of month
//! arithmetic, via the native path in `financial_calendar` — and
//! `fy_start` is refused there rather than parsed and dropped, because a
//! week-pattern year does not start on a month boundary to be moved.
use crate::*;
use polars::prelude as pl;

use super::financial_calendar::{fiscal_start, resolve_calendar};
use super::financial_period::retail_fy_start_error;
use crate::model::calendar::{WeekPattern, YearLabel};

pub(super) fn compile(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    // Receiver calls arrive with everything as keywords (the receiver
    // already bound to `date`), so each parameter resolves positionally
    // first, then by name — the same shape as `period_arguments`.
    let parameters = super::financial::parameter_names(name);
    let slots = super::financial::bind_slots(name, parameters, None, arguments, keywords)?;
    let calendar = if parameters.contains(&"calendar") {
        resolve_calendar(document, slots[parameters.len() - 1])?
    } else {
        // Pure month arithmetic reads no calendar at all — not even the
        // default — so retail declarations elsewhere cannot move it.
        super::financial_calendar::builtin_months_calendar()
    };
    let date = slots[0]
        .ok_or_else(|| format!("{name} expects the date to read"))?
        .to_polars(document)?;
    if calendar.pattern != WeekPattern::Months
        && matches!(
            name,
            "fiscal_year" | "fiscal_quarter" | "fiscal_period" | "period_start" | "period_end"
        )
    {
        // A written `fy_start` cannot be honoured here and must not be
        // silently dropped: the week table, not a month number, is what
        // says where a retail year starts. Refuse before answering.
        if parameters.get(1) == Some(&"fy_start") && slots[1].is_some() {
            return Err(retail_fy_start_error(name));
        }
        // Retail weeks are not months, so month arithmetic cannot answer
        // here; the week table does, one date at a time.
        return Ok(super::financial_calendar::compile_retail(
            name, date, calendar,
        ));
    }
    match name {
        "fiscal_year" | "fiscal_quarter" | "fiscal_period" => {
            let start = fiscal_start(slots[1], &calendar, document)?;
            Ok(match name {
                "fiscal_year" => fiscal_year_expr(date, start, calendar.year_label),
                "fiscal_quarter" => fiscal_quarter_expr(date, start),
                _ => fiscal_period_expr(date, start),
            })
        }
        "period_start" | "period_end" => Ok(if name == "period_start" {
            date.dt().month_start()
        } else {
            date.dt().month_end()
        }),
        "add_periods" => {
            let months = slots[1]
                .ok_or_else(|| format!("{name} expects a date and a month count"))?
                .to_polars(document)?
                .cast(pl::DataType::Int32);
            // The offset text is assembled row by row, so the count may be
            // a column — the same shape as date-plus-integer-days in
            // `compile_date_day_arithmetic`. A missing count makes a missing
            // date rather than a corrupt offset; a negative count shifts back.
            Ok(date
                .dt()
                .offset_by(months.cast(pl::DataType::String) + pl::lit("mo")))
        }
        _ => unreachable!(),
    }
}

// month - start runs -11 through 11, so shifting by a full year keeps the
// remainder exact: no negative dividends reach the modulo. This is the same
// fiscal-month numbering `period_index_expr` counts, exposed one date at a
// time instead of as an index.
fn fiscal_month_number(date: &pl::Expr, start: i64) -> pl::Expr {
    let month = date.clone().dt().month().cast(pl::DataType::Int32);
    let start = pl::lit(start as i32);
    ((month - start + pl::lit(12)) % pl::lit(12) + pl::lit(1)).cast(pl::DataType::Int32)
}

/// The calendar year numbering a date's fiscal year: its start year, or
/// its end year. January-start years are their own year either way, so
/// the convention only moves years that start later — by exactly one, for
/// every date, which is why the period index underneath never notices.
fn fiscal_year_expr(date: pl::Expr, start: i64, label: YearLabel) -> pl::Expr {
    let year = date.clone().dt().year().cast(pl::DataType::Int32);
    let month = date.dt().month().cast(pl::DataType::Int32);
    let starts_later = month.gt_eq(pl::lit(start as i32));
    let answer = match label {
        YearLabel::Start => pl::when(starts_later)
            .then(year.clone())
            .otherwise(year - pl::lit(1)),
        YearLabel::End => pl::when(starts_later.and(pl::lit(start > 1)))
            .then(year.clone() + pl::lit(1))
            .otherwise(year),
    };
    answer.cast(pl::DataType::Int32)
}

fn fiscal_quarter_expr(date: pl::Expr, start: i64) -> pl::Expr {
    ((fiscal_month_number(&date, start) - pl::lit(1)) / pl::lit(3) + pl::lit(1))
        .cast(pl::DataType::Int32)
}

fn fiscal_period_expr(date: pl::Expr, start: i64) -> pl::Expr {
    fiscal_month_number(&date, start)
}
