//! Fiscal-calendar date functions: which year, quarter and month a date
//! falls in under an offset year start, and calendar-month shifting with
//! end-of-month clamping.
//!
//! These take dates directly and need no period declaration — they are
//! row-local arithmetic, so shuffled rows and gaps cannot change an answer.
//! The declaration-owning functions (`period_index`, `prior`) live in
//! `financial_period`; the shared `fy_start` rule lives there too, so the
//! calendar on these calls can never drift from the calendar on the index.
//! `fy_start` names the month a fiscal year starts on, defaulting to
//! January; a fuller calendar object will supply that default later.
use crate::*;
use polars::prelude as pl;

use super::financial_period::fiscal_year_start;

pub(super) fn compile(
    name: &str,
    arguments: &[Expr],
    keywords: &[(String, Expr)],
    document: &Document,
) -> Result<pl::Expr, String> {
    // Receiver calls arrive with everything as keywords (the receiver
    // already bound to `date`), so each parameter resolves positionally
    // first, then by name — the same shape as `period_arguments`.
    let parameters: &[&str] = match name {
        "add_periods" => &["date", "n"],
        "period_start" | "period_end" => &["date"],
        _ => &["date", "fy_start"],
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
    let date = slots[0]
        .ok_or_else(|| format!("{name} expects the date to read"))?
        .to_polars(document)?;
    match name {
        "fiscal_year" | "fiscal_quarter" | "fiscal_period" => {
            let start = fiscal_year_start(slots[1], document)?;
            Ok(match name {
                "fiscal_year" => fiscal_year_expr(date, start),
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

fn fiscal_year_expr(date: pl::Expr, start: i64) -> pl::Expr {
    let year = date.clone().dt().year().cast(pl::DataType::Int32);
    let month = date.dt().month().cast(pl::DataType::Int32);
    pl::when(month.gt_eq(pl::lit(start as i32)))
        .then(year.clone())
        .otherwise(year - pl::lit(1))
        .cast(pl::DataType::Int32)
}

fn fiscal_quarter_expr(date: pl::Expr, start: i64) -> pl::Expr {
    ((fiscal_month_number(&date, start) - pl::lit(1)) / pl::lit(3) + pl::lit(1))
        .cast(pl::DataType::Int32)
}

fn fiscal_period_expr(date: pl::Expr, start: i64) -> pl::Expr {
    fiscal_month_number(&date, start)
}
